#[repr(C)]
pub struct Meta {
    see: MetaEntry,
    kernel: MetaEntry,
    dtb: MetaEntry,
}

#[repr(C)]
struct MetaEntry {
    offset: u32,
    size: u32,
}

macro_rules! read_payload {
    ($name:ident) => {
        #[inline]
        pub fn $name(&self) -> Option<(u32, usize)> {
            // 0 和 0xffffffff 认为是无效值
            if (0..u32::MAX).contains(&self.$name.size) {
                Some((self.$name.offset, self.$name.size as usize))
            } else {
                None
            }
        }
    };
}

impl Meta {
    pub const POS: u32 = 0x8000;

    #[inline]
    pub const unsafe fn uninit() -> Self {
        core::mem::MaybeUninit::uninit().assume_init()
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>())
        }
    }

    #[inline]
    pub fn as_buf(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, core::mem::size_of::<Self>())
        }
    }

    read_payload!(see);
    read_payload!(kernel);
    read_payload!(dtb);
}
