//! Greedy first-fit packing of binary footprints on a binary canvas.
//!
//! This mirrors the pixel-based layout strategy used by rayforge: for
//! each item, every allowed rotation is tested against the current
//! canvas, the first collision-free position in row-major order is
//! recorded, and the variant whose placement yields the smallest
//! combined bounding box is chosen.

use rustfft::num_complex::Complex64;
use rustfft::FftPlanner;

/// A binary footprint considered by [`Packer::place`].
pub struct PackMask {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

/// A greedy first-fit packer over a binary canvas.
pub struct Packer {
    width: usize,
    height: usize,
    canvas: Vec<u8>,
    placed_bounds: Vec<(i64, i64, i64, i64)>,
    planner: FftPlanner<f64>,
}

impl Packer {
    /// Creates a packer from a row-major binary canvas.
    pub fn new(canvas: &[u8], width: usize, height: usize) -> Self {
        assert_eq!(canvas.len(), width * height, "canvas buffer size mismatch");
        Self {
            width,
            height,
            canvas: canvas.iter().map(|&v| (v != 0) as u8).collect(),
            placed_bounds: Vec::new(),
            planner: FftPlanner::new(),
        }
    }

    /// Places one item.
    ///
    /// `masks` contains one binary footprint per allowed rotation. The
    /// return value is `(variant_index, y, x)` for the best placement,
    /// or `None` when no variant fits. The chosen footprint is marked
    /// on the canvas, so later calls see it as occupied.
    pub fn place(
        &mut self,
        masks: &[PackMask],
    ) -> Option<(usize, usize, usize)> {
        if self.width == 0 || self.height == 0 {
            return None;
        }

        let fitting: Vec<usize> = masks
            .iter()
            .enumerate()
            .filter(|(_, mask)| {
                mask.width > 0
                    && mask.height > 0
                    && mask.width <= self.width
                    && mask.height <= self.height
                    && mask.data.len() == mask.width * mask.height
            })
            .map(|(index, _)| index)
            .collect();
        if fitting.is_empty() {
            return None;
        }

        let max_width = fitting
            .iter()
            .map(|&index| masks[index].width)
            .max()
            .unwrap();
        let max_height = fitting
            .iter()
            .map(|&index| masks[index].height)
            .max()
            .unwrap();
        let fft_width = (self.width + max_width - 1).next_power_of_two();
        let fft_height = (self.height + max_height - 1).next_power_of_two();

        let mut canvas_spectrum =
            vec![Complex64::new(0.0, 0.0); fft_width * fft_height];
        for y in 0..self.height {
            let src = y * self.width;
            let dst = y * fft_width;
            for x in 0..self.width {
                if self.canvas[src + x] != 0 {
                    canvas_spectrum[dst + x] = Complex64::new(1.0, 0.0);
                }
            }
        }
        fft2(
            &mut self.planner,
            &mut canvas_spectrum,
            fft_width,
            fft_height,
            false,
        );

        let mut work = vec![Complex64::new(0.0, 0.0); fft_width * fft_height];
        let mut best = None;
        let mut best_score = i64::MAX;

        for &index in &fitting {
            let mask = &masks[index];
            work.fill(Complex64::new(0.0, 0.0));
            for y in 0..mask.height {
                for x in 0..mask.width {
                    if mask.data[y * mask.width + x] == 0 {
                        continue;
                    }
                    let flipped_y = mask.height - 1 - y;
                    let flipped_x = mask.width - 1 - x;
                    work[flipped_y * fft_width + flipped_x] =
                        Complex64::new(1.0, 0.0);
                }
            }
            fft2(&mut self.planner, &mut work, fft_width, fft_height, false);
            for (value, canvas) in work.iter_mut().zip(canvas_spectrum.iter()) {
                *value *= *canvas;
            }
            fft2(&mut self.planner, &mut work, fft_width, fft_height, true);

            let valid_width = self.width - mask.width + 1;
            let valid_height = self.height - mask.height + 1;
            let mut found = None;
            for y in 0..valid_height {
                let row = (y + mask.height - 1) * fft_width + (mask.width - 1);
                for x in 0..valid_width {
                    if work[row + x].re.round_ties_even() == 0.0 {
                        found = Some((y, x));
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }

            if let Some((y, x)) = found {
                let score = self.score(x, y, mask.width, mask.height);
                if score < best_score {
                    best_score = score;
                    best = Some((index, y, x));
                }
            }
        }

        let (index, y, x) = best?;
        let mask = &masks[index];
        for mask_y in 0..mask.height {
            for mask_x in 0..mask.width {
                if mask.data[mask_y * mask.width + mask_x] != 0 {
                    self.canvas[(y + mask_y) * self.width + x + mask_x] = 1;
                }
            }
        }
        self.placed_bounds.push((
            x as i64,
            y as i64,
            (x + mask.width) as i64,
            (y + mask.height) as i64,
        ));
        Some((index, y, x))
    }

    fn score(&self, x: usize, y: usize, width: usize, height: usize) -> i64 {
        let mut min_x = x as i64;
        let mut min_y = y as i64;
        let mut max_x = (x + width) as i64;
        let mut max_y = (y + height) as i64;
        for &(x0, y0, x1, y1) in &self.placed_bounds {
            min_x = min_x.min(x0);
            min_y = min_y.min(y0);
            max_x = max_x.max(x1);
            max_y = max_y.max(y1);
        }
        (max_x - min_x) * (max_y - min_y)
    }
}

fn fft2(
    planner: &mut FftPlanner<f64>,
    data: &mut [Complex64],
    width: usize,
    height: usize,
    inverse: bool,
) {
    let row_fft = if inverse {
        planner.plan_fft_inverse(width)
    } else {
        planner.plan_fft_forward(width)
    };
    let mut row = vec![Complex64::new(0.0, 0.0); width];
    for y in 0..height {
        let start = y * width;
        row.copy_from_slice(&data[start..start + width]);
        row_fft.process(&mut row);
        data[start..start + width].copy_from_slice(&row);
    }

    let column_fft = if inverse {
        planner.plan_fft_inverse(height)
    } else {
        planner.plan_fft_forward(height)
    };
    let mut column = vec![Complex64::new(0.0, 0.0); height];
    for x in 0..width {
        for y in 0..height {
            column[y] = data[y * width + x];
        }
        column_fft.process(&mut column);
        for y in 0..height {
            data[y * width + x] = column[y];
        }
    }

    if inverse {
        let scale = 1.0 / (width as f64 * height as f64);
        for value in data.iter_mut() {
            *value *= scale;
        }
    }
}
