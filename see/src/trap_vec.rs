use aclint::SifiveClint as Clint;
use core::arch::asm;
use fast_trap::trap_entry;
use hal::CLINT_BASE;
use riscv::register::mtvec::{self, TrapMode::*};

/// 加载陷入向量。
#[inline]
pub(crate) fn load(vec: bool) {
    unsafe { mtvec::write(trap_vec as _, if vec { Vectored } else { Direct }) };
}

/// 中断向量表
///
/// # Safety
///
/// 裸函数。
#[naked]
pub(crate) unsafe extern "C" fn trap_vec() {
    asm!(
        ".align 2",
        ".option push",
        ".option norvc",
        "j {default}", // exception
        "j {default}", // supervisor software
        "j {default}", // reserved
        "j {msoft} ",  // machine    software
        "j {default}", // reserved
        "j {default}", // supervisor timer
        "j {default}", // reserved
        "j {mtimer}",  // machine    timer
        "j {default}", // reserved
        "j {default}", // supervisor external
        "j {default}", // reserved
        "j {default}", // machine    external
        ".option pop",
        default = sym trap_entry,
        mtimer  = sym mtimer,
        msoft   = sym msoft,
        options(noreturn)
    )
}

/// machine timer 中断代理
///
/// # Safety
///
/// 裸函数。
#[naked]
unsafe extern "C" fn mtimer() {
    asm!(
        // 换栈：
        // sp      : M sp
        // mscratch: S sp
        "   csrrw sp, mscratch, sp",
        // 保护
        "   sd    a0, -1*8(sp)
            sd    a1, -2*8(sp)
        ",
        // 清除 mtimecmp
        "   li    a0, {clint} + 0x4000
            addi  a1, zero, -1
            sw    a1, (a0)
            addi  a0, a0, 4
            sw    a1, (a0)
        ",
        // 设置 stip
        "   li    a0, {mip_stip}
            csrrs zero, mip, a0
        ",
        // 恢复
        "   ld    a0, -1*8(sp)
            ld    a1, -2*8(sp)
        ",
        // 换栈：
        // sp      : S sp
        // mscratch: M sp
        "   csrrw sp, mscratch, sp",
        // 返回
        "   mret",
        mip_stip = const 1 << 5,
        clint    = const CLINT_BASE,
        options(noreturn)
    )
}

/// machine soft 中断代理
///
/// # Safety
///
/// 裸函数。
#[naked]
unsafe extern "C" fn msoft() {
    asm!(
        // 换栈：
        // sp      : M sp
        // mscratch: S sp
        "   csrrw sp, mscratch, sp",
        // 保护
        "   addi sp, sp, -3*8
            sd   ra, 0*8(sp)
            sd   a0, 1*8(sp)
            sd   a1, 2*8(sp)
        ",
        // 清除 msip 设置 ssip
        "   li   a0, {clint}
            mv   a1, zero
            call {clear_msip}
            csrrsi zero, mip, 1 << 1
        ",
        // 恢复
        "   ld   ra, 0*8(sp)
            ld   a0, 1*8(sp)
            ld   a1, 2*8(sp)
            addi sp, sp,  3*8
        ",
        // 换栈：
        // sp      : S sp
        // mscratch: M sp
        "   csrrw sp, mscratch, sp",
        // 返回
        "   mret",
        clint        = const CLINT_BASE,
        //               Clint::clear_msip_naked(&self, hart_idx)
        clear_msip = sym Clint::clear_msip_naked,
        options(noreturn)
    )
}
