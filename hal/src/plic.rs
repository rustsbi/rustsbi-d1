use d1_pac::plic::ctrl::CTRL_A;

#[inline]
pub fn allow_supervisor() {
    unsafe { &*d1_pac::PLIC::ptr() }
        .ctrl
        .write(|w| w.ctrl().variant(CTRL_A::MS));
}

#[inline]
pub fn deny_supervisor() {
    unsafe { &*d1_pac::PLIC::ptr() }
        .ctrl
        .write(|w| w.ctrl().variant(CTRL_A::M));
}
