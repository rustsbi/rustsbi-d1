#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]

mod extensions;
mod hart_csr_utils;
mod riscv_spec;
mod trap_stack;
mod trap_vec;

#[macro_use]
extern crate rcore_console;

use common::memory;
use core::{arch::asm, ops::Range, panic::PanicInfo};
use fast_trap::{EntireContext, EntireResult, FastContext, FastResult, FlowContext};
use hal::pac::UART0;
use riscv_spec::*;
use rustsbi::spec::binary::SbiRet;
use trap_stack::Stack;

const STACK_SIZE: usize = 4096;

/// 栈空间。
#[link_section = ".bss.uninit"]
static mut ROOT_STACK: Stack = Stack::ZERO;

static mut SUPERVISOR: Supervisor = Supervisor {
    start_addr: 0,
    opaque: 0,
};

static mut CONTEXT: FlowContext = FlowContext::ZERO;

/// 入口。
///
/// 1. 关中断
/// 2. 设置启动栈
/// 3. 跳转到 rust 入口函数
///
/// # Safety
///
/// 裸函数。
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    asm!(
        "   la   sp, {stack} + {stack_size}
            call {move_stack}
            call {rust_main}
            j    {trap}
        ",
        stack_size = const STACK_SIZE,
        stack      =   sym ROOT_STACK,
        move_stack =   sym fast_trap::reuse_stack_for_trap,
        rust_main  =   sym rust_main,
        trap       =   sym trap_vec::trap_vec,
        options(noreturn),
    )
}

