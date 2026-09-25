//! Transparency manipulation on ARGB32 pixel buffers.
//!
//! All functions operate in-place on raw Cairo ARGB32 buffers
//! (native-endian `0xAARRGGBB` pixels — see [`super::layout`]). The
//! `stride` parameter is in pixels (byte stride = stride * 4).

use super::layout::{OFFSET_A, OFFSET_B, OFFSET_G, OFFSET_R};

/// Make pixels with average brightness >= threshold transparent by
/// clearing the alpha channel. Uses BT.601-weighted brightness:
/// `(77*R + 150*G + 29*B) >> 8`.
pub fn make_transparent_by_brightness(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
    threshold: u8,
) {
    for y in 0..height {
        for x in 0..width {
            let px = (y * stride + x) * 4;
            let b = rgba[px + OFFSET_B] as u32;
            let g = rgba[px + OFFSET_G] as u32;
            let r = rgba[px + OFFSET_R] as u32;

            let brightness = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
            if brightness >= threshold {
                rgba[px + OFFSET_A] = 0;
            }
        }
    }
}

/// Make all pixels transparent except those matching the target RGB
/// color. Clears the alpha channel for non-matching pixels.
pub fn make_transparent_except_color(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
    target_r: u8,
    target_g: u8,
    target_b: u8,
) {
    for y in 0..height {
        for x in 0..width {
            let px = (y * stride + x) * 4;
            let b = rgba[px + OFFSET_B];
            let g = rgba[px + OFFSET_G];
            let r = rgba[px + OFFSET_R];

            if !(r == target_r && g == target_g && b == target_b) {
                rgba[px + OFFSET_A] = 0;
            }
        }
    }
}
