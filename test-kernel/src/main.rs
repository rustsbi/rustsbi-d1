//! A test kernel to test RustSBI function on all platforms

#![no_std]
#![no_main]
#![feature(naked_functions, asm_const)]

use core::arch::asm;
use sbi_testing::sbi;

#[macro_use]
mod console;

#[cfg_attr(not(test), panic_handler)]
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
        "   csrw sie,  zero
            la    sp, {stack}
            li    t0, {stack_size}
            add   sp,  sp, t0
            call {rust_main}
        1:  wfi
            j     1b
        ",
        stack      =   sym STACK,
        stack_size = const STACK_SIZE,
        rust_main  =   sym rust_main,
        options(noreturn)
    )
}

extern "C" fn rust_main(hartid: usize, dtb_pa: usize) -> ! {
    // 清空 bss
    extern "C" {
        static mut sbss: u64;
        static mut ebss: u64;
    }
    unsafe { r0::zero_bss(&mut sbss, &mut ebss) };
    console::init();

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

    sbi::system_reset(sbi::Shutdown, sbi::NoReason);
    unreachable!()
}
