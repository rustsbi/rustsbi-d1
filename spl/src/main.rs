#![no_std]
#![no_main]
#![feature(naked_functions, asm_sym, asm_const)]

mod flash;
mod logging;
mod magic;

use common::memory::*;
use core::{arch::asm, panic::PanicInfo};

#[naked]
#[no_mangle]
#[link_section = ".head.text"]
unsafe extern "C" fn head_jump() -> ! {
    asm!(
        ".option push",
        ".option rvc",
        "c.j    0x60", // 0x60: eGON.BT0 header; 0x08: FlashHead
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

#[naked]
#[no_mangle]
#[link_section = ".head.jump"]
unsafe extern "C" fn main_jump() -> ! {
    asm!("j {}", sym start, options(noreturn))
}

#[link_section = ".head.meta"]
static mut META: Meta = Meta::DEFAULT;

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
        "   csrw mie, zero",
        // 交换头 128 字节
        "   call {swap}",
        // 拷贝参数
        "   la   t0, {head}
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
        "   fence.i
            la   sp, {stack}
            li   t0, {stack_size}
            add  sp, sp, t0
            call {head}
        ",
        // 换回头 128 字节
        "   call {swap}",
        // 启动！
        "   call {main}
            fence.i
            jr   a0
        ",
        head       =   sym head_jump,
        swap       =   sym head_swap,
        param      =   sym magic::PARAM,
        param_len  = const magic::DDR3Param::LEN,

        stack      =   sym STACK,
        stack_size = const STACK_SIZE,
        main       =   sym main,
        options(noreturn)
    )
}

#[naked]
unsafe extern "C" fn head_swap() {
    asm!(
        "   la   t0, {head}
            la   t1, {magic_head}
            la   t2, {magic_tail}

        1:  bgeu t0, t2, 1f
            ld   t3, 0(t0)
            ld   t4, 0(t1)
            sd   t4, 0(t0)
            sd   t3, 0(t1)
            addi t1, t1, 8
            addi t0, t0, 8
            j    1b
        1:  ret
        ",
        head       = sym head_jump,
        magic_head = sym magic::HEAD,
        magic_tail = sym magic::TAIL,
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
    // 清空 bss
    extern "C" {
        static mut sbss: u64;
        static mut ebss: u64;
    }
    unsafe { r0::zero_bss(&mut sbss, &mut ebss) };
    let _ = Out << LOGO << Endl;
    // 如果不是从 flash 引导的，直接按照 dram 放好的位置跳
    if !unsafe { META.from_flash } {
        let see = unsafe { META.see };
        if see == !0 {
            arrow_walk()
        } else {
            return DRAM + see as usize;
        }
    }
    // 初始化 spi
    let p = Peripherals::take().unwrap();
    let clocks = Clocks {
        psi: 600_000_000.hz(),
        apb1: 24_000_000.hz(),
    };
    let gpio = Gpio::new(p.GPIO);
    let sck = gpio.portc.pc2.into_function_2();
    let scs = gpio.portc.pc3.into_function_2();
    let mosi = gpio.portc.pc4.into_function_2();
    let miso = gpio.portc.pc5.into_function_2();
    let spi = Spi::new(
        p.SPI0,
        (sck, scs, mosi, miso),
        spi::MODE_3,
        100_000_000.hz(),
        &clocks,
    );
    // 初始化 flash
    let mut flash = SpiNand::new(spi);
    let _ = Out << "NAND flash:";
    for c in flash.read_id() {
        let _ = Out << b' ' << Hex::Raw(c as _);
    }
    let _ = Out << Endl;
    // 读取 meta
    let mut meta = unsafe { common::flash::Meta::uninit() };
    flash.copy_into(common::flash::Meta::POS, meta.as_buf());
    // 如果 see 不存在，停在此阶段
    let (see_pos, see_len) = match meta.see() {
        Some(pair) => pair,
        None => arrow_walk(),
    };
    // 拷贝 dtb
    if let Some((base, len)) = meta.dtb() {
        flash.copy_into(base, unsafe { static_buf(DRAM, len) });
        let offset = dtb_offset(parse_memory_size(DRAM as _));
        unsafe { META.dtb = offset };
        let dst = (DRAM as u32 + offset) as *mut u8;
        unsafe { dst.copy_from_nonoverlapping(DRAM as *const u8, len) };
    }
    // 拷贝 see
    flash.copy_into(see_pos, unsafe { static_buf(DRAM, see_len) });
    unsafe { META.dtb = 0 };
    // 拷贝 kernel
    if let Some((base, len)) = meta.kernel() {
        flash.copy_into(base, unsafe { static_buf(KERNEL, len) });
        unsafe { META.dtb = (KERNEL - DRAM) as _ };
    }
    // 跳转
    let _ = Out << "everyting is ready, jump to main stage at " << Hex::Fmt(DRAM) << Endl;
    DRAM
}

const LOGO: &str = r"
   _  __        __          ___            __    __  ____  _ __
  / |/ /__ ___ / /  ___ _  / _ )___  ___  / /_  / / / / /_(_) /
 /    / -_)_ // _ \/ _ `/ / _  / _ \/ _ \/ __/ / /_/ / __/ / /
/_/|_/\__//__/_//_/\_,_/ /____/\___/\___/\__/  \____/\__/_/_/🦀";

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

#[cfg_attr(not(test), panic_handler)]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[inline]
unsafe fn static_buf(base: usize, size: usize) -> &'static mut [u8] {
    core::slice::from_raw_parts_mut(base as *mut u8, size)
}

fn arrow_walk() -> ! {
    use logging::Out;

    let _ = Out << "no payload ";
    let mut arrow = common::Arrow::init(52, |arr| {
        let _ = Out << unsafe { core::str::from_utf8_unchecked(arr) };
    });
    loop {
        arrow.next();
        for _ in 0..0x80_0000 {
            core::hint::spin_loop();
        }
    }
}
