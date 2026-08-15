use crate::image::scan::{
    apply_dot_width_trim, compute_power_values, downsample_power_values,
    find_mask_bounding_box, find_segments, generate_scan_lines,
    sample_mask_along_line, ScanLine,
};
use crate::ops::container::Ops;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanMode {
    Segmented,
    FullSweep,
}

fn calculate_ymax_mm(image_size: (i32, i32), pixels_per_mm: (f64, f64)) -> f64 {
    image_size.1 as f64 / pixels_per_mm.1
}

fn convert_y_to_output(y_mm: f64, ymax_mm: f64) -> f64 {
    ymax_mm - y_mm
}

fn is_reversed(scan_line_index: i64) -> bool {
    (scan_line_index % 2) != 0
}

/// Samples per mm along `scan_line`'s own sampled span (not an assumed
/// grid spacing), so it stays correct at any scan angle.
fn line_samples_per_mm(scan_line: &ScanLine, pixels_per_mm: (f64, f64)) -> f64 {
    if scan_line.pixels.len() < 2 {
        return 0.0;
    }
    let (start_mm, end_mm) = line_endpoints_mm(scan_line, pixels_per_mm);
    let dx = end_mm.0 - start_mm.0;
    let dy = end_mm.1 - start_mm.1;
    let length_mm = dx.hypot(dy);
    if length_mm < 1e-9 {
        return 0.0;
    }
    (scan_line.pixels.len() - 1) as f64 / length_mm
}

fn dot_width_trim_px(
    scan_line: &ScanLine,
    pixels_per_mm: (f64, f64),
    dot_width_correction_mm: f64,
) -> usize {
    if dot_width_correction_mm <= 0.0 {
        return 0;
    }
    let samples_per_mm = line_samples_per_mm(scan_line, pixels_per_mm);
    (dot_width_correction_mm * samples_per_mm).round() as usize
}

/// Half the size of one pixel along the scan line's own direction, in
/// millimetres. Used to extend a pixel-centre endpoint out to the edge
/// of the pixel so the scan covers the full pixel area rather than
/// stopping at pixel centres (which shrinks the raster by up to one
/// pixel at each end).
fn half_pixel_mm(
    scan_line: &ScanLine,
    pixels_per_mm: (f64, f64),
) -> (f64, f64) {
    let (px_per_mm_x, px_per_mm_y) = pixels_per_mm;
    let (dir_x, dir_y) = scan_line.direction();
    let half_x = 0.5 / px_per_mm_x * dir_x.abs();
    let half_y = 0.5 / px_per_mm_y * dir_y.abs();
    (half_x, half_y)
}

fn line_endpoints_mm(
    scan_line: &ScanLine,
    pixels_per_mm: (f64, f64),
) -> ((f64, f64), (f64, f64)) {
    let Some(first) = scan_line.pixels.first() else {
        return ((0.0, 0.0), (0.0, 0.0));
    };
    let last = scan_line.pixels.last().unwrap_or(first);
    let (s, e) = (
        scan_line.pixel_to_mm(first.0, first.1, pixels_per_mm),
        scan_line.pixel_to_mm(last.0, last.1, pixels_per_mm),
    );
    let (hx, hy) = half_pixel_mm(scan_line, pixels_per_mm);
    let dir_x = if e.0 >= s.0 { 1.0 } else { -1.0 };
    let dir_y = if e.1 >= s.1 { 1.0 } else { -1.0 };
    (
        (s.0 - hx * dir_x, s.1 - hy * dir_y),
        (e.0 + hx * dir_x, e.1 + hy * dir_y),
    )
}

