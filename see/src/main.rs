#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]

mod execute;
mod extensions;
mod hart_csr_utils;
mod trap_vec;

#[macro_use]
extern crate rcore_console;

use common::memory;
use core::{arch::asm, ops::Range, panic::PanicInfo};
use hal::pac::UART0;

/// 特权软件信息。
struct Supervisor {
    start_addr: usize,
    opaque: usize,
}

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
    const STACK_SIZE: usize = 4096;
    #[link_section = ".bss.uninit"]
    static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

    asm!(
        "   la sp, {stack} + {stack_size}
            j  {rust_main}
        ",
        stack_size = const STACK_SIZE,
        stack      =   sym STACK,
        rust_main  =   sym rust_main,
        options(noreturn),
    )
}

extern "C" fn rust_main() {
    use common::memory::*;
    use execute::execute_supervisor;

    extern "C" {
        static mut sbss: u64;
        static mut ebss: u64;
    }
    unsafe { r0::zero_bss(&mut sbss, &mut ebss) };
    rcore_console::init_console(&Console);
    rcore_console::set_log_level(option_env!("LOG"));
    extensions::init();

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
        execute_supervisor(Supervisor {
            start_addr: kernel,
            opaque: dtb,
        })
    }
}

/// 设置 PMP。
fn set_pmp(mem: core::ops::Range<usize>, kernel: usize) {
    use riscv::register::{pmpaddr0, pmpaddr1, pmpaddr2, pmpaddr3, pmpcfg0, Permission, Range};
    unsafe {
        pmpcfg0::set_pmp(0, Range::OFF, Permission::NONE, false);
        pmpaddr0::write(0);
        // 外设
        pmpcfg0::set_pmp(1, Range::TOR, Permission::RW, false);
        pmpaddr1::write(mem.start >> 2);
        // SBI
        pmpcfg0::set_pmp(2, Range::TOR, Permission::NONE, false);
        pmpaddr2::write(kernel >> 2);
        //主存
        pmpcfg0::set_pmp(3, Range::TOR, Permission::RWX, false);
        pmpaddr3::write(mem.end >> 2);
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

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{info}");
    loop {
        core::hint::spin_loop();
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
