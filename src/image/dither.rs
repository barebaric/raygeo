use crate::image::srgb;

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
