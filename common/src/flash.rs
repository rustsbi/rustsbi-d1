pub const META: u32 = 2 << 20; // 2 MiB
pub const SEE: u32 = 4 << 20; // 4 MiB
pub const DTB: u32 = 6 << 20; // 6 MiB
pub const KERNEL: u32 = 8 << 20; // 8 MiB

#[derive(Debug)]
#[repr(C)]
pub struct Meta {
    see: MetaEntry,
    kernel: MetaEntry,
    dtb: MetaEntry,
}

#[derive(Debug)]
#[repr(C)]
struct MetaEntry {
    offset: u32,
    size: u32,
}

impl MetaEntry {
    const DEFAULT: Self = Self {
        offset: !0,
        size: !0,
    };
}

macro_rules! read_payload {
    ($name:ident) => {
        #[inline]
        pub fn $name(&self) -> Option<(u32, usize)> {
            // 0 和 0xffffffff 认为是无效值
            if (0..!0).contains(&self.$name.size) {
                Some((self.$name.offset, self.$name.size as usize))
            } else {
                None
            }
        }
    };
}

impl crate::AsBinary for Meta {}

impl Meta {
    pub const DEFAULT: Self = Self {
        see: MetaEntry::DEFAULT,
        kernel: MetaEntry::DEFAULT,
        dtb: MetaEntry::DEFAULT,
    };

    read_payload!(see);
    read_payload!(kernel);
    read_payload!(dtb);

    #[inline]
    pub fn set_see(&mut self, base: u32, size: u32) {
        self.see = MetaEntry { offset: base, size };
    }

    #[inline]
    pub fn set_kernel(&mut self, base: u32, size: u32) {
        self.kernel = MetaEntry { offset: base, size };
    }

    #[inline]
    pub fn set_dtb(&mut self, base: u32, size: u32) {
        self.dtb = MetaEntry { offset: base, size };
    }
}
