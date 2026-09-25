//! Byte offsets of the four channels within a Cairo ARGB32 pixel.
//!
//! Cairo stores ARGB32 pixels as native-endian 32-bit `0xAARRGGBB`
//! values, so the memory order depends on the host: little-endian hosts
//! store `B, G, R, A`, big-endian hosts store `A, R, G, B`. Pixel
//! buffers must be indexed through these offsets to be portable.

#[cfg(target_endian = "little")]
pub const OFFSET_B: usize = 0;
#[cfg(target_endian = "little")]
pub const OFFSET_G: usize = 1;
#[cfg(target_endian = "little")]
pub const OFFSET_R: usize = 2;
#[cfg(target_endian = "little")]
pub const OFFSET_A: usize = 3;

#[cfg(target_endian = "big")]
pub const OFFSET_B: usize = 3;
#[cfg(target_endian = "big")]
pub const OFFSET_G: usize = 2;
#[cfg(target_endian = "big")]
pub const OFFSET_R: usize = 1;
#[cfg(target_endian = "big")]
pub const OFFSET_A: usize = 0;
