use crate::image::srgb;

pub fn rgba_to_grayscale(
    rgba: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    gray_out: &mut [u8],
    alpha_out: &mut [f32],
) {
    let lut = srgb::srgb_to_linear_lut();
    for y in 0..height {
        for x in 0..width {
            let px = y * stride * 4 + x * 4;
            let b = rgba[px] as f32;
            let g = rgba[px + 1] as f32;
            let r = rgba[px + 2] as f32;
            let a = rgba[px + 3] as f32 / 255.0;

            let a_safe = a.max(1e-6);
            let r_unpremult = (r / a_safe).clamp(0.0, 255.0);
            let g_unpremult = (g / a_safe).clamp(0.0, 255.0);
            let b_unpremult = (b / a_safe).clamp(0.0, 255.0);

            let r_lin = lut[r_unpremult as u8 as usize];
            let g_lin = lut[g_unpremult as u8 as usize];
            let b_lin = lut[b_unpremult as u8 as usize];

            let r_blended = 1.0 - (1.0 - r_lin) * a;
            let g_blended = 1.0 - (1.0 - g_lin) * a;
            let b_blended = 1.0 - (1.0 - b_lin) * a;

            let gray_lin =
                0.2989 * r_blended + 0.5870 * g_blended + 0.1140 * b_blended;

            let idx = y * width + x;
            let inv_lut = srgb::linear_to_srgb_lut();
            let scale = 1 << 15;
            let vi = gray_lin.clamp(0.0, 1.0);
            let li = (vi * scale as f32).round() as usize;
            gray_out[idx] = inv_lut[li.min(scale as usize)];
            alpha_out[idx] = a;
        }
    }
}

pub fn rgba_to_binary(
    rgba: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    threshold: u8,
    invert: bool,
    output: &mut [u8],
) {
    let lut = srgb::srgb_to_linear_lut();
    let inv_lut = srgb::linear_to_srgb_lut();
    let scale: usize = 1 << 15;

    for y in 0..height {
        for x in 0..width {
            let px = y * stride * 4 + x * 4;
            let b = rgba[px];
            let g = rgba[px + 1];
            let r = rgba[px + 2];
            let a = rgba[px + 3];

            let r_lin = lut[r as usize];
            let g_lin = lut[g as usize];
            let b_lin = lut[b as usize];

            let gray_lin = 0.2989 * r_lin + 0.5870 * g_lin + 0.1140 * b_lin;

            let vi = gray_lin.clamp(0.0, 1.0);
            let li = (vi * scale as f32).round() as usize;
            let gray_srgb = inv_lut[li.min(scale)];

            let idx = y * width + x;
            if a == 0 {
                output[idx] = 0;
            } else if invert {
                output[idx] = if gray_srgb > threshold { 1 } else { 0 };
            } else {
                output[idx] = if gray_srgb < threshold { 1 } else { 0 };
            }
        }
    }
}

