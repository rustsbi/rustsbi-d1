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
    pub const POS: u32 = 0x8000;
    pub const LEN: u32 = core::mem::size_of::<Self>() as _;
    pub const DEFAULT: Self = Self {
        see: MetaEntry::DEFAULT,
        kernel: MetaEntry::DEFAULT,
        dtb: MetaEntry::DEFAULT,
    };

    /// 构造一个未初始化的 flash 元数据。
    ///
    /// # Safety
    ///
    /// 生成的对象具有随机初始值。
    #[inline]
    pub const unsafe fn uninit() -> Self {
        #[allow(clippy::uninit_assumed_init)]
        core::mem::MaybeUninit::uninit().assume_init()
    }

    read_payload!(see);
    read_payload!(kernel);
    read_payload!(dtb);

    #[inline]
    pub fn set_see(&mut self, base: u32, size: u32) {
        self.see = MetaEntry {
            offset: base as u32,
            size: size as u32,
        };
    }

    #[inline]
    pub fn set_kernel(&mut self, base: u32, size: u32) {
        self.kernel = MetaEntry {
            offset: base as u32,
            size: size as u32,
        };
    }

    #[inline]
    pub fn set_dtb(&mut self, base: u32, size: u32) {
        self.dtb = MetaEntry {
            offset: base as u32,
            size: size as u32,
        };
    }
}
