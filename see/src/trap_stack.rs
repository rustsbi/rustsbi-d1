use crate::{fast_handler, CONTEXT, STACK_SIZE};
use core::{mem::forget, ptr::NonNull};
use fast_trap::{FreeTrapStack, TrapStackBlock};

/// 类型化栈。
#[repr(C, align(128))]
pub(crate) struct Stack([u8; STACK_SIZE]);

impl Stack {
    /// 零初始化以避免加载。
    pub const ZERO: Self = Self([0; STACK_SIZE]);

    #[inline]
    pub fn prepare_for_trap(&'static mut self) {
        forget(
            FreeTrapStack::new(
                StackRef(self),
                unsafe { NonNull::new_unchecked(&mut CONTEXT) },
                fast_handler,
            )
            .unwrap()
            .load(),
        );
    }
}

#[repr(transparent)]
struct StackRef(&'static mut Stack);

impl AsRef<[u8]> for StackRef {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0 .0
    }
}

impl AsMut<[u8]> for StackRef {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0 .0
    }
}

impl TrapStackBlock for StackRef {}

impl Drop for StackRef {
    fn drop(&mut self) {
        panic!("Root stack cannot be dropped")
    }
}
