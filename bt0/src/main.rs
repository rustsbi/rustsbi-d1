#![no_std]
#![no_main]
#![feature(naked_functions, asm_sym, asm_const)]

mod flash;
mod logging;
mod magic;

use core::{arch::asm, panic::PanicInfo};

#[naked]
#[no_mangle]
#[link_section = ".head.text"]
unsafe extern "C" fn head_jump() -> ! {
    asm!(
        ".option push",
        ".option rvc",
        "c.j    0x68", // 0x60: eGON.BT0 header; 0x08: FlashHead
        ".option pop",
        options(noreturn)
    )
}

const STAMP_CHECKSUM: u32 = 0x5F0A6C39;

#[no_mangle]
#[link_section = ".head.egon"]
static EGON_HEAD: EgonHead = EgonHead {
    magic: *b"eGON.BT0",
    checksum: STAMP_CHECKSUM, // real checksum filled by blob generator
    length: 0,                // real size filled by blob generator
    _head_size: 0,
    fel_script_address: 0,
    fel_uenv_length: 0,
    dt_name_offset: 0,
    dram_size: 0,
    boot_media: 0,
    string_pool: [0; 13],
};

#[link_section = ".head.main"]
static MAIN_STAGE_HEAD: MainStageHead = MainStageHead {
    offset: 0,
    length: 0,
};

// **NOTICE** 必须 mut，因为会被汇编修改
#[link_section = ".bss.uninit"]
static mut MAIN_STAGE_HEAD_COPY: MainStageHead = MainStageHead {
    offset: 0,
    length: 0,
};

#[naked]
#[no_mangle]
#[link_section = ".head.jump"]
unsafe extern "C" fn main_jump() -> ! {
    asm!("j {}", sym start, options(noreturn))
}

/// Jump over head data to executable code.
///
/// # Safety
///
/// Naked function.
///
/// NOTE: `mxstatus` is a custom T-Head register. Do not confuse with `mstatus`.
/// It allows for configuring special eXtensions. See further below for details.
#[naked]
#[link_section = ".text.entry"]
unsafe extern "C" fn start() -> ! {
    const STACK_SIZE: usize = 1024;
    #[link_section = ".bss.uninit"]
    static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
    asm!(
        // 关中断
        "   csrw   mie, zero",
        // 拷贝 main 阶段信息
        "
            la   t0, {main_head}
            la   t1, {main_head_copy}

            ld   t0, 0(t0)
            sd   t0, 0(t1)
        ",
        // 拷贝魔法二进制前 256 字节到 sram 开始位置
        "
            la   t0, {head}
            la   t1, {magic_head}
            la   t2, {magic_tail}

        1:
            bgeu t0, t2, 1f
            ld   t3, 0(t1)
            sd   t3, 0(t0)
            addi t1, t1, 8
            addi t0, t0, 8
            j    1b
        1:
        ",
        // 拷贝参数
        "
            la   t0, {head}
            la   t1, {param}
            li   t2, {param_len}

            addi t0, t0, 0x18
            add  t2, t2, t1

        1:
            bgeu t1, t2, 1f
            lw   t3, 0(t1)
            sw   t3, 0(t0)
            addi t1, t1, 4
            addi t0, t0, 4
            j    1b
        1:
        ",
        // 魔法
        "
            fence.i
            la   sp, {stack}
            li   t0, {stack_size}
            add  sp, sp, t0
            call {head}
        ",
        // 拷贝下一阶段
        "   call {main}",
        // 启动！
        "
            fence.i
            jr   a0
        ",
        main_head      = sym MAIN_STAGE_HEAD,
        main_head_copy = sym MAIN_STAGE_HEAD_COPY,

        head       =   sym head_jump,
        magic_head =   sym magic::HEAD,
        magic_tail =   sym magic::TAIL,
        param      =   sym magic::PARAM,
        param_len  = const magic::DDR3Param::LEN,

        stack      =   sym STACK,
        stack_size = const STACK_SIZE,
        main       =   sym main,
        options(noreturn)
    )
}

extern "C" fn main() -> usize {
    use flash::SpiNand;
    use hal::{
        ccu::Clocks,
        gpio::Gpio,
        pac::Peripherals,
        spi::{self, Spi},
        time::U32Ext,
    };
    use logging::*;

    const RAM_BASE: usize = 0x4000_0000;

    extern "C" {
        static mut sbss: u64;
        static mut ebss: u64;
    }
    unsafe { r0::zero_bss(&mut sbss, &mut ebss) };

    let p = Peripherals::take().unwrap();
    let clocks = Clocks {
        psi: 600_000_000.hz(),
        apb1: 24_000_000.hz(),
    };
    let gpio = Gpio::new(p.GPIO);

    let spi_speed = 100_000_000.hz();

    // prepare spi interface to use in flash
    let sck = gpio.portc.pc2.into_function_2();
    let scs = gpio.portc.pc3.into_function_2();
    let mosi = gpio.portc.pc4.into_function_2();
    let miso = gpio.portc.pc5.into_function_2();
    let spi = Spi::new(
        p.SPI0,
        (sck, scs, mosi, miso),
        spi::MODE_3,
        spi_speed,
        &clocks,
    );
    let mut flash = SpiNand::new(spi);

    let _ = Out << "NAND flash:";
    for c in flash.read_id() {
        let _ = Out << b' ' << Hex::Raw(c as _);
    }
    let _ = Out << Endl;

    let main = unsafe { MAIN_STAGE_HEAD_COPY };

    let mut payload_size_buf = [0u8; 8];
    flash.copy_into(main.offset as _, &mut payload_size_buf);
    let [_, _, _, _, a, b, c, d] = payload_size_buf;
    let payload_size = u32::from_le_bytes([a, b, c, d]);

    let _ = Out
        << "oreboot 🦀"
        << Endl
        << "main stage: "
        << (main.length as usize)
        << " bytes at "
        << Hex::Fmt(main.offset as _)
        << Endl
        << "payload:    "
        << (payload_size as usize)
        << " bytes"
        << Endl;

    let total_size = if payload_size > 0 {
        2 * 1024 * 1024 + payload_size
    } else {
        main.length
    };
    let ddr_buffer = unsafe { core::slice::from_raw_parts_mut(RAM_BASE as _, total_size as _) };
    flash.copy_into(main.offset as _, ddr_buffer);

    let _ = Out << "everyting is ready, jump to main stage at " << Hex::Fmt(RAM_BASE) << Endl;

    RAM_BASE
}

#[repr(C)]
pub struct EgonHead {
    magic: [u8; 8],
    checksum: u32,
    length: u32,
    _head_size: u32,
    fel_script_address: u32,
    fel_uenv_length: u32,
    dt_name_offset: u32,
    dram_size: u32,
    boot_media: u32,
    string_pool: [u32; 13],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct MainStageHead {
    /// real offset filled by xtask
    offset: u32,
    /// real size filled by xtask
    length: u32,
}

#[cfg_attr(not(test), panic_handler)]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
