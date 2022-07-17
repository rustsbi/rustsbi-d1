pub const SRAM: usize = 0x0002_0000;
pub const DRAM: usize = 0x4000_0000;
pub const KERNEL: usize = 0x4020_0000;
pub const META: usize = 0x0002_0068;

#[inline]
pub fn dtb_offset(mem_size: usize) -> u32 {
    const PAGE: u32 = 2 << 20;
    ((mem_size as u32).min(1 << 30) - PAGE) & !(PAGE - 1)
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

    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>())
        }
    }

    read_payload!(see);
    read_payload!(kernel);
    read_payload!(dtb);

    #[inline]
    pub fn set_see(&mut self, val: u32) {
        self.see = val;
    }

    #[inline]
    pub fn set_kernel(&mut self, val: u32) {
        self.kernel = val;
    }

    #[inline]
    pub fn set_dtb(&mut self, val: u32) {
        self.dtb = val;
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn parse_memory_size(ptr: *const u8) -> usize {
    use dtb_walker::{Dtb, DtbObj, HeaderError::*, Property, WalkOperation::*};

    let mut ans = 0usize;
    unsafe { Dtb::from_raw_parts_filtered(ptr, |e| matches!(e, LastCompVersion(16))) }
        .unwrap()
        .walk(|path, obj| match obj {
            DtbObj::SubNode { name } if path.is_root() && name.starts_with("memory") => StepInto,
            DtbObj::Property(Property::Reg(mut reg)) if path.last().starts_with("memory") => {
                ans = reg.next().unwrap().len();
                Terminate
            }
            _ => StepOver,
        });
    ans
}
