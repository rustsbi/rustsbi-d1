#![no_std]

mod arrow;
pub mod flash;
pub mod memory;

pub extern crate dtb_walker;
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
