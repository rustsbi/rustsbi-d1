use aclint::SifiveClint as Clint;
use hal::{pac::UART0, CLINT_BASE};
use riscv::register::mip;
use rustsbi::{spec::binary::SbiRet, HartMask};

struct LegacyConsole;
struct Timer;
struct Reset;
struct Ipi;

pub fn init() {
    rustsbi::legacy_stdio::init_legacy_stdio(&LegacyConsole);
    rustsbi::init_timer(&Timer);
    rustsbi::init_reset(&Reset);
    rustsbi::init_ipi(&Ipi);
}

impl rustsbi::legacy_stdio::LegacyStdio for LegacyConsole {
    fn getchar(&self) -> u8 {
        unimplemented!()
    }

    fn putchar(&self, ch: u8) {
        let uart = unsafe { &*UART0::ptr() };
        // 等待 FIFO 空位
        while uart.usr.read().tfnf().is_full() {
            core::hint::spin_loop();
        }
        uart.thr().write(|w| w.thr().variant(ch));
    }
}

impl rustsbi::Timer for Timer {
    fn set_timer(&self, stime_value: u64) {
        unsafe {
            let clint = &*hal::pac::CLINT::PTR;
            clint.mtimecmpl.write(|w| w.bits(stime_value as _));
            clint
                .mtimecmph
                .write(|w| w.bits((stime_value >> u32::BITS) as _));
            mip::clear_stimer();
        }
    }
}

impl rustsbi::Reset for Reset {
    fn system_reset(&self, _reset_type: u32, _reset_reason: u32) -> SbiRet {
        print!("[rustsbi] system reset ");
        let mut arrow = common::Arrow::init(25, |arr| {
            print!("{}", unsafe { core::str::from_utf8_unchecked(arr) })
        });
        loop {
            arrow.next();
            for _ in 0..0x80_0000 {
                core::hint::spin_loop();
            }
        }
    }
}

impl rustsbi::Ipi for Ipi {
    fn send_ipi(&self, hart_mask: HartMask) -> SbiRet {
        if hart_mask.has_bit(0) {
            unsafe { (*(CLINT_BASE as *const Clint)).set_msip(0) };
        }
        SbiRet::success(0)
    }
}
