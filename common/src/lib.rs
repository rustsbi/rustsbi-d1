#![no_std]

#[derive(Clone)]
pub struct PayloadMeta {
    pub see: u32,
    pub kernel: u32,
    pub dtb: u32,
    pub dtb_offset: u32,
}

impl PayloadMeta {
    #[inline]
    pub fn as_buf(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, core::mem::size_of::<Self>())
        }
    }
}

pub mod memory {
    use crate::PayloadMeta;
    pub const SRAM: usize = 0x20000;
    pub const DRAM: usize = 0x4000_0000;
    pub const KERNEL: usize = 0x4020_0000;
    const META: usize = SRAM + 16584;

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