fn segment_endpoints_mm(
    scan_line: &ScanLine,
    start_idx: usize,
    end_idx: usize,
    rev: bool,
    pixels_per_mm: (f64, f64),
) -> ((f64, f64), (f64, f64)) {
    let (sp, ep) = if rev {
        (scan_line.pixels[end_idx - 1], scan_line.pixels[start_idx])
    } else {
        (scan_line.pixels[start_idx], scan_line.pixels[end_idx - 1])
    };
    let (s, e) = (
        scan_line.pixel_to_mm(sp.0, sp.1, pixels_per_mm),
        scan_line.pixel_to_mm(ep.0, ep.1, pixels_per_mm),
    );
    let (hx, hy) = half_pixel_mm(scan_line, pixels_per_mm);
    let dir_x = if e.0 >= s.0 { 1.0 } else { -1.0 };
    let dir_y = if e.1 >= s.1 { 1.0 } else { -1.0 };
    (
        (s.0 - hx * dir_x, s.1 - hy * dir_y),
        (e.0 + hx * dir_x, e.1 + hy * dir_y),
    )
}

fn endpoints_for_scan(
    scan_line: &ScanLine,
    rev: bool,
    pixels_per_mm: (f64, f64),
) -> ((f64, f64), (f64, f64)) {
    let (mut s, mut e) = line_endpoints_mm(scan_line, pixels_per_mm);
    if rev {
        std::mem::swap(&mut s, &mut e);
    }
    (s, e)
}

fn emit_downsampled_scan(
    ops: &mut Ops,
    power: &[u8],
    start_mm: (f64, f64),
    end_mm: (f64, f64),
    sample_interval_mm: f64,
    ymax_mm: f64,
    rev: bool,
) -> bool {
    let ds =
        downsample_power_values(power, start_mm, end_mm, sample_interval_mm);
    if ds.power.is_empty() {
        return false;
    }

    let (ds_power, ds_x_mm, ds_y_mm) = if rev {
        (
            ds.power.into_iter().rev().collect::<Vec<_>>(),
            ds.x_mm.into_iter().rev().collect::<Vec<_>>(),
            ds.y_mm.into_iter().rev().collect::<Vec<_>>(),
        )
    } else {
        (ds.power, ds.x_mm, ds.y_mm)
    };

    let sx = ds_x_mm[0];
    let sy = ds_y_mm[0];
    let ex = ds_x_mm[ds_x_mm.len() - 1];
    let ey = ds_y_mm[ds_y_mm.len() - 1];

    if (ex - sx).abs() < 1e-6 && (ey - sy).abs() < 1e-6 {
        return false;
    }

    ops.move_to(sx, convert_y_to_output(sy, ymax_mm), 0.0, None);
    ops.scan_to(ex, convert_y_to_output(ey, ymax_mm), 0.0, ds_power, None);
    true
}

fn emit_scan(
    ops: &mut Ops,
    start_mm: (f64, f64),
    end_mm: (f64, f64),
    ymax_mm: f64,
    z: f64,
    power_values: Vec<u8>,
) {
    ops.move_to(
        start_mm.0,
        convert_y_to_output(start_mm.1, ymax_mm),
        z,
        None,
    );
    ops.scan_to(
        end_mm.0,
        convert_y_to_output(end_mm.1, ymax_mm),
        z,
        power_values,
        None,
    );
}

fn emit_line(
    ops: &mut Ops,
    start_mm: (f64, f64),
    end_mm: (f64, f64),
    ymax_mm: f64,
    z: f64,
) {
    ops.move_to(
        start_mm.0,
        convert_y_to_output(start_mm.1, ymax_mm),
        z,
        None,
    );
    ops.line_to(end_mm.0, convert_y_to_output(end_mm.1, ymax_mm), z, None);
}

#[allow(clippy::too_many_arguments)]
fn process_power_full_sweep(
    ops: &mut Ops,
    scan_line: &ScanLine,
    power_values: &[u8],
    pixels_per_mm: (f64, f64),
    sample_interval_mm: f64,
    ymax_mm: f64,
    rev: bool,
    trim_px: usize,
) {
    let (start_mm, end_mm) = line_endpoints_mm(scan_line, pixels_per_mm);
    let mut power_values = power_values.to_vec();
    apply_dot_width_trim(&mut power_values, trim_px);
    emit_downsampled_scan(
        ops,
        &power_values,
        start_mm,
        end_mm,
        sample_interval_mm,
        ymax_mm,
        rev,
    );
}

