use hal::{
    clint::{msip, mtimecmp},
    pac::UART0,
};
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
        mtimecmp::write(stime_value);
        unsafe {
            // clear the pending timer interrupt bit as well.
            mip::clear_mtimer();
            mip::set_mtimer()
        };
    }
}

impl rustsbi::Reset for Reset {
    fn system_reset(&self, _reset_type: u32, _reset_reason: u32) -> SbiRet {
        loop {
            core::hint::spin_loop();
        }
    }
}

impl rustsbi::Ipi for Ipi {
    fn send_ipi_many(&self, hart_mask: HartMask) -> SbiRet {
        if hart_mask.has_bit(0) {
            msip::set_ipi(0);
            msip::clear_ipi(0);
        }
        SbiRet::ok(0)
    }
}
