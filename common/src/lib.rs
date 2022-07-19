#![no_std]

mod arrow;
pub mod flash;
pub mod memory;

pub extern crate dtb_walker;
pub use arrow::Arrow;
use core::ops::Range;

#[repr(C)]
pub struct EgonHead {
    magic: [u8; 8],
    pub checksum: u32,
    pub length: u32,
    _head_size: u32,
    fel_script_address: u32,
    fel_uenv_length: u32,
    dt_name_offset: u32,
    dram_size: u32,
    boot_media: u32,
    string_pool: [u32; 13],
}

impl AsBinary for EgonHead {}

impl EgonHead {
    pub const DEFAULT: Self = Self {
        magic: *b"eGON.BT0",
        checksum: 0x5F0A6C39, // real checksum filled by blob generator
        length: 0,            // real size filled by blob generator
        _head_size: 0,
        fel_script_address: 0,
        fel_uenv_length: 0,
        dt_name_offset: 0,
        dram_size: 0,
        boot_media: 0,
        string_pool: [0; 13],
    };
}

#[derive(Clone)]
pub struct PayloadMeta {
    see: u32,
    kernel: u32,
    dtb: u32,
    pub dtb_offset: u32,
}

const VALID_SIZE: Range<u32> = 4..(1 << 30);

impl AsBinary for PayloadMeta {}

impl PayloadMeta {
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

/// 表明一个类型可以映射成字节数组。
pub trait AsBinary: Sized {
    const SIZE: usize = core::mem::size_of::<Self>();

    #[inline]
    fn as_buf(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, Self::SIZE) }
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, Self::SIZE) }
    }
}

/// 构造一个未初始化的对象。
///
/// # Safety
///
/// 生成的对象具有随机初始值。
#[inline]
pub const unsafe fn uninit<T: AsBinary>() -> T {
    #[allow(clippy::uninit_assumed_init)]
    core::mem::MaybeUninit::uninit().assume_init()
}

#[inline]
pub const fn bytes_of<T: AsBinary>(val: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts(val as *const _ as *const u8, T::SIZE) }
}
