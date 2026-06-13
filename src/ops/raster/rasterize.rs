use crate::ops::container::Ops;

use super::scan::{
    downsample_power_values, find_mask_bounding_box, find_segments,
    generate_scan_lines, ScanLine,
};

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

fn sample_image(
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

fn is_reversed(scan_line_index: i64) -> bool {
    (scan_line_index % 2) != 0
}

fn line_endpoints_mm(
    scan_line: &ScanLine,
    pixels_per_mm: (f64, f64),
) -> ((f64, f64), (f64, f64)) {
    let Some(first) = scan_line.pixels.first() else {
        return ((0.0, 0.0), (0.0, 0.0));
    };
    let last = scan_line.pixels.last().unwrap_or(first);
    (
        scan_line.pixel_to_mm(first.0, first.1, pixels_per_mm),
        scan_line.pixel_to_mm(last.0, last.1, pixels_per_mm),
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
    (
        scan_line.pixel_to_mm(sp.0, sp.1, pixels_per_mm),
        scan_line.pixel_to_mm(ep.0, ep.1, pixels_per_mm),
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

fn sample_mask_along_line(
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
fn compute_power_values(
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
    ops.scan_to(
        ex,
        convert_y_to_output(ey, ymax_mm),
        0.0,
        Some(ds_power),
        None,
    );
    true
}

fn emit_scan(
    ops: &mut Ops,
    start_mm: (f64, f64),
    end_mm: (f64, f64),
    ymax_mm: f64,
    power_values: Vec<u8>,
) {
    ops.move_to(
        start_mm.0,
        convert_y_to_output(start_mm.1, ymax_mm),
        0.0,
        None,
    );
    ops.scan_to(
        end_mm.0,
        convert_y_to_output(end_mm.1, ymax_mm),
        0.0,
        Some(power_values),
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

fn process_power_full_sweep(
    ops: &mut Ops,
    scan_line: &ScanLine,
    power_values: &[u8],
    pixels_per_mm: (f64, f64),
    sample_interval_mm: f64,
    ymax_mm: f64,
    rev: bool,
) {
    let (start_mm, end_mm) = line_endpoints_mm(scan_line, pixels_per_mm);
    emit_downsampled_scan(
        ops,
        power_values,
        start_mm,
        end_mm,
        sample_interval_mm,
        ymax_mm,
        rev,
    );
}

fn process_power_segmented(
    ops: &mut Ops,
    scan_line: &ScanLine,
    power_values: &[u8],
    pixels_per_mm: (f64, f64),
    sample_interval_mm: f64,
    ymax_mm: f64,
    rev: bool,
) {
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
        let (seg_start, seg_end) =
            segment_endpoints_mm(scan_line, si, ei, rev, pixels_per_mm);
        emit_downsampled_scan(
            ops,
            &power_values[si..ei],
            seg_start,
            seg_end,
            sample_interval_mm,
            ymax_mm,
            rev,
        );
    }
}

fn process_scan_full_sweep(
    ops: &mut Ops,
    scan_line: &ScanLine,
    values: &[u8],
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    step_power: f64,
    rev: bool,
) {
    let power_byte = (255.0 * step_power).round() as u8;
    let power_values: Vec<u8> = values
        .iter()
        .map(|&v| if v > 0 { power_byte } else { 0 })
        .collect();

    let (start_mm, end_mm) = endpoints_for_scan(scan_line, rev, pixels_per_mm);
    let pw = if rev {
        power_values.into_iter().rev().collect()
    } else {
        power_values
    };
    emit_scan(ops, start_mm, end_mm, ymax_mm, pw);
}

fn process_scan_segmented(
    ops: &mut Ops,
    scan_line: &ScanLine,
    values: &[u8],
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    step_power: f64,
    rev: bool,
) {
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
        let pw = vec![power_byte; ei - si];
        emit_scan(ops, start_mm, end_mm, ymax_mm, pw);
    }
}

fn process_line_full_sweep(
    ops: &mut Ops,
    scan_line: &ScanLine,
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    z: f64,
    rev: bool,
) {
    let (start_mm, end_mm) = endpoints_for_scan(scan_line, rev, pixels_per_mm);
    emit_line(ops, start_mm, end_mm, ymax_mm, z);
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

#[allow(clippy::too_many_arguments)]
pub fn rasterize_power_modulation(
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
            ),
            ScanMode::Segmented => process_power_segmented(
                &mut ops,
                scan_line,
                &power_values,
                pixels_per_mm,
                sample_interval_mm,
                ymax_mm,
                rev,
            ),
        }
    }

    ops
}

#[allow(clippy::too_many_arguments)]
pub fn rasterize_mask_scan(
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
            ScanMode::FullSweep => process_scan_full_sweep(
                &mut ops,
                scan_line,
                &values,
                pixels_per_mm,
                ymax_mm,
                step_power,
                rev,
            ),
            ScanMode::Segmented => process_scan_segmented(
                &mut ops,
                scan_line,
                &values,
                pixels_per_mm,
                ymax_mm,
                step_power,
                rev,
            ),
        }
    }

    ops
}

#[allow(clippy::too_many_arguments)]
pub fn rasterize_mask_lines(
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
pub fn rasterize_multi_pass(
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
    let mut ops = Ops::new();

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
        let pass_ops = rasterize_mask_lines(
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
        ops.extend(&pass_ops);
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_filled_image(height: usize, width: usize, val: u8) -> Vec<u8> {
        vec![val; height * width]
    }

    fn make_empty_image(height: usize, width: usize) -> Vec<u8> {
        vec![0u8; height * width]
    }

    fn make_striped_image(
        height: usize,
        width: usize,
        stripe_w: usize,
        gap_w: usize,
    ) -> Vec<u8> {
        let mut img = vec![0u8; height * width];
        let mut x = 0;
        while x < width {
            let end = (x + stripe_w).min(width);
            for y in 0..height {
                img[y * width + end - 1] = 1;
                for xi in x..end {
                    img[y * width + xi] = 1;
                }
            }
            x += stripe_w + gap_w;
        }
        img
    }

    #[test]
    fn test_is_reversed() {
        assert!(!is_reversed(0));
        assert!(!is_reversed(2));
        assert!(is_reversed(1));
        assert!(is_reversed(3));
    }

    #[test]
    fn test_compute_power_values_black_max_power() {
        let gray = vec![0u8; 1];
        let alpha = vec![255u8; 1];
        let pixels = [(0, 0)];
        let pv = compute_power_values(
            &gray, &alpha, 1, 1, &pixels, 0.0, 1.0, 1.0, 256,
        );
        assert_eq!(pv[0], 255);
    }

    #[test]
    fn test_compute_power_values_white_min_power() {
        let gray = vec![255u8; 1];
        let alpha = vec![255u8; 1];
        let pixels = [(0, 0)];
        let pv = compute_power_values(
            &gray, &alpha, 1, 1, &pixels, 0.0, 1.0, 1.0, 256,
        );
        assert_eq!(pv[0], 0);
    }

    #[test]
    fn test_compute_power_values_transparent_zero() {
        let gray = vec![0u8; 1];
        let alpha = vec![0u8; 1];
        let pixels = [(0, 0)];
        let pv = compute_power_values(
            &gray, &alpha, 1, 1, &pixels, 0.0, 1.0, 1.0, 256,
        );
        assert_eq!(pv[0], 0);
    }

    #[test]
    fn test_compute_power_values_quantization() {
        let gray = vec![0u8; 1];
        let alpha = vec![255u8; 1];
        let pixels = [(0, 0)];
        let pv = compute_power_values(
            &gray, &alpha, 1, 1, &pixels, 0.0, 1.0, 1.0, 4,
        );
        assert_eq!(pv[0], 255);
    }

    #[test]
    fn test_sample_mask_along_line() {
        let mask = vec![0, 1, 0, 1, 0, 1, 0, 1, 0];
        let sl = ScanLine {
            index: 0,
            start_mm: (0.0, 0.0),
            end_mm: (2.0, 0.0),
            pixels: vec![(0, 0), (1, 0), (2, 0)],
            line_interval_mm: 0.1,
        };
        let values = sample_mask_along_line(&mask, 3, 3, &sl);
        assert_eq!(values, vec![0, 1, 0]);
    }

    #[test]
    fn test_downsample_empty() {
        let ds = downsample_power_values(&[], (0.0, 0.0), (1.0, 0.0), 0.1);
        assert!(ds.power.is_empty());
        assert!(ds.x_mm.is_empty());
        assert!(ds.y_mm.is_empty());
    }

    #[test]
    fn test_downsample_single() {
        let ds = downsample_power_values(&[128], (0.0, 0.0), (1.0, 0.0), 0.1);
        assert_eq!(ds.power, vec![128]);
        assert_eq!(ds.x_mm, vec![0.0]);
        assert_eq!(ds.y_mm, vec![0.0]);
    }

    #[test]
    fn test_downsample_short_segment() {
        let ds =
            downsample_power_values(&[100, 200], (0.0, 0.0), (0.01, 0.0), 0.1);
        assert_eq!(ds.power.len(), 2);
        assert_eq!(ds.power[0], 100);
        assert_eq!(ds.power[1], 200);
    }

    #[test]
    fn test_downsample_long_segment() {
        let power: Vec<u8> = (0..100).map(|i| (i * 2) as u8).collect();
        let ds = downsample_power_values(&power, (0.0, 0.0), (10.0, 0.0), 1.0);
        assert!(ds.power.len() < power.len());
        assert!(ds.power.len() >= 2);
        assert!((ds.x_mm[0] - 0.0).abs() < 1e-9);
        assert!((ds.x_mm[ds.x_mm.len() - 1] - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_convert_y_to_output() {
        assert!((convert_y_to_output(0.0, 10.0) - 10.0).abs() < 1e-9);
        assert!((convert_y_to_output(10.0, 10.0) - 0.0).abs() < 1e-9);
        assert!((convert_y_to_output(5.0, 10.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_rasterize_power_modulation_empty_alpha() {
        let gray = make_filled_image(10, 10, 128);
        let alpha = make_empty_image(10, 10);
        let ops = rasterize_power_modulation(
            &gray,
            &alpha,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.05,
            0.0,
            1.0,
            1.0,
            256,
            0.0,
            ScanMode::Segmented,
        );
        assert!(ops.is_empty());
    }

    #[test]
    fn test_rasterize_power_modulation_full_image() {
        let gray = make_filled_image(10, 10, 128);
        let alpha = make_filled_image(10, 10, 255);
        let ops = rasterize_power_modulation(
            &gray,
            &alpha,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.05,
            0.0,
            1.0,
            1.0,
            256,
            0.0,
            ScanMode::Segmented,
        );
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_rasterize_mask_scan_empty() {
        let mask = make_empty_image(10, 10);
        let ops = rasterize_mask_scan(
            &mask,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            1.0,
            0.0,
            ScanMode::Segmented,
        );
        assert!(ops.is_empty());
    }

    #[test]
    fn test_rasterize_mask_scan_full() {
        let mask = make_filled_image(10, 10, 1);
        let ops = rasterize_mask_scan(
            &mask,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            1.0,
            0.0,
            ScanMode::Segmented,
        );
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_rasterize_mask_lines_empty() {
        let mask = make_empty_image(10, 10);
        let ops = rasterize_mask_lines(
            &mask,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.0,
            0.0,
            ScanMode::Segmented,
        );
        assert!(ops.is_empty());
    }

    #[test]
    fn test_rasterize_mask_lines_full() {
        let mask = make_filled_image(10, 10, 1);
        let ops = rasterize_mask_lines(
            &mask,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            -1.0,
            0.0,
            ScanMode::Segmented,
        );
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_rasterize_multi_pass_empty() {
        let gray = make_filled_image(10, 10, 255);
        let ops = rasterize_multi_pass(
            &gray,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            5,
            0.5,
            0.0,
            0.0,
            ScanMode::Segmented,
        );
        assert!(ops.is_empty());
    }

    #[test]
    fn test_rasterize_multi_pass_full() {
        let gray = make_filled_image(10, 10, 128);
        let ops = rasterize_multi_pass(
            &gray,
            10,
            10,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            3,
            0.5,
            0.0,
            0.0,
            ScanMode::Segmented,
        );
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_power_modulation_full_sweep_fewer_scans() {
        let mask = make_striped_image(10, 60, 10, 10);
        let gray = make_filled_image(10, 60, 128);
        let alpha = make_filled_image(10, 60, 255);

        let ops_seg = rasterize_power_modulation(
            &gray,
            &alpha,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.05,
            0.0,
            1.0,
            1.0,
            256,
            0.0,
            ScanMode::Segmented,
        );
        let ops_full = rasterize_power_modulation(
            &gray,
            &alpha,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.05,
            0.0,
            1.0,
            1.0,
            256,
            0.0,
            ScanMode::FullSweep,
        );
        assert!(!ops_full.is_empty());
        assert!(ops_full.len() < ops_seg.len());
    }

    #[test]
    fn test_mask_scan_full_sweep_fewer_scans() {
        let mask = make_striped_image(10, 60, 10, 10);

        let ops_seg = rasterize_mask_scan(
            &mask,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            1.0,
            0.0,
            ScanMode::Segmented,
        );
        let ops_full = rasterize_mask_scan(
            &mask,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            1.0,
            0.0,
            ScanMode::FullSweep,
        );
        assert!(!ops_full.is_empty());
        assert!(ops_full.len() < ops_seg.len());
    }

    #[test]
    fn test_mask_lines_full_sweep_fewer_lines() {
        let mask = make_striped_image(10, 60, 10, 10);

        let ops_seg = rasterize_mask_lines(
            &mask,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.0,
            0.0,
            ScanMode::Segmented,
        );
        let ops_full = rasterize_mask_lines(
            &mask,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            0.0,
            0.0,
            ScanMode::FullSweep,
        );
        assert!(!ops_full.is_empty());
        assert!(ops_full.len() < ops_seg.len());
    }

    #[test]
    fn test_multi_pass_full_sweep_fewer_lines() {
        let gray = make_filled_image(10, 60, 64);

        let ops_seg = rasterize_multi_pass(
            &gray,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            2,
            0.5,
            0.0,
            0.0,
            ScanMode::Segmented,
        );
        let ops_full = rasterize_multi_pass(
            &gray,
            10,
            60,
            (10.0, 10.0),
            0.0,
            0.0,
            0.1,
            2,
            0.5,
            0.0,
            0.0,
            ScanMode::FullSweep,
        );
        assert!(!ops_full.is_empty());
        assert!(ops_full.len() < ops_seg.len());
    }
}
