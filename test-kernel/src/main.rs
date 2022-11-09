//! A test kernel to test RustSBI function on all platforms

#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]

use core::arch::asm;
use hal::pac::UART0;
use riscv::register::*;
use sbi_testing::sbi;

#[macro_use]
extern crate rcore_console;

/// 内核入口。
///
/// # Safety
///
/// 裸函数。
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start(hartid: usize, device_tree_paddr: usize) -> ! {
    const STACK_SIZE: usize = 8192;
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

extern "C" fn rust_main(hartid: usize, dtb_pa: usize) -> ! {
    // 清空 bss
    extern "C" {
        static mut sbss: u64;
        static mut ebss: u64;
    }
    unsafe { r0::zero_bss(&mut sbss, &mut ebss) };
    rcore_console::init_console(&Console);
    rcore_console::set_log_level(option_env!("LOG"));

    let smp = 1;
    let frequency = 24_000_000;

    println!(
        r"
 _____         _     _  __                    _
|_   _|__  ___| |_  | |/ /___ _ __ _ __   ___| |
  | |/ _ \/ __| __| | ' // _ \ '__| '_ \ / _ \ |
  | |  __/\__ \ |_  | . \  __/ |  | | | |  __/ |
  |_|\___||___/\__| |_|\_\___|_|  |_| |_|\___|_|
================================================
| boot hart id          | {hartid:20} |
| smp                   | {smp:20} |
| timebase frequency    | {frequency:17} Hz |
| dtb physical address  | {dtb_pa:#20x} |
------------------------------------------------"
    );

    sbi_testing::Testing {
        hartid,
        hart_mask: (1 << smp) - 1,
        hart_mask_base: 0,
        delay: frequency,
    }
    .test();

    // 测试完整路径
    let time: usize;
    unsafe { asm!("rdtime s5", out("s5") time) };
    println!("read time to s5: {time}");

    // 测试调用延迟
    let t0 = time::read();

    for _ in 0..0xffff {
        let _ = sbi::get_marchid();
    }

    let t1 = time::read();
    log::info!("marchid duration = {}", t1 - t0);

    // 打开软中断
    unsafe {
        asm!("csrw sip, zero", options(nomem));
        sie::set_ssoft();
        sstatus::set_sie();
    };
    // 测试中断响应延迟
    let t0 = time::read();

    for _ in 0..0x20000 {
        unsafe {
            core::arch::asm!(
                "   la   {0}, 1f
                    csrw stvec, {0}
                    mv   a0, a2
                    mv   a1, zero
                    ecall
                    wfi
                .align 2
                1:  csrrci zero, sip, 1 << 1

                ",
                out(reg) _,
                in("a7") 0x735049,
                in("a6") 0,
                in("a0") 0,
                in("a1") 0,
                in("a2") 1 << hartid,
                options(nomem),
            );
        }
    }

    let t1 = time::read();
    log::info!("ipi duration = {}", t1 - t0);

    sbi::system_reset(sbi::Shutdown, sbi::NoReason);
    unreachable!()
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let (hard_id, pc): (usize, usize);
    unsafe { asm!("mv    {}, tp", out(reg) hard_id) };
    unsafe { asm!("auipc {},  0", out(reg) pc) };
    println!("[test-kernel-panic] hart {hard_id} {info}");
    println!("[test-kernel-panic] pc = {pc:#x}");
    println!("[test-kernel-panic] SBI test FAILED due to panic");
    sbi::system_reset(sbi::Shutdown, sbi::SystemFailure);
    loop {}
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
