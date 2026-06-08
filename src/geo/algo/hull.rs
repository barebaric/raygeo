use crate::geo::shape::polygon::get_polygon_convex_hull;
use crate::types::Point;

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
    contour.push((cx as f64, cy as f64));

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
                    contour.push((cx as f64, cy as f64));
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

fn convex_hull_points(points: &[Point]) -> Vec<Point> {
    let hull = get_polygon_convex_hull(&points.to_vec());
    if hull.len() < 3 {
        return hull;
    }
    hull
}

pub fn get_enclosing_hull(
    image: &[u8],
    width: usize,
    height: usize,
) -> Option<Vec<Point>> {
    let contours = find_external_contours(image, width, height);
    if contours.is_empty() {
        return None;
    }

    let all_points: Vec<Point> = contours.iter().flatten().copied().collect();
    if all_points.len() < 3 {
        return None;
    }

    let hull = convex_hull_points(&all_points);
    if hull.len() < 3 {
        None
    } else {
        Some(hull)
    }
}

pub fn get_hulls_from_image(
    image: &[u8],
    width: usize,
    height: usize,
) -> Vec<Vec<Point>> {
    let contours = find_external_contours(image, width, height);
    let mut result = Vec::new();

    for contour in &contours {
        if contour.len() < 3 {
            continue;
        }
        let hull = convex_hull_points(contour);
        if hull.len() >= 3 {
            result.push(hull);
        }
    }

    result
}

pub fn get_concave_hull(
    image: &[u8],
    width: usize,
    height: usize,
    gravity: f64,
) -> Option<Vec<Point>> {
    let effective_gravity = gravity.clamp(0.0, 1.0);
    if effective_gravity < 1e-6 {
        return get_enclosing_hull(image, width, height);
    }

    let contours = find_external_contours(image, width, height);
    if contours.is_empty() {
        return None;
    }

    let all_points: Vec<Point> = contours.iter().flatten().copied().collect();
    if all_points.len() < 3 {
        return get_enclosing_hull(image, width, height);
    }

    let hull_vertices = convex_hull_points(&all_points);
    let all_contour_points: Vec<Point> =
        contours.iter().flatten().copied().collect();

    let num_hull = hull_vertices.len();
    let samples_per_curve = 20;
    let mut points = Vec::new();

    for i in 0..num_hull {
        let p0 = hull_vertices[i];
        let p2 = hull_vertices[(i + 1) % num_hull];
        let midpoint = ((p0.0 + p2.0) / 2.0, (p0.1 + p2.1) / 2.0);

        let closest = find_closest_point(&all_contour_points, midpoint);

        let target_sag = (
            midpoint.0 * (1.0 - effective_gravity)
                + closest.0 * effective_gravity,
            midpoint.1 * (1.0 - effective_gravity)
                + closest.1 * effective_gravity,
        );

        let control = (
            midpoint.0 + 2.0 * (target_sag.0 - midpoint.0),
            midpoint.1 + 2.0 * (target_sag.1 - midpoint.1),
        );

        for j in 0..=samples_per_curve {
            let t = j as f64 / samples_per_curve as f64;
            let x = (1.0 - t).powi(2) * p0.0
                + 2.0 * (1.0 - t) * t * control.0
                + t * t * p2.0;
            let y = (1.0 - t).powi(2) * p0.1
                + 2.0 * (1.0 - t) * t * control.1
                + t * t * p2.1;
            points.push((x, y));
        }
    }

    Some(points)
}

fn find_closest_point(points: &[Point], target: Point) -> Point {
    let mut best = points[0];
    let mut best_dist = f64::MAX;
    for &p in points {
        let d = (p.0 - target.0).powi(2) + (p.1 - target.1).powi(2);
        if d < best_dist {
            best_dist = d;
            best = p;
        }
    }
    best
}
