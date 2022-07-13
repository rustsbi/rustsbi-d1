#![no_std]

#[derive(Clone, Copy, Debug)]
pub struct PayloadEntry {
    pub target_offset: u32,
    pub size: u32,
}

impl PayloadEntry {
    pub const ZERO: Self = Self {
        target_offset: 0,
        size: 0,
    };
}

pub struct PayloadMeta(pub [PayloadEntry; 8]);

impl PayloadMeta {
    pub const ZERO: Self = Self([PayloadEntry::ZERO; 8]);

    pub const SIZE_IN_BYTES: usize = core::mem::size_of::<Self>();

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr().cast()
    }
}
