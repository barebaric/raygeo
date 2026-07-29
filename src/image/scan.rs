use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct ScanLine {
    pub index: i64,
    pub start_mm: (f64, f64),
    pub end_mm: (f64, f64),
    pub pixels: Vec<(i32, i32)>,
    pub line_interval_mm: f64,
}

impl ScanLine {
    pub fn length_mm(&self) -> f64 {
        let dx = self.end_mm.0 - self.start_mm.0;
        let dy = self.end_mm.1 - self.start_mm.1;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn direction(&self) -> (f64, f64) {
        let length = self.length_mm();
        if length < 1e-9 {
            return (1.0, 0.0);
        }
        (
            (self.end_mm.0 - self.start_mm.0) / length,
            (self.end_mm.1 - self.start_mm.1) / length,
        )
    }

    pub fn pixel_to_mm(
        &self,
        px: i32,
        py: i32,
        pixels_per_mm: (f64, f64),
    ) -> (f64, f64) {
        let (px_per_mm_x, px_per_mm_y) = pixels_per_mm;
        let px_mm = px as f64 / px_per_mm_x;
        let py_mm = py as f64 / px_per_mm_y;

        let dx = self.end_mm.0 - self.start_mm.0;
        let dy = self.end_mm.1 - self.start_mm.1;
        let length = (dx * dx + dy * dy).sqrt();
        if length < 1e-9 {
            return (self.start_mm.0, self.start_mm.1);
        }
        let inv_len = 1.0 / length;
        let dir_x = dx * inv_len;
        let dir_y = dy * inv_len;

        let cx = (self.start_mm.0 + self.end_mm.0) * 0.5;
        let cy = (self.start_mm.1 + self.end_mm.1) * 0.5;

        let t = (px_mm - cx) * dir_x + (py_mm - cy) * dir_y;
        (cx + t * dir_x, cy + t * dir_y)
    }
}

pub type BoundingBox = (i32, i32, i32, i32);

pub fn find_mask_bounding_box(
    mask: &[u8],
    height: usize,
    width: usize,
) -> Option<BoundingBox> {
    let mut y_min = i32::MAX;
    let mut y_max = i32::MIN;
    let mut x_min = i32::MAX;
    let mut x_max = i32::MIN;
    let mut found = false;

    for y in 0..height {
        for x in 0..width {
            if mask[y * width + x] != 0 {
                found = true;
                if (y as i32) < y_min {
                    y_min = y as i32;
                }
                if (y as i32) > y_max {
                    y_max = y as i32;
                }
                if (x as i32) < x_min {
                    x_min = x as i32;
                }
                if (x as i32) > x_max {
                    x_max = x as i32;
                }
            }
        }
    }

    if found {
        Some((y_min, y_max, x_min, x_max))
    } else {
        None
    }
}

pub fn generate_horizontal_scan_positions(
    y_min_px: i32,
    y_max_px: i32,
    height_px: i32,
    pixels_per_mm: (f64, f64),
    line_interval_mm: f64,
    offset_y_mm: f64,
) -> (Vec<f64>, Vec<f64>) {
    let px_per_mm_y = pixels_per_mm.1;

    let y_min_mm = y_min_px as f64 / px_per_mm_y;
    let y_max_mm = (y_max_px + 1) as f64 / px_per_mm_y;

    let global_y_min_mm = offset_y_mm + y_min_mm;
    let num_intervals = (global_y_min_mm / line_interval_mm).ceil() as i64;
    let first_scan_y_mm = num_intervals as f64 * line_interval_mm - offset_y_mm;

    let mut y_coords_mm = Vec::new();
    let mut y = first_scan_y_mm;
    while y < y_max_mm {
        y_coords_mm.push(y);
        y += line_interval_mm;
    }

    if y_coords_mm.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let y_coords_px: Vec<f64> = y_coords_mm
        .iter()
        .map(|&y_mm| (y_mm * px_per_mm_y).clamp(0.0, (height_px - 1) as f64))
        .collect();

    (y_coords_mm, y_coords_px)
}

pub fn resample_rows(
    image: &[u8],
    height: usize,
    width: usize,
    y_coords_px: &[f64],
) -> Vec<f64> {
    if y_coords_px.is_empty() {
        return Vec::new();
    }

    let mut result = vec![0.0f64; y_coords_px.len() * width];

    for (i, &y_px) in y_coords_px.iter().enumerate() {
        let y0 = y_px.floor() as usize;
        let y1 = ((y_px.ceil() as usize).clamp(0, height - 1)).min(height - 1);
        let y0 = y0.min(height - 1);
        let frac = y_px - y0 as f64;

        for x in 0..width {
            let row0_val = image[y0 * width + x] as f64;
            let row1_val = image[y1 * width + x] as f64;
            result[i * width + x] = row0_val * (1.0 - frac) + row1_val * frac;
        }
    }

    result
}

pub fn line_pixels(
    start: (f64, f64),
    end: (f64, f64),
    width: i32,
    height: i32,
) -> Vec<(i32, i32)> {
    let x0 = start.0.round() as i32;
    let y0 = start.1.round() as i32;
    let x1 = end.0.round() as i32;
    let y1 = end.1.round() as i32;

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();

    let max_pixels = dx.max(dy) + 1;
    if max_pixels <= 1 {
        if x0 >= 0 && x0 < width && y0 >= 0 && y0 < height {
            return vec![(x0, y0)];
        }
        return Vec::new();
    }

    if dy == 0 {
        let y = y0;
        if y < 0 || y >= height {
            return Vec::new();
        }
        let x_start = x0.min(x1).max(0);
        let x_end = (x0.max(x1) + 1).min(width);
        if x_start >= x_end {
            return Vec::new();
        }
        return (x_start..x_end).map(|x| (x, y)).collect();
    }

    if dx == 0 {
        let x = x0;
        if x < 0 || x >= width {
            return Vec::new();
        }
        let y_start = y0.min(y1).max(0);
        let y_end = (y0.max(y1) + 1).min(height);
        if y_start >= y_end {
            return Vec::new();
        }
        return (y_start..y_end).map(|y| (x, y)).collect();
    }

    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    let mut pixels = Vec::with_capacity(max_pixels as usize);
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < width && y >= 0 && y < height {
            pixels.push((x, y));
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }

    pixels
}

#[allow(clippy::too_many_arguments)]
pub fn generate_scan_lines(
    bbox: BoundingBox,
    image_size: (i32, i32),
    pixels_per_mm: (f64, f64),
    line_interval_mm: f64,
    direction_degrees: f64,
    offset_x_mm: f64,
    offset_y_mm: f64,
    global_center_mm: Option<(f64, f64)>,
) -> Vec<ScanLine> {
    let (y_min, y_max, x_min, x_max) = bbox;
    let (img_width, img_height) = image_size;
    let (px_per_mm_x, px_per_mm_y) = pixels_per_mm;

    let angle_rad = direction_degrees * PI / 180.0;
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    let bbox_width_mm = (x_max - x_min + 1) as f64 / px_per_mm_x;
    let bbox_height_mm = (y_max - y_min + 1) as f64 / px_per_mm_y;

    let bbox_center_x_mm = (x_min + x_max + 1) as f64 / (2.0 * px_per_mm_x);
    let bbox_center_y_mm = (y_min + y_max + 1) as f64 / (2.0 * px_per_mm_y);

    let diag_mm = (bbox_width_mm * bbox_width_mm
        + bbox_height_mm * bbox_height_mm)
        .sqrt();

    let perp_angle_rad = angle_rad + PI / 2.0;
    let perp_cos = perp_angle_rad.cos();
    let perp_sin = perp_angle_rad.sin();

    let (rotation_center_x_mm, rotation_center_y_mm) =
        if let Some(center) = global_center_mm {
            center
        } else {
            (
                bbox_center_x_mm + offset_x_mm,
                bbox_center_y_mm + offset_y_mm,
            )
        };

    let bbox_global_x_mm = bbox_center_x_mm + offset_x_mm;
    let bbox_global_y_mm = bbox_center_y_mm + offset_y_mm;

    let rotation_center_perp =
        rotation_center_x_mm * perp_cos + rotation_center_y_mm * perp_sin;

    let bbox_center_perp =
        bbox_global_x_mm * perp_cos + bbox_global_y_mm * perp_sin;

    let perp_extent_start = bbox_center_perp - diag_mm / 2.0;
    let perp_extent_end = bbox_center_perp + diag_mm / 2.0;

    let first_line_index = (perp_extent_start / line_interval_mm).ceil() as i64;
    let last_line_index = (perp_extent_end / line_interval_mm).floor() as i64;

    let mut result = Vec::new();

    for line_index in first_line_index..=last_line_index {
        let line_global_perp = line_index as f64 * line_interval_mm;
        let line_offset_from_rotation = line_global_perp - rotation_center_perp;

        let line_center_global_x_mm =
            rotation_center_x_mm + line_offset_from_rotation * perp_cos;
        let line_center_global_y_mm =
            rotation_center_y_mm + line_offset_from_rotation * perp_sin;

        let line_center_x_mm = line_center_global_x_mm - offset_x_mm;
        let line_center_y_mm = line_center_global_y_mm - offset_y_mm;

        let half_diag = diag_mm / 2.0;
        let start_x_mm = line_center_x_mm - half_diag * cos_a;
        let start_y_mm = line_center_y_mm - half_diag * sin_a;
        let end_x_mm = line_center_x_mm + half_diag * cos_a;
        let end_y_mm = line_center_y_mm + half_diag * sin_a;

        let start_x_px = start_x_mm * px_per_mm_x;
        let start_y_px = start_y_mm * px_per_mm_y;
        let end_x_px = end_x_mm * px_per_mm_x;
        let end_y_px = end_y_mm * px_per_mm_y;

        let pixels = line_pixels(
            (start_x_px, start_y_px),
            (end_x_px, end_y_px),
            img_width,
            img_height,
        );

        if pixels.is_empty() {
            continue;
        }

        result.push(ScanLine {
            index: line_index,
            start_mm: (start_x_mm, start_y_mm),
            end_mm: (end_x_mm, end_y_mm),
            pixels,
            line_interval_mm,
        });
    }

    result
}

pub fn extract_zero_power_segments(
    start: (f64, f64, f64),
    end: (f64, f64, f64),
    power_values: &[u8],
) -> Vec<f32> {
    let num_steps = power_values.len();
    if num_steps == 0 {
        return Vec::new();
    }

    let (sx, sy, sz) = start;
    let (ex, ey, ez) = end;
    let dx = ex - sx;
    let dy = ey - sy;
    let dz = ez - sz;
    let inv_n = 1.0 / num_steps as f64;

    let mut result: Vec<f32> = Vec::new();
    let mut run_start: isize = -1;

    for (i, &val) in power_values.iter().enumerate() {
        if val == 0 {
            if run_start < 0 {
                run_start = i as isize;
            }
        } else if run_start >= 0 {
            let t0 = run_start as f64 * inv_n;
            let t1 = i as f64 * inv_n;
            result.push((sx + t0 * dx) as f32);
            result.push((sy + t0 * dy) as f32);
            result.push((sz + t0 * dz) as f32);
            result.push((sx + t1 * dx) as f32);
            result.push((sy + t1 * dy) as f32);
            result.push((sz + t1 * dz) as f32);
            run_start = -1;
        }
    }

    if run_start >= 0 {
        let t0 = run_start as f64 * inv_n;
        result.push((sx + t0 * dx) as f32);
        result.push((sy + t0 * dy) as f32);
        result.push((sz + t0 * dz) as f32);
        result.push(ex as f32);
        result.push(ey as f32);
        result.push(ez as f32);
    }

    result
}

/// Extract overlay (firing) segments from a scanline's power values.
///
/// Walks the power byte array and detects on→off and off→on
/// transitions, emitting segment endpoint pairs with corresponding
/// power values and laser indices.
///
/// Returns the number of vertices added (always even: 2 per
/// segment).
#[allow(clippy::too_many_arguments)]
pub fn extract_overlay_segments(
    start: (f64, f64, f64),
    end: (f64, f64, f64),
    power_values: &[u8],
    laser_index: i32,
    out_pos: &mut Vec<f32>,
    out_pow: &mut Vec<f32>,
    out_lid: &mut Vec<f32>,
) -> usize {
    let num_steps = power_values.len();
    if num_steps == 0 {
        return 0;
    }

    let (sx, sy, sz) = start;
    let (ex, ey, ez) = end;
    let dx = (ex - sx) / num_steps as f64;
    let dy = (ey - sy) / num_steps as f64;
    let dz = (ez - sz) / num_steps as f64;

    let mut vertex_count: usize = 0;
    let mut prev_power_on = false;
    let mut seg_start_x = 0.0f64;
    let mut seg_start_y = 0.0f64;
    let mut seg_start_z = 0.0f64;
    let mut seg_power = 0.0f32;

    for (i, &power_byte) in power_values.iter().enumerate() {
        let power_on = power_byte > 0;

        if power_on && !prev_power_on {
            seg_start_x = sx + i as f64 * dx;
            seg_start_y = sy + i as f64 * dy;
            seg_start_z = sz + i as f64 * dz;
            seg_power = power_byte as f32 / 255.0;
        } else if !power_on && prev_power_on {
            let seg_end_x = sx + i as f64 * dx;
            let seg_end_y = sy + i as f64 * dy;
            let seg_end_z = sz + i as f64 * dz;
            out_pos.extend([
                seg_start_x as f32,
                seg_start_y as f32,
                seg_start_z as f32,
                seg_end_x as f32,
                seg_end_y as f32,
                seg_end_z as f32,
            ]);
            out_pow.push(seg_power);
            out_pow.push(seg_power);
            out_lid.push(laser_index as f32);
            out_lid.push(laser_index as f32);
            vertex_count += 2;
        }

        prev_power_on = power_byte > 0;
    }

    if prev_power_on {
        out_pos.extend([
            seg_start_x as f32,
            seg_start_y as f32,
            seg_start_z as f32,
            ex as f32,
            ey as f32,
            ez as f32,
        ]);
        out_pow.push(seg_power);
        out_pow.push(seg_power);
        out_lid.push(laser_index as f32);
        out_lid.push(laser_index as f32);
        vertex_count += 2;
    }

    vertex_count
}

pub fn find_segments(values: &[u8]) -> Vec<(usize, usize)> {
    if values.is_empty() {
        return Vec::new();
    }

    let n = values.len();
    let mut segments = Vec::new();
    let mut i = 0;

    while i < n {
        if values[i] != 0 {
            let start = i;
            while i < n && values[i] != 0 {
                i += 1;
            }
            segments.push((start, i));
        } else {
            i += 1;
        }
    }

    segments
}

/// Zeroes `trim_px` samples at each end of every contiguous nonzero run,
/// shortening firing time without moving toolpath geometry.
pub fn apply_dot_width_trim(values: &mut [u8], trim_px: usize) {
    if trim_px == 0 {
        return;
    }
    for (start, end) in find_segments(values) {
        let len = end - start;
        if len <= trim_px * 2 {
            for v in &mut values[start..end] {
                *v = 0;
            }
        } else {
            for v in &mut values[start..start + trim_px] {
                *v = 0;
            }
            for v in &mut values[end - trim_px..end] {
                *v = 0;
            }
        }
    }
}

pub struct DownsampledPower {
    pub power: Vec<u8>,
    pub x_mm: Vec<f64>,
    pub y_mm: Vec<f64>,
}

pub fn downsample_power_values(
    power_values: &[u8],
    start_mm: (f64, f64),
    end_mm: (f64, f64),
    sample_interval_mm: f64,
) -> DownsampledPower {
    if power_values.is_empty() {
        return DownsampledPower {
            power: Vec::new(),
            x_mm: Vec::new(),
            y_mm: Vec::new(),
        };
    }

    let dx = end_mm.0 - start_mm.0;
    let dy = end_mm.1 - start_mm.1;
    let segment_length = (dx * dx + dy * dy).sqrt();

    if segment_length < 1e-9 || power_values.len() == 1 {
        return DownsampledPower {
            power: vec![power_values[0]],
            x_mm: vec![start_mm.0],
            y_mm: vec![start_mm.1],
        };
    }

    let n = power_values.len();
    let pixel_spacing = segment_length / (n - 1) as f64;

    if sample_interval_mm <= pixel_spacing * 1.5 {
        if n <= 1 {
            return DownsampledPower {
                power: power_values.to_vec(),
                x_mm: vec![start_mm.0],
                y_mm: vec![start_mm.1],
            };
        }
        let step_x = dx / (n - 1) as f64;
        let step_y = dy / (n - 1) as f64;
        let mut x_mm = Vec::with_capacity(n);
        let mut y_mm = Vec::with_capacity(n);
        for i in 0..n {
            x_mm.push(start_mm.0 + i as f64 * step_x);
            y_mm.push(start_mm.1 + i as f64 * step_y);
        }
        return DownsampledPower {
            power: power_values.to_vec(),
            x_mm,
            y_mm,
        };
    }

    let num_samples =
        2.max((segment_length / sample_interval_mm).ceil() as usize);

    let step_x = dx / (num_samples - 1) as f64;
    let step_y = dy / (num_samples - 1) as f64;
    let mut resampled_power = Vec::with_capacity(num_samples);
    let mut resampled_x = Vec::with_capacity(num_samples);
    let mut resampled_y = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f64 / (num_samples - 1) as f64;
        let pixel_t = t * (n - 1) as f64;
        let idx = (pixel_t as usize).min(n - 1);
        resampled_power.push(power_values[idx]);
        resampled_x.push(start_mm.0 + i as f64 * step_x);
        resampled_y.push(start_mm.1 + i as f64 * step_y);
    }

