#[cfg(feature = "m-mode")]
pub mod mtimecmp {
    use super::clint;

    #[inline]
    pub fn write(mtime_val: u64) {
        clint()
            .mtimecmph
            .write(|w| unsafe { w.bits((mtime_val >> u32::BITS) as _) });
        clint()
            .mtimecmpl
            .write(|w| unsafe { w.bits(mtime_val as _) });
    }
}

pub mod stimecmp {
    use super::clint;

    #[inline]
    pub fn write(stime_val: u64) {
        clint()
            .stimecmph
            .write(|w| unsafe { w.bits((stime_val >> u32::BITS) as _) });
        clint()
            .stimecmpl
            .write(|w| unsafe { w.bits(stime_val as _) });
    }
}

#[cfg(feature = "m-mode")]
pub mod msip {
    use super::clint;

    #[inline]
    pub fn set() {
        clint().msip.write(|w| unsafe { w.bits(1) });
    }

    #[inline]
    pub fn clear() {
        clint().msip.reset();
    }
}

pub mod ssip {
    use super::clint;

    #[inline]
    pub fn set() {
        clint().ssip.write(|w| unsafe { w.bits(1) });
    }

    #[inline]
    pub fn clear() {
        clint().ssip.reset();
    }
}

#[inline(always)]
const fn clint() -> &'static d1_pac::clint::RegisterBlock {
    unsafe { &*d1_pac::CLINT::PTR }
}
