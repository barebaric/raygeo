use std::sync::OnceLock;

use crate::image::srgb;

pub struct DiffusionKernel {
    pub divisor: f32,
    pub taps: &'static [(i32, i32, i32)],
}

macro_rules! diffusion_kernel {
    ($name:ident, $divisor:expr, [$($dx:expr, $dy:expr, $weight:expr),+ $(,)?]) => {
        static $name: DiffusionKernel = DiffusionKernel {
            divisor: $divisor as f32,
            taps: &[$(($dx, $dy, $weight)),+],
        };
    };
}

diffusion_kernel!(FLOYD_STEINBERG, 16, [1, 0, 7, -1, 1, 3, 0, 1, 5, 1, 1, 1,]);
diffusion_kernel!(
    ATKINSON,
    8,
    [1, 0, 1, 2, 0, 1, -1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 2, 1,]
);
diffusion_kernel!(
    STUCKI,
    42,
    [
        1, 0, 8, 2, 0, 4, -2, 1, 2, -1, 1, 4, 0, 1, 8, 1, 1, 4, 2, 1, 2, -2, 2,
        1, -1, 2, 2, 0, 2, 4, 1, 2, 2, 2, 2, 1,
    ]
);
diffusion_kernel!(
    JARVIS_JUDICE_NINKE,
    48,
    [
        1, 0, 7, 2, 0, 5, -2, 1, 3, -1, 1, 5, 0, 1, 7, 1, 1, 5, 2, 1, 3, -2, 2,
        1, -1, 2, 3, 0, 2, 5, 1, 2, 3, 2, 2, 1,
    ]
);
diffusion_kernel!(
    SIERRA,
    32,
    [
        1, 0, 5, 2, 0, 3, -2, 1, 2, -1, 1, 4, 0, 1, 5, 1, 1, 4, 2, 1, 2, -1, 2,
        2, 0, 2, 3, 1, 2, 2,
    ]
);
diffusion_kernel!(
    SIERRA_2ROW,
    16,
    [1, 0, 4, 2, 0, 3, -2, 1, 1, -1, 1, 2, 0, 1, 3, 1, 1, 2, 2, 1, 1,]
);
diffusion_kernel!(SIERRA_LITE, 4, [1, 0, 2, -1, 1, 1, 0, 1, 1]);
diffusion_kernel!(
    BURKES,
    32,
    [1, 0, 8, 2, 0, 4, -2, 1, 2, -1, 1, 4, 0, 1, 8, 1, 1, 4, 2, 1, 2,]
);

pub const DIFFUSION_KERNEL_NAMES: &[&str] = &[
    "floyd_steinberg",
    "atkinson",
    "stucki",
    "jarvis_judice_ninke",
    "sierra",
    "sierra_2row",
    "sierra_lite",
    "burkes",
];

pub fn diffusion_kernel_by_name(
    name: &str,
) -> Option<&'static DiffusionKernel> {
    match name {
        "floyd_steinberg" => Some(&FLOYD_STEINBERG),
        "atkinson" => Some(&ATKINSON),
        "stucki" => Some(&STUCKI),
        "jarvis_judice_ninke" => Some(&JARVIS_JUDICE_NINKE),
        "sierra" => Some(&SIERRA),
        "sierra_2row" => Some(&SIERRA_2ROW),
        "sierra_lite" => Some(&SIERRA_LITE),
        "burkes" => Some(&BURKES),
        _ => None,
    }
}