    DownsampledPower {
        power: resampled_power,
        x_mm: resampled_x,
        y_mm: resampled_y,
    }
}

pub fn sample_image(
    image: &[u8],
    height: usize,
    width: usize,
    x: i32,
    y: i32,
) -> u8 {
    if x >= 0 && (x as usize) < width && y >= 0 && (y as usize) < height {
        image[y as usize * width + x as usize]
    } else {
        0
    }
}

pub fn sample_mask_along_line(
    mask: &[u8],
    height: usize,
    width: usize,
    scan_line: &ScanLine,
) -> Vec<u8> {
    let mut values = Vec::with_capacity(scan_line.pixels.len());
    for &(px, py) in &scan_line.pixels {
        values.push(sample_image(mask, height, width, px, py));
    }
    values
}

#[allow(clippy::too_many_arguments)]
pub fn compute_power_values(
    gray_image: &[u8],
    alpha: &[u8],
    height: usize,
    width: usize,
    pixels: &[(i32, i32)],
    min_power: f64,
    max_power: f64,
    step_power: f64,
    num_power_levels: usize,
) -> Vec<u8> {
    let power_range = max_power - min_power;
    let mut result = Vec::with_capacity(pixels.len());

    for &(px, py) in pixels {
        let gray = sample_image(gray_image, height, width, px, py);
        let a = sample_image(alpha, height, width, px, py);

        let mut fraction =
            min_power + (1.0 - gray as f64 / 255.0) * power_range;
        fraction *= step_power;
        let mut pv = (fraction * 255.0).round() as u8;
        if a == 0 {
            pv = 0;
        }

        if num_power_levels < 256 {
            let levels = 2.max(num_power_levels).min(256);
            let quantized = (pv as f64 * (levels - 1) as f64 / 255.0).round()
                * 255.0
                / (levels - 1) as f64;
            pv = quantized.round() as u8;
        }

        result.push(pv);
    }
    result
}
