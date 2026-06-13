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