pub fn apply_error_diffusion_dither(
    grayscale: &[u8],
    width: usize,
    height: usize,
    kernel: &DiffusionKernel,
    invert: bool,
    serpentine: bool,
    output: &mut [u8],
) {
    if height == 0 || width == 0 {
        return;
    }
    let mut forward = [0.0f32; 3];
    let mut later: Vec<(i32, i32, f32)> = Vec::new();
    for &(dx, dy, weight) in kernel.taps {
        if dy == 0 {
            forward[dx as usize] = weight as f32 / kernel.divisor;
        } else {
            later.push((dx, dy, weight as f32 / kernel.divisor));
        }
    }
    let pad = later
        .iter()
        .map(|&(dx, _dy, _w)| dx.unsigned_abs() as usize)
        .max()
        .unwrap_or(0);
    let extra_rows =
        later.iter().map(|&(_dx, dy, _w)| dy).max().unwrap_or(0) as usize;
    let stride = width + 2 * pad;
    let mut work = vec![0.0f32; (height + extra_rows) * stride];
    {
        let lut = srgb::srgb_to_linear_lut();
        for y in 0..height {
            for x in 0..width {
                work[y * stride + pad + x] =
                    lut[grayscale[y * width + x] as usize];
            }
        }
    }

    let mut values = vec![0.0f32; width + 2];
    let mut errors = vec![0.0f32; width];
    let mut dark = vec![0u8; width];
    for y in 0..height {
        let reverse = serpentine && y % 2 == 1;
        let row_start = y * stride + pad;
        if reverse {
            for x in 0..width {
                values[x] = work[row_start + width - 1 - x];
            }
        } else {
            values[..width]
                .copy_from_slice(&work[row_start..row_start + width]);
        }
        for x in 0..2 {
            values[width + x] = 0.0;
        }
        for x in 0..width {
            let old = values[x];
            let error = if old < 0.5 { old } else { old - 1.0 };
            dark[x] = if old < 0.5 { 1 } else { 0 };
            errors[x] = error;
            values[x + 1] += error * forward[1];
            values[x + 2] += error * forward[2];
        }
        if reverse {
            dark.reverse();
        }
        if invert {
            for x in 0..width {
                output[y * width + x] = 1 - dark[x];
            }
        } else {
            output[y * width..(y + 1) * width].copy_from_slice(&dark);
        }
        if reverse {
            errors.reverse();
        }
        for &(dx, dy, weight) in &later {
            let start = pad as i32 + if reverse { -dx } else { dx };
            let base = (y + dy as usize) * stride + start as usize;
            for x in 0..width {
                work[base + x] += errors[x] * weight;
            }
        }
    }
}

pub fn apply_floyd_steinberg_dither(
    grayscale: &[u8],
    width: usize,
    height: usize,
    invert: bool,
    output: &mut [u8],
) {
    let mut dithered = vec![0.0f32; width * height];
    srgb::srgb_to_linear(grayscale, &mut dithered[..width * height]);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let old_pixel = dithered[idx];
            let new_pixel = if old_pixel < 0.5 { 0.0 } else { 1.0 };
            dithered[idx] = new_pixel;
            let quant_error = old_pixel - new_pixel;

            if x + 1 < width {
                dithered[idx + 1] += quant_error * 7.0 / 16.0;
            }
            if y + 1 < height {
                if x > 0 {
                    dithered[(y + 1) * width + x - 1] +=
                        quant_error * 3.0 / 16.0;
                }
                dithered[(y + 1) * width + x] += quant_error * 5.0 / 16.0;
                if x + 1 < width {
                    dithered[(y + 1) * width + x + 1] +=
                        quant_error * 1.0 / 16.0;
                }
            }
        }
    }

    for i in 0..width * height {
        if invert {
            output[i] = if dithered[i] >= 0.5 { 1 } else { 0 };
        } else {
            output[i] = if dithered[i] < 0.5 { 1 } else { 0 };
        }
    }
}

