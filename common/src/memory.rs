pub const DRAM: usize = 0x4000_0000;
pub const KERNEL: usize = 0x4020_0000;
pub const META: usize = 0x0002_0068;

#[inline]
pub fn dtb_offset(mem_size: usize) -> usize {
    const MASK: usize = (2 << 20) - 1;
    ((mem_size.min(1 << 30) - 1) + MASK) & !MASK
}

#[repr(C)]
pub struct Meta {
    pub from_flash: bool,
    _zero: [u8; 3],
    pub see: u32,
    pub kernel: u32,
    pub dtb: u32,
}

const NONE: u32 = !0;

macro_rules! read_payload {
    ($name:ident) => {
        #[inline]
        pub const fn $name(&self) -> Option<usize> {
            match self.$name {
                NONE => None,
                offset => Some(DRAM + offset as usize),
            }
        }
    };
}

impl Meta {
    pub const DEFAULT: Self = Self {
        from_flash: false,
        _zero: [!0; 3],
        see: NONE,
        kernel: NONE,
        dtb: NONE,
    };

    #[inline]
    pub fn static_ref() -> &'static Self {
        unsafe { &*(META as *const Self) }
    }

    #[inline]
    pub const fn as_u32s(&self) -> &[u32] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u32,
                core::mem::size_of::<Self>() / 4,
            )
        }
    }

    read_payload!(see);
    read_payload!(kernel);
    read_payload!(dtb);
}