#[allow(clippy::too_many_arguments)]
fn process_power_segmented(
    ops: &mut Ops,
    scan_line: &ScanLine,
    power_values: &[u8],
    pixels_per_mm: (f64, f64),
    sample_interval_mm: f64,
    ymax_mm: f64,
    rev: bool,
    trim_px: usize,
) {
    // Segments come from the untrimmed array so geometry can't shift.
    let segments = find_segments(power_values);
    if segments.is_empty() {
        return;
    }

    let iter_segments: Vec<(usize, usize)> = if rev {
        segments.into_iter().rev().collect()
    } else {
        segments
    };

    for (si, ei) in iter_segments {
        if power_values[si] == 0 {
            continue;
        }
        // Endpoints stay in image (left-to-right) order here:
        // emit_downsampled_scan's ``rev`` handling reverses both the
        // traversal direction and the power profile consistently.
        // Passing rev-aware endpoints here as well would flip the
        // direction twice and mirror the per-sample power values
        // around the segment center.
        let (seg_start, seg_end) =
            segment_endpoints_mm(scan_line, si, ei, false, pixels_per_mm);
        let mut seg_values = power_values[si..ei].to_vec();
        apply_dot_width_trim(&mut seg_values, trim_px);
        emit_downsampled_scan(
            ops,
            &seg_values,
            seg_start,
            seg_end,
            sample_interval_mm,
            ymax_mm,
            rev,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn process_scan_full_sweep(
    ops: &mut Ops,
    scan_line: &ScanLine,
    values: &[u8],
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    step_power: f64,
    rev: bool,
    trim_px: usize,
) {
    let power_byte = (255.0 * step_power).round() as u8;
    let mut power_values: Vec<u8> = values
        .iter()
        .map(|&v| if v > 0 { power_byte } else { 0 })
        .collect();
    apply_dot_width_trim(&mut power_values, trim_px);

    let (start_mm, end_mm) = endpoints_for_scan(scan_line, rev, pixels_per_mm);
    let pw = if rev {
        power_values.into_iter().rev().collect()
    } else {
        power_values
    };
    emit_scan(ops, start_mm, end_mm, ymax_mm, 0.0, pw);
}

#[allow(clippy::too_many_arguments)]
fn process_scan_segmented(
    ops: &mut Ops,
    scan_line: &ScanLine,
    values: &[u8],
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    step_power: f64,
    rev: bool,
    trim_px: usize,
) {
    // Segments come from the untrimmed mask so geometry can't shift.
    let power_byte = (255.0 * step_power).round() as u8;
    let segments = find_segments(values);
    if segments.is_empty() {
        return;
    }

    let iter_segments: Vec<(usize, usize)> = if rev {
        segments.into_iter().rev().collect()
    } else {
        segments
    };

    for (si, ei) in iter_segments {
        if values[si] == 0 {
            continue;
        }
        let (start_mm, end_mm) =
            segment_endpoints_mm(scan_line, si, ei, rev, pixels_per_mm);
        let mut pw = vec![power_byte; ei - si];
        apply_dot_width_trim(&mut pw, trim_px);
        emit_scan(ops, start_mm, end_mm, ymax_mm, 0.0, pw);
    }
}

fn process_line_full_sweep(
    ops: &mut Ops,
    scan_line: &ScanLine,
    values: &[u8],
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    z: f64,
    rev: bool,
) {
    if !values.iter().any(|&v| v > 0) {
        return;
    }

    let power_values: Vec<u8> = values
        .iter()
        .map(|&v| if v > 0 { 255 } else { 0 })
        .collect();
    let pw = if rev {
        power_values.into_iter().rev().collect()
    } else {
        power_values
    };
    let (start_mm, end_mm) = endpoints_for_scan(scan_line, rev, pixels_per_mm);
    emit_scan(ops, start_mm, end_mm, ymax_mm, z, pw);
}

fn process_line_segmented(
    ops: &mut Ops,
    scan_line: &ScanLine,
    values: &[u8],
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    z: f64,
    rev: bool,
) {
    let segments = find_segments(values);
    if segments.is_empty() {
        return;
    }

    let iter_segments: Vec<(usize, usize)> = if rev {
        segments.into_iter().rev().collect()
    } else {
        segments
    };

    for (si, ei) in iter_segments {
        if values[si] == 0 {
            continue;
        }
        let (start_mm, end_mm) =
            segment_endpoints_mm(scan_line, si, ei, rev, pixels_per_mm);
        emit_line(ops, start_mm, end_mm, ymax_mm, z);
    }
}

impl Ops {
    #[allow(clippy::too_many_arguments)]
    pub fn from_power_modulated_image(
        gray_image: &[u8],
        alpha: &[u8],
        height: usize,
        width: usize,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        sample_interval_mm: f64,
        min_power: f64,
        max_power: f64,
        step_power: f64,
        num_power_levels: usize,
        angle: f64,
        scan_mode: ScanMode,
        dot_width_correction_mm: f64,
    ) -> Ops {
        let mut ops = Ops::new();
        let ymax_mm =
            calculate_ymax_mm((width as i32, height as i32), pixels_per_mm);

        let bbox = match find_mask_bounding_box(alpha, height, width) {
            Some(b) => b,
            None => return ops,
        };

        let scan_lines = generate_scan_lines(
            bbox,
            (width as i32, height as i32),
            pixels_per_mm,
            line_interval_mm,
            angle,
            offset_x_mm,
            offset_y_mm,
            None,
        );

        for scan_line in &scan_lines {
            if scan_line.pixels.is_empty() {
                continue;
            }

            let power_values = compute_power_values(
                gray_image,
                alpha,
                height,
                width,
                &scan_line.pixels,
                min_power,
                max_power,
                step_power,
                num_power_levels,
            );

            if !power_values.iter().any(|&v| v > 0) {
                continue;
            }

            let trim_px = dot_width_trim_px(
                scan_line,
                pixels_per_mm,
                dot_width_correction_mm,
            );
            let rev = is_reversed(scan_line.index);

            match scan_mode {
                ScanMode::FullSweep => process_power_full_sweep(
                    &mut ops,
                    scan_line,
                    &power_values,
                    pixels_per_mm,
                    sample_interval_mm,
                    ymax_mm,
                    rev,
                    trim_px,
                ),
                ScanMode::Segmented => process_power_segmented(
                    &mut ops,
                    scan_line,
                    &power_values,
                    pixels_per_mm,
                    sample_interval_mm,
                    ymax_mm,
                    rev,
                    trim_px,
                ),
            }
        }

        ops
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_mask_scan(
        mask: &[u8],
        height: usize,
        width: usize,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        step_power: f64,
        angle: f64,
        scan_mode: ScanMode,
        dot_width_correction_mm: f64,
    ) -> Ops {
        let mut ops = Ops::new();
        let ymax_mm =
            calculate_ymax_mm((width as i32, height as i32), pixels_per_mm);

        let bbox = match find_mask_bounding_box(mask, height, width) {
            Some(b) => b,
            None => return ops,
        };

        let scan_lines = generate_scan_lines(
            bbox,
            (width as i32, height as i32),
            pixels_per_mm,
            line_interval_mm,
            angle,
            offset_x_mm,
            offset_y_mm,
            None,
        );

        for scan_line in &scan_lines {
            if scan_line.pixels.is_empty() {
                continue;
            }

            let values = sample_mask_along_line(mask, height, width, scan_line);
            if !values.iter().any(|&v| v > 0) {
                continue;
            }

            let trim_px = dot_width_trim_px(
                scan_line,
                pixels_per_mm,
                dot_width_correction_mm,
            );
            let rev = is_reversed(scan_line.index);

            match scan_mode {
                ScanMode::FullSweep => process_scan_full_sweep(
                    &mut ops,
                    scan_line,
                    &values,
                    pixels_per_mm,
                    ymax_mm,
                    step_power,
                    rev,
                    trim_px,
                ),
                ScanMode::Segmented => process_scan_segmented(
                    &mut ops,
                    scan_line,
                    &values,
                    pixels_per_mm,
                    ymax_mm,
                    step_power,
                    rev,
                    trim_px,
                ),
            }
        }

        ops
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_mask_lines(
        mask: &[u8],
        height: usize,
        width: usize,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        z: f64,
        angle: f64,
        scan_mode: ScanMode,
    ) -> Ops {
        let mut ops = Ops::new();
        let ymax_mm =
            calculate_ymax_mm((width as i32, height as i32), pixels_per_mm);

        let bbox = match find_mask_bounding_box(mask, height, width) {
            Some(b) => b,
            None => return ops,
        };

        let scan_lines = generate_scan_lines(
            bbox,
            (width as i32, height as i32),
            pixels_per_mm,
            line_interval_mm,
            angle,
            offset_x_mm,
            offset_y_mm,
            None,
        );

        for scan_line in &scan_lines {
            if scan_line.pixels.is_empty() {
                continue;
            }

            let values = sample_mask_along_line(mask, height, width, scan_line);
            if !values.iter().any(|&v| v > 0) {
                continue;
            }

            let rev = is_reversed(scan_line.index);

            match scan_mode {
                ScanMode::FullSweep => process_line_full_sweep(
                    &mut ops,
                    scan_line,
                    &values,
                    pixels_per_mm,
                    ymax_mm,
                    z,
                    rev,
                ),
                ScanMode::Segmented => process_line_segmented(
                    &mut ops,
                    scan_line,
                    &values,
                    pixels_per_mm,
                    ymax_mm,
                    z,
                    rev,
                ),
            }
        }

        ops
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_multi_pass_image(
        gray_image: &[u8],
        height: usize,
        width: usize,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        num_depth_levels: usize,
        z_step_down: f64,
        angle: f64,
        angle_increment: f64,
        scan_mode: ScanMode,
    ) -> Ops {
        let passes = Self::multi_pass_ops(
            gray_image,
            height,
            width,
            pixels_per_mm,
            offset_x_mm,
            offset_y_mm,
            line_interval_mm,
            num_depth_levels,
            z_step_down,
            angle,
            angle_increment,
            scan_mode,
        );
        let mut ops = Ops::new();
        for p in passes {
            ops.extend(&p);
        }
        ops
    }

    #[allow(clippy::too_many_arguments)]
    pub fn multi_pass_ops(
        gray_image: &[u8],
        height: usize,
        width: usize,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        num_depth_levels: usize,
        z_step_down: f64,
        angle: f64,
        angle_increment: f64,
        scan_mode: ScanMode,
    ) -> Vec<Ops> {
        let mut passes: Vec<Ops> = Vec::new();

        let mut pass_map: Vec<i32> = Vec::with_capacity(height * width);
        for &val in gray_image {
            let level = (((255 - val as i32) as f64 / 255.0)
                * num_depth_levels as f64)
                .ceil() as i32;
            pass_map.push(level);
        }

        for pass_level in 1..=num_depth_levels {
            let mut mask = vec![0u8; height * width];
            let mut has_content = false;
            for i in 0..height * width {
                if pass_map[i] >= pass_level as i32 {
                    mask[i] = 1;
                    has_content = true;
                }
            }
            if !has_content {
                continue;
            }

            let z_offset = -((pass_level - 1) as f64 * z_step_down);
            let pass_angle = angle + (pass_level - 1) as f64 * angle_increment;
            let pass_ops = Self::from_mask_lines(
                &mask,
                height,
                width,
                pixels_per_mm,
                offset_x_mm,
                offset_y_mm,
                line_interval_mm,
                z_offset,
                pass_angle,
                scan_mode,
            );
            passes.push(pass_ops);
        }

        passes
    }
}
