use crate::ops::container::Ops;

use super::scan::{
    downsample_power_values, find_mask_bounding_box, find_segments,
    generate_scan_lines, ScanLine,
};

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
) -> Ops {
    let mut ops = Ops::new();
    let ymax_mm =
        calculate_ymax_mm((width as i32, height as i32), pixels_per_mm);

    let bbox = match find_mask_bounding_box(alpha, height, width) {
        Some(b) => b,
        None => return ops,
    };

    let power_range = max_power - min_power;

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

        let mut power_values: Vec<u8> =
            Vec::with_capacity(scan_line.pixels.len());
        let mut has_nonzero = false;

        for &(px, py) in &scan_line.pixels {
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
                let quantized =
                    (pv as f64 * (levels - 1) as f64 / 255.0).round() * 255.0
                        / (levels - 1) as f64;
                pv = quantized.round() as u8;
            }

            if pv > 0 {
                has_nonzero = true;
            }
            power_values.push(pv);
        }

        if !has_nonzero {
            continue;
        }

        let segments = find_segments(&power_values);
        if segments.is_empty() {
            continue;
        }

        let is_reversed = (scan_line.index % 2) != 0;

        let iter_segments: Vec<(usize, usize)> = if is_reversed {
            segments.into_iter().rev().collect()
        } else {
            segments
        };

        for (start_idx, end_idx) in iter_segments {
            if power_values[start_idx] == 0 {
                continue;
            }

            let seg_pixels = &scan_line.pixels[start_idx..end_idx];
            let seg_power = &power_values[start_idx..end_idx];

            let seg_start_mm = scan_line.pixel_to_mm(
                seg_pixels[0].0,
                seg_pixels[0].1,
                pixels_per_mm,
            );
            let seg_end_mm = scan_line.pixel_to_mm(
                seg_pixels[seg_pixels.len() - 1].0,
                seg_pixels[seg_pixels.len() - 1].1,
                pixels_per_mm,
            );

            let ds = downsample_power_values(
                seg_power,
                seg_start_mm,
                seg_end_mm,
                sample_interval_mm,
            );

            if ds.power.is_empty() {
                continue;
            }

            let (ds_power, ds_x_mm, ds_y_mm) = if is_reversed {
                (
                    ds.power.into_iter().rev().collect::<Vec<_>>(),
                    ds.x_mm.into_iter().rev().collect::<Vec<_>>(),
                    ds.y_mm.into_iter().rev().collect::<Vec<_>>(),
                )
            } else {
                (ds.power, ds.x_mm, ds.y_mm)
            };

            let start_x = ds_x_mm[0];
            let start_y = ds_y_mm[0];
            let end_x = ds_x_mm[ds_x_mm.len() - 1];
            let end_y = ds_y_mm[ds_y_mm.len() - 1];

            if (end_x - start_x).abs() < 1e-6 && (end_y - start_y).abs() < 1e-6
            {
                continue;
            }

            let final_start_y = convert_y_to_output(start_y, ymax_mm);
            let final_end_y = convert_y_to_output(end_y, ymax_mm);

            ops.move_to(start_x, final_start_y, 0.0, None);
            ops.scan_to(end_x, final_end_y, 0.0, Some(ds_power), None);
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

    rasterize_mask_scan_inner(
        &mut ops,
        mask,
        height,
        width,
        pixels_per_mm,
        ymax_mm,
        step_power,
        &scan_lines,
    );

    ops
}

#[allow(clippy::too_many_arguments)]
fn rasterize_mask_scan_inner(
    ops: &mut Ops,
    mask: &[u8],
    height: usize,
    width: usize,
    pixels_per_mm: (f64, f64),
    ymax_mm: f64,
    step_power: f64,
    scan_lines: &[ScanLine],
) {
    let power_byte = (255.0 * step_power).round() as u8;

    for scan_line in scan_lines {
        if scan_line.pixels.is_empty() {
            continue;
        }

        let mut values = Vec::with_capacity(scan_line.pixels.len());
        for &(px, py) in &scan_line.pixels {
            values.push(sample_image(mask, height, width, px, py));
        }

        let segments = find_segments(&values);
        if segments.is_empty() {
            continue;
        }

        let is_reversed = (scan_line.index % 2) != 0;
        let iter_segments: Vec<(usize, usize)> = if is_reversed {
            segments.into_iter().rev().collect()
        } else {
            segments
        };

        for (start_idx, end_idx) in iter_segments {
            if values[start_idx] == 0 {
                continue;
            }

            let (seg_start_px, seg_end_px) = if is_reversed {
                (scan_line.pixels[end_idx - 1], scan_line.pixels[start_idx])
            } else {
                (scan_line.pixels[start_idx], scan_line.pixels[end_idx - 1])
            };

            let start_mm = scan_line.pixel_to_mm(
                seg_start_px.0,
                seg_start_px.1,
                pixels_per_mm,
            );
            let end_mm = scan_line.pixel_to_mm(
                seg_end_px.0,
                seg_end_px.1,
                pixels_per_mm,
            );

            let segment_length_px = end_idx - start_idx;
            let power_values = vec![power_byte; segment_length_px];

            let final_start_y = convert_y_to_output(start_mm.1, ymax_mm);
            let final_end_y = convert_y_to_output(end_mm.1, ymax_mm);

            ops.move_to(start_mm.0, final_start_y, 0.0, None);
            ops.scan_to(end_mm.0, final_end_y, 0.0, Some(power_values), None);
        }
    }
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

        let mut values = Vec::with_capacity(scan_line.pixels.len());
        for &(px, py) in &scan_line.pixels {
            values.push(sample_image(mask, height, width, px, py));
        }

        let segments = find_segments(&values);
        if segments.is_empty() {
            continue;
        }

        let is_reversed = (scan_line.index % 2) != 0;
        let iter_segments: Vec<(usize, usize)> = if is_reversed {
            segments.into_iter().rev().collect()
        } else {
            segments
        };

        for (start_idx, end_idx) in iter_segments {
            if values[start_idx] == 0 {
                continue;
            }

            let (seg_start_px, seg_end_px) = if is_reversed {
                (scan_line.pixels[end_idx - 1], scan_line.pixels[start_idx])
            } else {
                (scan_line.pixels[start_idx], scan_line.pixels[end_idx - 1])
            };

            let start_mm = scan_line.pixel_to_mm(
                seg_start_px.0,
                seg_start_px.1,
                pixels_per_mm,
            );
            let end_mm = scan_line.pixel_to_mm(
                seg_end_px.0,
                seg_end_px.1,
                pixels_per_mm,
            );

            let final_start_y = convert_y_to_output(start_mm.1, ymax_mm);
            let final_end_y = convert_y_to_output(end_mm.1, ymax_mm);

            ops.move_to(start_mm.0, final_start_y, z, None);
            ops.line_to(end_mm.0, final_end_y, z, None);
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
        );
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_convert_y_to_output() {
        assert!((convert_y_to_output(0.0, 10.0) - 10.0).abs() < 1e-9);
        assert!((convert_y_to_output(10.0, 10.0) - 0.0).abs() < 1e-9);
        assert!((convert_y_to_output(5.0, 10.0) - 5.0).abs() < 1e-9);
    }
}