pub fn rgba_to_grayscale_inplace(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
) {
    let lut = srgb::srgb_to_linear_lut();
    let inv_lut = srgb::linear_to_srgb_lut();
    let scale: usize = 1 << 15;

    for y in 0..height {
        for x in 0..width {
            let px = y * stride * 4 + x * 4;
            let b = rgba[px];
            let g = rgba[px + 1];
            let r = rgba[px + 2];

            let r_lin = lut[r as usize];
            let g_lin = lut[g as usize];
            let b_lin = lut[b as usize];

            let gray_lin = 0.2989 * r_lin + 0.5870 * g_lin + 0.1140 * b_lin;

            let vi = gray_lin.clamp(0.0, 1.0);
            let li = (vi * scale as f32).round() as usize;
            let gray_srgb = inv_lut[li.min(scale)];

            rgba[px] = gray_srgb;
            rgba[px + 1] = gray_srgb;
            rgba[px + 2] = gray_srgb;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_opaque_white_px() -> Vec<u8> {
        vec![255, 255, 255, 255]
    }

    fn make_opaque_black_px() -> Vec<u8> {
        vec![0, 0, 0, 255]
    }

    fn make_transparent_px() -> Vec<u8> {
        vec![0, 0, 0, 0]
    }

    #[test]
    fn test_rgba_to_grayscale_white() {
        let rgba = make_opaque_white_px();
        let mut gray = vec![0u8; 1];
        let mut alpha = vec![0.0f32; 1];
        rgba_to_grayscale(&rgba, 1, 1, 1, &mut gray, &mut alpha);
        assert_eq!(gray[0], 255);
        assert!((alpha[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rgba_to_grayscale_black() {
        let rgba = make_opaque_black_px();
        let mut gray = vec![0u8; 1];
        let mut alpha = vec![0.0f32; 1];
        rgba_to_grayscale(&rgba, 1, 1, 1, &mut gray, &mut alpha);
        assert_eq!(gray[0], 0);
    }

    #[test]
    fn test_rgba_to_grayscale_transparent() {
        let rgba = make_transparent_px();
        let mut gray = vec![0u8; 1];
        let mut alpha = vec![0.0f32; 1];
        rgba_to_grayscale(&rgba, 1, 1, 1, &mut gray, &mut alpha);
        assert_eq!(gray[0], 255);
        assert!(alpha[0] < 0.01);
    }

    #[test]
    fn test_rgba_to_binary_white() {
        let rgba = make_opaque_white_px();
        let mut out = vec![0u8; 1];
        rgba_to_binary(&rgba, 1, 1, 1, 128, false, &mut out);
        assert_eq!(out[0], 0);
    }

    #[test]
    fn test_rgba_to_binary_black() {
        let rgba = make_opaque_black_px();
        let mut out = vec![0u8; 1];
        rgba_to_binary(&rgba, 1, 1, 1, 128, false, &mut out);
        assert_eq!(out[0], 1);
    }

    #[test]
    fn test_rgba_to_binary_transparent() {
        let rgba = make_transparent_px();
        let mut out = vec![0u8; 1];
        rgba_to_binary(&rgba, 1, 1, 1, 128, false, &mut out);
        assert_eq!(out[0], 0);
    }

    #[test]
    fn test_rgba_to_binary_invert() {
        let rgba = make_opaque_white_px();
        let mut out = vec![0u8; 1];
        rgba_to_binary(&rgba, 1, 1, 1, 128, true, &mut out);
        assert_eq!(out[0], 1);
    }

    #[test]
    fn test_rgba_to_grayscale_inplace_white() {
        let mut rgba = make_opaque_white_px();
        rgba_to_grayscale_inplace(&mut rgba, 1, 1, 1);
        assert_eq!(rgba[0], 255);
        assert_eq!(rgba[1], 255);
        assert_eq!(rgba[2], 255);
    }

    #[test]
    fn test_rgba_to_grayscale_inplace_midgray() {
        let mut rgba = vec![128u8, 128, 128, 255];
        rgba_to_grayscale_inplace(&mut rgba, 1, 1, 1);
        assert_eq!(rgba[0], rgba[1]);
        assert_eq!(rgba[1], rgba[2]);
    }

    #[test]
    fn test_rgba_to_grayscale_with_stride() {
        let width = 2;
        let height = 1;
        let stride = 3;
        let mut rgba = vec![0u8; stride * 4];
        rgba[0..4].copy_from_slice(&[0, 0, 255, 255]);
        rgba[4..8].copy_from_slice(&[0, 255, 0, 255]);
        let mut gray = vec![0u8; width * height];
        let mut alpha = vec![0.0f32; width * height];
        rgba_to_grayscale(&rgba, width, height, stride, &mut gray, &mut alpha);
        assert!((gray[0] as i32 - 149).abs() <= 1);
        assert!((gray[1] as i32 - 201).abs() <= 1);
    }
}
