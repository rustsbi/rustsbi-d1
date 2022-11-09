#![no_std]

pub mod ccu;
pub mod gpio;
pub mod plic;
pub mod spi;
pub mod time;
pub use d1_pac as pac;

#[allow(clippy::transmutes_expressible_as_ptr_casts)]
pub const CLINT_BASE: usize = unsafe { core::mem::transmute(pac::CLINT::PTR) };
