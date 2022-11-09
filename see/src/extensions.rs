use aclint::SifiveClint as Clint;
use core::{convert::Infallible, mem::MaybeUninit};
use hal::CLINT_BASE;
use riscv::register::mip;
use rustsbi::{spec::binary::SbiRet, HartMask, RustSBI};

static mut SBI: MaybeUninit<FixedRustSBI> = MaybeUninit::uninit();

pub(crate) struct Impl;
pub(crate) type FixedRustSBI<'a> =
    RustSBI<&'a Impl, &'a Impl, Infallible, Infallible, &'a Impl, Infallible>;

pub(crate) fn init() {
    unsafe {
        SBI = MaybeUninit::new(
            rustsbi::Builder::new_machine()
                .with_timer(&Impl)
                .with_ipi(&Impl)
                .with_reset(&Impl)
                .build(),
        )
    }
}

#[inline]
pub(crate) fn sbi<'a>() -> &'static mut FixedRustSBI<'a> {
    unsafe { SBI.assume_init_mut() }
}

impl rustsbi::Timer for Impl {
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

impl rustsbi::Reset for Impl {
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

impl rustsbi::Ipi for Impl {
    fn send_ipi(&self, hart_mask: HartMask) -> SbiRet {
        if hart_mask.has_bit(0) {
            unsafe { (*(CLINT_BASE as *const Clint)).set_msip(0) };
        }
        SbiRet::success(0)
    }
}