pub fn apply_minimum_run_length(
    binary: &mut [u8],
    width: usize,
    height: usize,
    min_run_length: usize,
) {
    if min_run_length <= 1 {
        return;
    }

    for y in 0..height {
        let row_start = y * width;
        let mut x = 0;
        while x < width {
            if binary[row_start + x] == 1 {
                let run_start = x;
                while x < width && binary[row_start + x] == 1 {
                    x += 1;
                }
                if x - run_start < min_run_length {
                    for i in run_start..x {
                        binary[row_start + i] = 0;
                    }
                }
            } else {
                x += 1;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_bayer_dither(
    grayscale: &[u8],
    width: usize,
    height: usize,
    bayer_matrix: &[f32],
    matrix_size: usize,
    invert: bool,
    cell_size: usize,
    output: &mut [u8],
) {
    let matrix_entries = matrix_size * matrix_size;
    let scale = 255.0 / matrix_entries as f32;

    for y in 0..height {
        for x in 0..width {
            let cell_x = (x / cell_size) % matrix_size;
            let cell_y = (y / cell_size) % matrix_size;
            let threshold = bayer_matrix[cell_y * matrix_size + cell_x] * scale;
            let val = grayscale[y * width + x] as f32;
            if invert {
                output[y * width + x] = if val > threshold { 1 } else { 0 };
            } else {
                output[y * width + x] = if val <= threshold { 1 } else { 0 };
            }
        }
    }
}

const NEWSPRINT_MATRIX_SIZE: usize = 8;
const SPOT_CDF_SAMPLES: usize = 256;

fn spot_function(u: f64, v: f64) -> f64 {
    ((2.0 * std::f64::consts::PI * u).cos()
        + (2.0 * std::f64::consts::PI * v).cos())
        / 4.0
        + 0.5
}

/// Sorted spot values over one screen cell, used to turn a spot value
/// into the fraction of the cell that it ranks at or under.
fn spot_cdf() -> &'static Vec<f64> {
    static SPOT_CDF: OnceLock<Vec<f64>> = OnceLock::new();
    SPOT_CDF.get_or_init(|| {
        let n = SPOT_CDF_SAMPLES as f64;
        let mut values: Vec<f64> =
            Vec::with_capacity(SPOT_CDF_SAMPLES * SPOT_CDF_SAMPLES);
        for vy in 0..SPOT_CDF_SAMPLES {
            let v = (vy as f64 + 0.5) / n;
            for vx in 0..SPOT_CDF_SAMPLES {
                let u = (vx as f64 + 0.5) / n;
                values.push(spot_function(u, v));
            }
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        values
    })
}

fn coverage_for_spot(spot: f64) -> f64 {
    let cdf = spot_cdf();
    let index = cdf.partition_point(|&value| value < spot);
    (index as f64 + 0.5) / cdf.len() as f64
}

/// Clustered-dot threshold ranks with the dots on a 45 degree lattice.
/// Rank 0 marks the first pixel to darken, rank N-1 the last.
fn newsprint_matrix() -> &'static Vec<u8> {
    static NEWSPRINT_MATRIX: OnceLock<Vec<u8>> = OnceLock::new();
    NEWSPRINT_MATRIX.get_or_init(|| {
        let size = NEWSPRINT_MATRIX_SIZE as f64;
        let mut spot: Vec<(f64, usize)> =
            Vec::with_capacity(NEWSPRINT_MATRIX_SIZE * NEWSPRINT_MATRIX_SIZE);
        for vy in 0..NEWSPRINT_MATRIX_SIZE {
            let v = (vy as f64 + 0.5) / size;
            for vx in 0..NEWSPRINT_MATRIX_SIZE {
                let u = (vx as f64 + 0.5) / size;
                spot.push((
                    spot_function(u + v, u - v),
                    vy * NEWSPRINT_MATRIX_SIZE + vx,
                ));
            }
        }
        spot.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let mut ranks =
            vec![0u8; NEWSPRINT_MATRIX_SIZE * NEWSPRINT_MATRIX_SIZE];
        for (rank, &(_value, index)) in spot.iter().enumerate() {
            ranks[index] = rank as u8;
        }
        ranks
    })
}

#[allow(clippy::too_many_arguments)]
pub fn apply_halftone_dither(
    grayscale: &[u8],
    width: usize,
    height: usize,
    cell_size_mm: f64,
    angle_degrees: f64,
    pixels_per_mm_x: f64,
    pixels_per_mm_y: f64,
    invert: bool,
    output: &mut [u8],
) {
    let mut cell_mm = cell_size_mm;
    if cell_mm <= 0.0 {
        cell_mm = 1.0 / pixels_per_mm_x.max(pixels_per_mm_y);
    }
    let theta = angle_degrees.to_radians();
    let cos_t = theta.cos() / cell_mm;
    let sin_t = theta.sin() / cell_mm;
    for y in 0..height {
        let y_mm = (y as f64 + 0.5) / pixels_per_mm_y;
        for x in 0..width {
            let x_mm = (x as f64 + 0.5) / pixels_per_mm_x;
            let u = x_mm * cos_t + y_mm * sin_t;
            let v = y_mm * cos_t - x_mm * sin_t;
            let coverage = coverage_for_spot(spot_function(u, v));
            let brightness = grayscale[y * width + x] as f64 / 255.0;
            let engrave = brightness < coverage;
            output[y * width + x] = if engrave != invert { 1 } else { 0 };
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn apply_newsprint_dither(
    grayscale: &[u8],
    width: usize,
    height: usize,
    cell_size: usize,
    pixels_per_mm_x: f64,
    pixels_per_mm_y: f64,
    invert: bool,
    output: &mut [u8],
) {
    let matrix = newsprint_matrix();
    let size = NEWSPRINT_MATRIX_SIZE;
    let cell_x = cell_size.max(1);
    let cell_y = ((cell_x as f64 * pixels_per_mm_y / pixels_per_mm_x).round()
        as usize)
        .max(1);
    for y in 0..height {
        for x in 0..width {
            let col = (x / cell_x) % size;
            let row = (y / cell_y) % size;
            let rank = matrix[row * size + col] as f64;
            let entries = (size * size) as f64;
            let threshold = 255.0 * (1.0 - (rank + 0.5) / entries);
            let val = grayscale[y * width + x] as f64;
            let engrave = val < threshold;
            output[y * width + x] = if engrave != invert { 1 } else { 0 };
        }
    }
}
