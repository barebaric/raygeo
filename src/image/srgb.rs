pub(crate) const FRAC_BITS: i32 = 15;
pub(crate) const SCALE: i32 = 1 << FRAC_BITS;
pub(crate) const INV_TABLE_SIZE: usize = (SCALE + 1) as usize;

fn build_srgb_to_linear_lut() -> [f32; 256] {
    let mut lut = [0.0f32; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let s = i as f32 / 255.0;
        *slot = if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        };
    }
    lut
}

fn build_linear_to_srgb_lut() -> [u8; INV_TABLE_SIZE] {
    let mut lut = [0u8; INV_TABLE_SIZE];
    for (i, slot) in lut.iter_mut().enumerate() {
        let lin = i as f32 / SCALE as f32;
        let s = if lin <= 0.0031308 {
            12.92 * lin
        } else {
            1.055 * lin.powf(1.0 / 2.4) - 0.055
        };
        *slot = (s * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    lut
}

static SRGB_TO_LINEAR: std::sync::OnceLock<[f32; 256]> =
    std::sync::OnceLock::new();

static LINEAR_TO_SRGB: std::sync::OnceLock<[u8; INV_TABLE_SIZE]> =
    std::sync::OnceLock::new();

pub(crate) fn srgb_to_linear_lut() -> &'static [f32; 256] {
    SRGB_TO_LINEAR.get_or_init(build_srgb_to_linear_lut)
}

pub(crate) fn linear_to_srgb_lut() -> &'static [u8; INV_TABLE_SIZE] {
    LINEAR_TO_SRGB.get_or_init(build_linear_to_srgb_lut)
}

pub fn srgb_to_linear(input: &[u8], output: &mut [f32]) {
    let lut = srgb_to_linear_lut();
    let len = input.len().min(output.len());
    for i in 0..len {
        output[i] = lut[input[i] as usize];
    }
}

pub fn linear_to_srgb(input: &[f32], output: &mut [u8]) {
    let lut = linear_to_srgb_lut();
    let len = input.len().min(output.len());
    for i in 0..len {
        let v = input[i].clamp(0.0, 1.0);
        let idx = (v * SCALE as f32).round() as usize;
        output[i] = lut[idx.min(SCALE as usize)];
    }
}

pub fn linear_to_srgb_dithered(
    input: &[f32],
    output: &mut [u8],
    noise: &[f32],
) {
    let lut = linear_to_srgb_lut();
    let len = input.len().min(output.len()).min(noise.len());
    for i in 0..len {
        let v = input[i].clamp(0.0, 1.0);
        let fixed = v * SCALE as f32 + noise[i];
        let idx = (fixed.round() as i32).clamp(0, SCALE) as usize;
        output[i] = lut[idx];
    }
}
