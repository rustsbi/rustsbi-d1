use crate::{fast_handler, CONTEXT, STACK_SIZE};
use core::{mem::forget, ptr::NonNull};
use fast_trap::FreeTrapStack;

/// 类型化栈。
#[repr(C, align(128))]
pub(crate) struct Stack([u8; STACK_SIZE]);

impl Stack {
    /// 零初始化以避免加载。
    pub const ZERO: Self = Self([0; STACK_SIZE]);

    #[inline]
    pub fn prepare_for_trap(&'static mut self) {
        let range = self.0.as_ptr_range();
        forget(
            FreeTrapStack::new(
                range.start as usize..range.end as usize,
                |_| {},
                unsafe { NonNull::new_unchecked(&mut CONTEXT) },
                fast_handler,
            )
            .unwrap()
            .load(),
        );
    }
}
