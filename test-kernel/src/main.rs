//! A test kernel to test RustSBI function on all platforms

#![feature(naked_functions, asm_sym, asm_const)]
#![no_std]
#![no_main]

use core::arch::asm;
use sbi_testing::sbi;

#[macro_use]
mod console;

#[cfg_attr(not(test), panic_handler)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use sbi::{system_reset, RESET_REASON_SYSTEM_FAILURE, RESET_TYPE_SHUTDOWN};

    let (hard_id, pc): (usize, usize);
    unsafe { asm!("mv    {}, tp", out(reg) hard_id) };
    unsafe { asm!("auipc {},  0", out(reg) pc) };
    println!("[test-kernel-panic] hart {hard_id} {info}");
    println!("[test-kernel-panic] pc = {pc:#x}");
    println!("[test-kernel-panic] SBI test FAILED due to panic");
    system_reset(RESET_TYPE_SHUTDOWN, RESET_REASON_SYSTEM_FAILURE);
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

    println!(
        r"
 _____         _     _  __                    _
|_   _|__  ___| |_  | |/ /___ _ __ _ __   ___| |
  | |/ _ \/ __| __| | ' // _ \ '__| '_ \ / _ \ |
  | |  __/\__ \ |_  | . \  __/ |  | | | |  __/ |
  |_|\___||___/\__| |_|\_\___|_|  |_| |_|\___|_|
================================================
| boot hart id          | {hartid:20} |
| dtb physical address  | {dtb_pa:#20x} |
------------------------------------------------"
    );

    use sbi_testing::{base::NotExist, spi::SendIpi, Case, Extension as Ext};
    let _ = sbi_testing::test(hartid, 24_000_000, |case| match case {
        Case::Begin(ext) => {
            match ext {
                Ext::Base => println!("[test-kernel] Testing Base"),
                Ext::Time => println!("[test-kernel] Testing TIME"),
                Ext::Spi => println!("[test-kernel] Testing sPI"),
            }
            true
        }
        Case::End(_) => true,
        Case::Base(case) => {
            use sbi_testing::base::Case::*;
            match case {
                GetSbiSpecVersion(version) => {
                    println!("[test-kernel] sbi spec version = {version}");
                }
                GetSbiImplId(Ok(name)) => {
                    println!("[test-kernel] sbi impl = {name}");
                }
                GetSbiImplId(Err(unknown)) => {
                    println!("[test-kernel] unknown sbi impl = {unknown:#x}");
                }
                GetSbiImplVersion(version) => {
                    println!("[test-kernel] sbi impl version = {version:#x}");
                }
                ProbeExtensions(exts) => {
                    println!("[test-kernel] sbi extensions = {exts}");
                }
                GetMVendorId(id) => {
                    println!("[test-kernel] mvendor id = {id:#x}");
                }
                GetMArchId(id) => {
                    println!("[test-kernel] march id = {id:#x}");
                }
                GetMimpId(id) => {
                    println!("[test-kernel] mimp id = {id:#x}");
                }
            }
            true
        }
        Case::BaseFatel(NotExist) => panic!("sbi base not exist"),
        Case::Time(case) => {
            use sbi_testing::time::Case::*;
            match case {
                Interval { begin: _, end: _ } => {
                    println!("[test-kernel] read time register successfuly, set timer +1s");
                }
                SetTimer => {
                    println!("[test-kernel] timer interrupt delegate successfuly");
                }
            }
            true
        }
        Case::TimeFatel(fatel) => {
            use sbi_testing::time::Fatel::*;
            match fatel {
                NotExist => panic!("sbi time not exist"),
                TimeDecreased { a, b } => panic!("time decreased: {a} -> {b}"),
                UnexpectedTrap(trap) => {
                    panic!("expect trap at supervisor timer, but {trap:?} was caught");
                }
            }
        }
        Case::Spi(SendIpi) => {
            println!("[test-kernel] send ipi successfuly");
            true
        }
        Case::SpiFatel(fatel) => {
            use sbi_testing::spi::Fatel::*;
            match fatel {
                NotExist => panic!("sbi spi not exist"),
                UnexpectedTrap(trap) => {
                    panic!("expect trap at supervisor soft, but {trap:?} was caught");
                }
            }
        }
    });

    sbi::system_reset(sbi::RESET_TYPE_SHUTDOWN, sbi::RESET_REASON_NO_REASON);
    unreachable!()
}