extern "C" fn rust_main() {
    use memory::*;

    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        let mut ptr = sbss as usize as *mut u8;
        let end = ebss as usize as *mut u8;
        while ptr < end {
            ptr.write(0);
            ptr = ptr.offset(1);
        }
    };
    rcore_console::init_console(&Console);
    rcore_console::set_log_level(option_env!("LOG"));

    let meta = Meta::static_ref();
    let board_info = match meta.dtb() {
        Some(dtb) => parse_board_info(dtb),
        None => {
            println!("[rustsbi] no dtb file detected");
            None
        }
    };

    let kernel = meta.kernel().unwrap_or(0);
    print!(
        "\
[rustsbi] RustSBI version {ver_sbi}, adapting to RISC-V SBI v1.0.0
{logo}
[rustsbi] Implementation     : RustSBI-D1 Version {ver_impl}
[rustsbi] Extensions         : [legacy console, timer, reset, ipi]
[rustsbi] Platform Name      : {model}
[rustsbi] Platform SMP       : 1
[rustsbi] Platform Memory    : {mem:#x?}
[rustsbi] Boot HART          : 0
[rustsbi] Device Tree Region : {dtb:#x?}
[rustsbi] Firmware Address   : {firmware:#x}
[rustsbi] Supervisor Address : {kernel:#x}
",
        model = board_info.as_ref().map_or("unknown", |i| i.model.as_str()),
        mem = board_info.as_ref().map_or(0..0, |i| i.mem.clone()),
        dtb = board_info.as_ref().map_or(0..0, |i| i.dtb.clone()),
        ver_sbi = rustsbi::VERSION,
        logo = rustsbi::LOGO,
        ver_impl = env!("CARGO_PKG_VERSION"),
        firmware = _start as usize,
    );

    if kernel == 0 {
        arrow_walk()
    } else {
        const DEFAULT: Range<usize> = memory::DRAM..memory::DRAM + (512 << 20);
        let mem = board_info.as_ref().map_or(DEFAULT, |i| i.mem.clone());
        set_pmp(mem, kernel);
        hart_csr_utils::print_pmps();

        hal::plic::allow_supervisor();

        let dtb = board_info.as_ref().map_or(0, |i| i.dtb.start);
        println!("execute_supervisor at {kernel:#x} with a1 = {dtb:#x}");

        extensions::init();
        // 准备启动调度
        unsafe {
            use riscv::register::medeleg;
            asm!("csrw mcause,  {}", in(reg) cause::BOOT);
            asm!("csrw mideleg, {}", in(reg) !0);
            asm!("csrw medeleg, {}", in(reg) !0);
            medeleg::clear_supervisor_env_call();
            medeleg::clear_illegal_instruction();
            trap_vec::load(true);
            ROOT_STACK.prepare_for_trap();
            SUPERVISOR = Supervisor {
                start_addr: kernel,
                opaque: dtb,
            };
        }
    }
}

mod cause {
    pub(crate) const BOOT: usize = 24;
}

#[inline(never)]
extern "C" fn fast_handler(
    mut ctx: FastContext,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
) -> FastResult {
    use riscv::register::{
        mcause::{self, Exception as E, Interrupt as I, Trap as T},
        mtval, time,
    };

    let cause = mcause::read();
    // 启动
    if (cause.cause() == T::Exception(E::Unknown) && cause.bits() == cause::BOOT)
        || cause.cause() == T::Interrupt(I::MachineSoft)
    {
        mstatus::update(|bits| {
            *bits &= !mstatus::MPP;
            *bits |= mstatus::MPIE | mstatus::MPP_SUPERVISOR;
        });
        mie::write(mie::MSIE | mie::MTIE);
        ctx.regs().a[0] = 0;
        ctx.regs().a[1] = unsafe { SUPERVISOR.opaque };
        ctx.regs().pc = unsafe { SUPERVISOR.start_addr };
        return ctx.call(2);
    }
    match cause.cause() {
        // SBI call
        T::Exception(E::SupervisorEnvCall) => {
            use sbi_spec::{base, legacy};
            let mut ret = extensions::sbi().handle_ecall(a7, a6, [ctx.a0(), a1, a2, a3, a4, a5]);
            if ret.is_ok() {
                if a7 == base::EID_BASE
                    && a6 == base::PROBE_EXTENSION
                    && ctx.a0() == legacy::LEGACY_CONSOLE_PUTCHAR
                {
                    ret.value = 1;
                }
            } else {
                if a7 == legacy::LEGACY_CONSOLE_PUTCHAR {
                    print!("{}", ctx.a0() as u8 as char);
                    ret = SbiRet::success(a1);
                }
            }
            ctx.regs().a = [ret.error, ret.value, a2, a3, a4, a5, a6, a7];
            mepc::next();
            ctx.restore()
        }
        // rdtime?
        T::Exception(E::IllegalInstruction) => {
            let ins = mtval::read();
            const RD_MASK: usize = ((1 << 5) - 1) << 7;
            if ins & !RD_MASK == 0xC0102073 {
                // rdtime is actually a csrrw instruction

                ctx.regs().a = [ctx.a0(), a1, a2, a3, a4, a5, a6, a7];
                let rd = (ins & RD_MASK) >> RD_MASK.trailing_zeros();
                match rd {
                    0 => {}
                    1 => ctx.regs().ra = time::read(),
                    2 => ctx.regs().sp = time::read(),
                    3 => ctx.regs().gp = time::read(),
                    4 => ctx.regs().tp = time::read(),
                    5..=7 => ctx.regs().t[rd - 5] = time::read(),
                    8..=9 => return ctx.continue_with(entire_handler, rd - 8),
                    10..=17 => ctx.regs().a[rd - 10] = time::read(),
                    18..=27 => return ctx.continue_with(entire_handler, rd - 18 + 2),
                    28..=31 => ctx.regs().t[rd - 28 + 3] = time::read(),
                    _ => panic!("invalid rd: x{rd}"),
                }
                mepc::next();
                ctx.restore()
            } else {
                println!("IllegalInstruction: {ins:#x} at mepc = {:#x}", mepc::read(),);
                panic!("stopped with unsupported trap")
            }
        }
        // 其他陷入
        trap => {
            println!(
                "
-----------------------------
> trap:    {trap:?}
> mstatus: {:#018x}
> mepc:    {:#018x}
> mtval:   {:#018x}
-----------------------------
            ",
                mstatus::read(),
                mepc::read(),
                mtval::read()
            );
            panic!("stopped with unsupported trap")
        }
    }
}

#[inline(never)]
extern "C" fn entire_handler(ctx: EntireContext<usize>) -> EntireResult {
    let (mut ctx, rd) = ctx.split();
    ctx.regs().s[rd.get()] = riscv::register::time::read();

    mepc::next();
    ctx.restore()
}

/// 设置 PMP。
fn set_pmp(mem: core::ops::Range<usize>, kernel: usize) {
    use riscv::register::*;
    unsafe {
        pmpcfg0::set_pmp(0, Range::OFF, Permission::NONE, false);
        pmpaddr0::write(0);
        // 外设
        pmpcfg0::set_pmp(1, Range::TOR, Permission::RW, false);
        pmpaddr1::write(mem.start >> 2);
        // SBI
        pmpcfg0::set_pmp(2, Range::TOR, Permission::NONE, false);
        pmpaddr2::write(kernel >> 2);
        // 主存
        pmpcfg0::set_pmp(3, Range::TOR, Permission::RWX, false);
        pmpaddr3::write(mem.end >> 2);
        // 其他
        pmpcfg0::set_pmp(4, Range::TOR, Permission::RW, false);
        pmpaddr4::write(1 << (usize::BITS - 1));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{info}");
    arrow_walk()
}

fn arrow_walk() -> ! {
    print!("[rustsbi] no kernel ");
    let mut arrow = common::Arrow::init(51, |arr| {
        print!("{}", unsafe { core::str::from_utf8_unchecked(arr) })
    });
    loop {
        arrow.next();
        for _ in 0..0x40_0000 {
            core::hint::spin_loop();
        }
    }
}

/// 特权软件信息。
struct Supervisor {
    start_addr: usize,
    opaque: usize,
}

struct Console;

impl rcore_console::Console for Console {
    #[inline]
    fn put_char(&self, c: u8) {
        let uart = unsafe { &*UART0::ptr() };
        // 等待 FIFO 空位
        while uart.usr.read().tfnf().is_full() {
            core::hint::spin_loop();
        }
        uart.thr().write(|w| w.thr().variant(c));
    }
}

/// 从设备树采集的板信息。
struct BoardInfo {
    pub dtb: Range<usize>,
    pub model: StringInline<128>,
    pub mem: Range<usize>,
}

/// 在栈上存储有限长度字符串。
struct StringInline<const N: usize>(usize, [u8; N]);

impl<const N: usize> StringInline<N> {
    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.1[..self.0]) }
    }
}

fn parse_board_info(addr: usize) -> Option<BoardInfo> {
    use common::dtb_walker::{Dtb, DtbObj, HeaderError::*, Property, WalkOperation::*};

    let dtb = unsafe {
        match Dtb::from_raw_parts_filtered(addr as _, |e| matches!(e, LastCompVersion(16))) {
            Ok(dtb) => dtb,
            Err(e) => {
                println!("Dtb not detected at {addr:#x}: {e:?}");
                return None;
            }
        }
    };
    let mut any = false;
    let mut ans = BoardInfo {
        dtb: addr..addr,
        model: StringInline(0, [0u8; 128]),
        mem: 0..0,
    };
    ans.dtb.end += dtb.total_size();
    dtb.walk(|path, obj| match obj {
        DtbObj::SubNode { name } => {
            if path.is_root() && name.starts_with("memory") {
                StepInto
            } else {
                StepOver
            }
        }
        DtbObj::Property(Property::Model(model)) if path.is_root() => {
            ans.model.0 = model.as_bytes().len();
            ans.model.1[..ans.model.0].copy_from_slice(model.as_bytes());
            if any {
                Terminate
            } else {
                any = true;
                StepOver
            }
        }
        DtbObj::Property(Property::Reg(mut reg)) if path.name().starts_with("memory") => {
            ans.mem = reg.next().unwrap();
            if any {
                Terminate
            } else {
                any = true;
                StepOut
            }
        }
        DtbObj::Property(_) => StepOver,
    });
    Some(ans)
}
