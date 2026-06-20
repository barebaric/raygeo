use crate::types::{Command, Point, Point3D};

/// Simplify a sequence of 3D points using the Ramer-Douglas-Peucker algorithm.
pub fn simplify_polyline(points: &[Point3D], tolerance: f64) -> Vec<Point3D> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }

    let tol_sq = tolerance * tolerance;
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;

    let mut stack: Vec<(usize, usize)> = vec![(0, n - 1)];

    while let Some((start, end)) = stack.pop() {
        if end - start < 2 {
            continue;
        }

        let p_start = (points[start].x, points[start].y);
        let p_end = (points[end].x, points[end].y);
        let chord_vec = (p_end.0 - p_start.0, p_end.1 - p_start.1);
        let chord_len_sq =
            chord_vec.0 * chord_vec.0 + chord_vec.1 * chord_vec.1;

        let mut max_dist_sq = 0.0_f64;
        let mut max_idx = start;

        if chord_len_sq < 1e-12 {
            for (i, p) in points.iter().enumerate().take(end).skip(start + 1) {
                let d_sq =
                    (p.x - p_start.0).powi(2) + (p.y - p_start.1).powi(2);
                if d_sq > max_dist_sq {
                    max_dist_sq = d_sq;
                    max_idx = i;
                }
            }
        } else {
            for (i, p) in points.iter().enumerate().take(end).skip(start + 1) {
                let cross = (Point::new(p.x, p.y)
                    - Point::new(p_start.0, p_start.1))
                .perp_dot(Point::new(chord_vec.0, chord_vec.1));
                let d_sq = (cross * cross) / chord_len_sq;
                if d_sq > max_dist_sq {
                    max_dist_sq = d_sq;
                    max_idx = i;
                }
            }
        }

        if max_dist_sq > tol_sq {
            keep[max_idx] = true;
            stack.push((start, max_idx));
            stack.push((max_idx, end));
        }
    }

    points
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, p)| *p)
        .collect()
}

fn chord_len_sq(p_start: &Point3D, p_end: &Point3D) -> f64 {
    let dx = p_end.x - p_start.x;
    let dy = p_end.y - p_start.y;
    dx * dx + dy * dy
}

/// Simplify geometry command data using the Ramer-Douglas-Peucker algorithm.
///
/// Uses an iterative stack-based approach (not recursion). The first and last
/// points of each subpath are always preserved. MOVE commands are always kept.
pub fn simplify_data(data: &[Command], tolerance: f64) -> Vec<Command> {
    let n = data.len();
    if n < 3 {
        return data.to_vec();
    }

    let tol_sq = tolerance * tolerance;
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;

    let mut stack: Vec<(usize, usize)> = vec![(0, n - 1)];

    while let Some((start, end)) = stack.pop() {
        if end - start < 2 {
            continue;
        }

        let p_start = data[start].end_point();
        let p_end = data[end].end_point();
        let chord_len_sq_val = chord_len_sq(&p_start, &p_end);

        let mut max_dist_sq = 0.0_f64;
        let mut max_idx = start;

        if chord_len_sq_val < 1e-12 {
            for (i, cmd) in data.iter().enumerate().take(end).skip(start + 1) {
                if matches!(cmd, Command::Move { .. }) {
                    keep[i] = true;
                    continue;
                }
                let p = cmd.end_point();
                let d_sq =
                    (p.x - p_start.x).powi(2) + (p.y - p_start.y).powi(2);
                if d_sq > max_dist_sq {
                    max_dist_sq = d_sq;
                    max_idx = i;
                }
            }
        } else {
            for (i, cmd) in data.iter().enumerate().take(end).skip(start + 1) {
                if matches!(cmd, Command::Move { .. }) {
                    keep[i] = true;
                    continue;
                }
                let p = cmd.end_point();
                let cross = (Point::new(p.x, p.y)
                    - Point::new(p_start.x, p_start.y))
                .perp_dot(Point::new(p_end.x - p_start.x, p_end.y - p_start.y));
                let d_sq = (cross * cross) / chord_len_sq_val;
                if d_sq > max_dist_sq {
                    max_dist_sq = d_sq;
                    max_idx = i;
                }
            }
        }

        if max_dist_sq > tol_sq {
            keep[max_idx] = true;
            stack.push((start, max_idx));
            stack.push((max_idx, end));
        }
    }

    data.iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, cmd)| cmd.clone())
        .collect()
}
