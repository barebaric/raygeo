//! Trace: contour extraction from binary images.
//!
//! Provides boundary tracing of foreground regions in a boolean image,
//! returning ordered point loops around each component.

use crate::geo::types::Point;

/// Extracts the ordered boundary contour of each foreground region in a
/// binary image. Pixels with value 0 are treated as background; non-zero
/// values are foreground. Contours with fewer than 3 points are dropped.
pub fn find_external_contours(
    image: &[u8],
    width: usize,
    height: usize,
) -> Vec<Vec<Point>> {
    let mut visited = vec![false; width * height];
    let mut contours: Vec<Vec<Point>> = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if image[idx] == 0 || visited[idx] {
                continue;
            }
            if x == 0 || image[idx - 1] == 0 {
                let contour =
                    trace_contour(image, width, height, &mut visited, x, y);
                if contour.len() >= 3 {
                    contours.push(contour);
                }
            }
        }
    }
    contours
}

fn trace_contour(
    image: &[u8],
    width: usize,
    height: usize,
    visited: &mut [bool],
    start_x: usize,
    start_y: usize,
) -> Vec<Point> {
    let mut contour: Vec<Point> = Vec::new();
    let dirs: [(i32, i32); 8] = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];

    let mut cx = start_x as i32;
    let mut cy = start_y as i32;
    contour.push(Point::new(cx as f64, cy as f64));

    let mut dir_idx = 7;
    let max_steps = width * height * 2;

    for _ in 0..max_steps {
        let search_start = (dir_idx + 6) % 8;
        let mut found = false;

        for i in 0..8 {
            let d = (search_start + i) % 8;
            let nx = cx + dirs[d].0;
            let ny = cy + dirs[d].1;
            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                let nidx = ny as usize * width + nx as usize;
                if image[nidx] != 0 {
                    cx = nx;
                    cy = ny;
                    dir_idx = d;
                    visited[nidx] = true;
                    contour.push(Point::new(cx as f64, cy as f64));
                    found = true;
                    break;
                }
            }
        }

        if !found {
            break;
        }

        if cx == start_x as i32 && cy == start_y as i32 {
            break;
        }
    }

    contour
}
