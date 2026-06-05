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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_segments_empty() {
        assert_eq!(find_segments(&[]), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn test_find_segments_all_zeros() {
        assert_eq!(find_segments(&[0, 0, 0]), Vec::<(usize, usize)>::new());
    }

    #[test]
    fn test_find_segments_all_ones() {
        assert_eq!(find_segments(&[1, 1, 1]), vec![(0, 3)]);
    }

    #[test]
    fn test_find_segments_single() {
        assert_eq!(find_segments(&[0, 1, 0]), vec![(1, 2)]);
    }

    #[test]
    fn test_find_segments_multiple() {
        assert_eq!(find_segments(&[0, 1, 1, 0, 1, 0]), vec![(1, 3), (4, 5)]);
    }

    #[test]
    fn test_find_segments_starts_with_value() {
        assert_eq!(find_segments(&[1, 1, 0, 0]), vec![(0, 2)]);
    }

    #[test]
    fn test_find_segments_ends_with_value() {
        assert_eq!(find_segments(&[0, 0, 1, 1]), vec![(2, 4)]);
    }

    #[test]
    fn test_find_segments_non_binary() {
        assert_eq!(find_segments(&[0, 5, 10, 0, 3]), vec![(1, 3), (4, 5)]);
    }

    #[test]
    fn test_find_segments_adjacent() {
        assert_eq!(find_segments(&[1, 0, 1, 1]), vec![(0, 1), (2, 4)]);
    }

    #[test]
    fn test_find_mask_bounding_box_empty() {
        let mask = vec![0u8; 10 * 10];
        assert_eq!(find_mask_bounding_box(&mask, 10, 10), None);
    }

    #[test]
    fn test_find_mask_bounding_box_full() {
        let mask = vec![1u8; 10 * 10];
        assert_eq!(find_mask_bounding_box(&mask, 10, 10), Some((0, 9, 0, 9)));
    }

    #[test]
    fn test_find_mask_bounding_box_single_pixel() {
        let mut mask = vec![0u8; 10 * 10];
        mask[3 * 10 + 7] = 1;
        assert_eq!(find_mask_bounding_box(&mask, 10, 10), Some((3, 3, 7, 7)));
    }

    #[test]
    fn test_find_mask_bounding_box_corner() {
        let mut mask = vec![0u8; 10 * 10];
        mask[9 * 10 + 9] = 1;
        assert_eq!(find_mask_bounding_box(&mask, 10, 10), Some((9, 9, 9, 9)));
    }

    #[test]
    fn test_line_pixels_horizontal() {
        let pixels = line_pixels((0.0, 5.0), (10.0, 5.0), 11, 11);
        assert_eq!(pixels.len(), 11);
        assert_eq!(pixels[0], (0, 5));
        assert_eq!(pixels[10], (10, 5));
    }

    #[test]
    fn test_line_pixels_vertical() {
        let pixels = line_pixels((5.0, 0.0), (5.0, 10.0), 11, 11);
        assert_eq!(pixels.len(), 11);
        assert_eq!(pixels[0], (5, 0));
        assert_eq!(pixels[10], (5, 10));
    }

    #[test]
    fn test_line_pixels_diagonal() {
        let pixels = line_pixels((0.0, 0.0), (10.0, 10.0), 11, 11);
        assert_eq!(pixels.len(), 11);
        assert_eq!(pixels[0], (0, 0));
        assert_eq!(pixels[10], (10, 10));
    }

    #[test]
    fn test_line_pixels_outside_bounds() {
        let pixels = line_pixels((-5.0, 5.0), (15.0, 5.0), 11, 11);
        for &(x, y) in &pixels {
            assert!(x >= 0 && x < 11);
            assert!(y >= 0 && y < 11);
        }
    }

    #[test]
    fn test_line_pixels_single_pixel() {
        let pixels = line_pixels((5.0, 5.0), (5.0, 5.0), 11, 11);
        assert_eq!(pixels, vec![(5, 5)]);
    }

    #[test]
    fn test_scan_line_length_horizontal() {
        let sl = ScanLine {
            index: 0,
            start_mm: (0.0, 0.0),
            end_mm: (10.0, 0.0),
            pixels: vec![],
            line_interval_mm: 0.1,
        };
        assert!((sl.length_mm() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_scan_line_length_vertical() {
        let sl = ScanLine {
            index: 0,
            start_mm: (0.0, 0.0),
            end_mm: (0.0, 5.0),
            pixels: vec![],
            line_interval_mm: 0.1,
        };
        assert!((sl.length_mm() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_scan_line_length_diagonal() {
        let sl = ScanLine {
            index: 0,
            start_mm: (0.0, 0.0),
            end_mm: (3.0, 4.0),
            pixels: vec![],
            line_interval_mm: 0.1,
        };
        assert!((sl.length_mm() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_scan_line_direction_horizontal() {
        let sl = ScanLine {
            index: 0,
            start_mm: (0.0, 0.0),
            end_mm: (10.0, 0.0),
            pixels: vec![],
            line_interval_mm: 0.1,
        };
        let (dx, dy) = sl.direction();
        assert!((dx - 1.0).abs() < 1e-9);
        assert!(dy.abs() < 1e-9);
    }

    #[test]
    fn test_scan_line_direction_zero_length() {
        let sl = ScanLine {
            index: 0,
            start_mm: (5.0, 5.0),
            end_mm: (5.0, 5.0),
            pixels: vec![],
            line_interval_mm: 0.1,
        };
        let (dx, dy) = sl.direction();
        assert!((dx - 1.0).abs() < 1e-9);
        assert!(dy.abs() < 1e-9);
    }

    #[test]
    fn test_scan_line_pixel_to_mm_horizontal() {
        let sl = ScanLine {
            index: 0,
            start_mm: (0.0, 0.5),
            end_mm: (10.0, 0.5),
            pixels: vec![],
            line_interval_mm: 0.1,
        };
        let (x, _y) = sl.pixel_to_mm(50, 5, (10.0, 10.0));
        assert!((x - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_scan_line_pixel_to_mm_vertical() {
        let sl = ScanLine {
            index: 0,
            start_mm: (0.5, 0.0),
            end_mm: (0.5, 10.0),
            pixels: vec![],
            line_interval_mm: 0.1,
        };
        let (_x, y) = sl.pixel_to_mm(5, 50, (10.0, 10.0));
        assert!((y - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_generate_scan_lines_horizontal_count() {
        let lines = generate_scan_lines(
            (0, 9, 0, 9),
            (10, 10),
            (10.0, 10.0),
            0.1,
            0.0,
            0.0,
            0.0,
            None,
        );
        assert!(!lines.is_empty());
        assert!(lines.len() >= 10);
    }

    #[test]
    fn test_generate_scan_lines_vertical() {
        let lines = generate_scan_lines(
            (0, 9, 0, 9),
            (10, 10),
            (10.0, 10.0),
            0.1,
            90.0,
            0.0,
            0.0,
            None,
        );
        assert!(!lines.is_empty());
        assert!(lines.len() >= 10);
    }

    #[test]
    fn test_generate_scan_lines_sequential_indices() {
        let lines = generate_scan_lines(
            (0, 9, 0, 9),
            (10, 10),
            (10.0, 10.0),
            0.1,
            0.0,
            0.0,
            0.0,
            None,
        );
        for i in 1..lines.len() {
            assert_eq!(lines[i].index, lines[i - 1].index + 1);
        }
    }

    #[test]
    fn test_generate_scan_lines_spacing() {
        let lines = generate_scan_lines(
            (0, 9, 0, 9),
            (10, 10),
            (10.0, 10.0),
            0.1,
            0.0,
            0.0,
            0.0,
            None,
        );
        if lines.len() >= 2 {
            let dy0 = (lines[1].start_mm.1 - lines[0].start_mm.1).abs();
            let dy1 = (lines[1].end_mm.1 - lines[0].end_mm.1).abs();
            assert!((dy0 - 0.1).abs() < 0.01);
            assert!((dy1 - 0.1).abs() < 0.01);
        }
    }

    #[test]
    fn test_generate_scan_lines_pixels_within_bounds() {
        let lines = generate_scan_lines(
            (0, 99, 0, 99),
            (100, 100),
            (10.0, 10.0),
            0.1,
            0.0,
            0.0,
            0.0,
            None,
        );
        for sl in &lines {
            for &(x, y) in &sl.pixels {
                assert!(x >= 0 && x < 100);
                assert!(y >= 0 && y < 100);
            }
        }
    }

    #[test]
    fn test_generate_horizontal_scan_positions_basic() {
        let (mm, px) = generate_horizontal_scan_positions(
            0,
            9,
            10,
            (10.0, 10.0),
            0.1,
            0.0,
        );
        assert!(!mm.is_empty());
        assert_eq!(mm.len(), px.len());
    }

    #[test]
    fn test_generate_horizontal_scan_positions_empty() {
        let (mm, px) = generate_horizontal_scan_positions(
            20,
            10,
            30,
            (10.0, 10.0),
            0.1,
            0.0,
        );
        assert!(mm.is_empty());
        assert!(px.is_empty());
    }

    #[test]
    fn test_resample_rows_identity() {
        let image = vec![100u8; 3 * 4];
        let y_coords = vec![0.0, 1.0, 2.0];
        let result = resample_rows(&image, 3, 4, &y_coords);
        assert_eq!(result.len(), 12);
    }

    #[test]
    fn test_resample_rows_empty() {
        let result = resample_rows(&[], 0, 0, &[]);
        assert!(result.is_empty());
    }
}
