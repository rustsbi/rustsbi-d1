#![no_std]

mod arrow;

pub use arrow::Arrow;
use core::ops::Range;

#[derive(Clone)]
pub struct PayloadMeta {
    see: u32,
    kernel: u32,
    dtb: u32,
    pub dtb_offset: u32,
}

const VALID_SIZE: Range<u32> = 4..(1 << 30);

impl PayloadMeta {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    #[inline]
    pub fn as_buf(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, Self::SIZE) }
    }

    #[inline]
    pub fn len_see(&self) -> u32 {
        if VALID_SIZE.contains(&self.see) {
            self.see
        } else {
            0
        }
    }

    #[inline]
    pub fn len_kernel(&self) -> u32 {
        if VALID_SIZE.contains(&self.kernel) {
            self.kernel
        } else {
            0
        }
    }

    #[inline]
    pub fn len_dtb(&self) -> u32 {
        if VALID_SIZE.contains(&self.dtb) {
            self.dtb
        } else {
            0
        }
    }

    #[inline]
    pub fn dtb(&self) -> Option<&[u8]> {
        let len = Some(self.len_dtb())
            .filter(|len| *len > 0)
            .map(|len| len as usize)?;
        let ptr = Some(self.dtb_offset)
            .filter(|off| *off > 0)
            .map(|off| (memory::DRAM + off as usize) as *const u8)?;
        Some(unsafe { core::slice::from_raw_parts(ptr, len) })
    }
}

pub mod memory {
    use crate::PayloadMeta;
    pub const SRAM: usize = 0x20000;
    pub const DRAM: usize = 0x4000_0000;
    pub const KERNEL: usize = 0x4020_0000;
    pub const META: usize = SRAM + 16584;

    #[inline]
    pub fn dtb_offset(mem_size: usize) -> usize {
        const MASK: usize = (2 << 20) - 1;
        ((mem_size.min(1 << 30) - 1) + MASK) & !MASK
    }

    #[inline]
    pub fn meta() -> &'static PayloadMeta {
        unsafe { &*(META as *const PayloadMeta) }
    }

    #[inline]
    pub fn meta_mut() -> &'static mut PayloadMeta {
        unsafe { &mut *(META as *mut PayloadMeta) }
    }
}

pub mod flash {
    #[derive(Clone, Copy)]
    pub struct Pos(u32);

    impl Pos {
        pub const META: Self = Self(0x8000);

        #[inline]
        pub const fn value(&self) -> usize {
            self.0 as _
        }

        #[inline]
        pub fn next(&mut self, size: u32) -> u32 {
            const MASK: u32 = (1 << 12) - 1;
            let ans = self.0;
            self.0 = (self.0 + size + MASK) & !MASK;
            ans
        }
    }
}
