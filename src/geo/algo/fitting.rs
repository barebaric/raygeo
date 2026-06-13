use std::f64::consts::PI;

use crate::geo::algo::simplify::simplify_polyline;
use crate::geo::shape::arc::is_arc_clockwise;
use crate::geo::shape::arc::{get_arc_angles, linearize_arc};
use crate::geo::shape::bezier::linearize_bezier_from_params;
use crate::types::{Command, Point, Point3D};

/// Converts all arc commands in a geometry data array into cubic bezier approximations.
pub fn convert_arcs_to_beziers(data: &[Command]) -> Vec<Command> {
    if data.is_empty() {
        return vec![];
    }
    let mut result: Vec<Command> = Vec::new();
    let mut last_pos = (0.0, 0.0, 0.0);
    for cmd in data {
        let end_pos = cmd.end_point();

        match cmd {
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => {
                let bezier_cmds = convert_arc_to_beziers_from_array(
                    last_pos,
                    end_pos,
                    *center_offset,
                    *clockwise,
                );
                result.extend(bezier_cmds);
            }
            _ => {
                result.push(cmd.clone());
            }
        }
        last_pos = end_pos;
    }
    result
}

/// Converts all arc and bezier commands into chains of line segments.
pub fn linearize_data(data: &[Command], tolerance: f64) -> Vec<Command> {
    if data.is_empty() {
        return vec![];
    }
    let mut result: Vec<Command> = Vec::new();
    let mut last_pos = (0.0, 0.0, 0.0);
    for cmd in data {
        let end_pos = cmd.end_point();

        match cmd {
            Command::Move { .. } | Command::Line { .. } => {
                result.push(cmd.clone());
            }
            Command::Arc {
                end,
                center_offset,
                clockwise,
                ..
            } => {
                let segments = linearize_arc(
                    *end,
                    *center_offset,
                    *clockwise,
                    last_pos,
                    tolerance,
                );
                for (_, p_end) in segments {
                    result.push(Command::Line { end: p_end });
                }
            }
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => {
                let segments = linearize_bezier_from_params(
                    *end, *control1, *control2, last_pos, tolerance,
                );
                for (_, p_end) in segments {
                    result.push(Command::Line { end: p_end });
                }
            }
        }
        last_pos = end_pos;
    }
    result
}

/// Converts geometry data into a list of dense point lists (one per subpath).
pub fn flatten_to_points(
    data: &[Command],
    resolution: f64,
) -> Vec<Vec<Point3D>> {
    if data.is_empty() {
        return vec![];
    }

    let mut subpaths: Vec<Vec<Point3D>> = Vec::new();
    let mut current_subpath: Vec<Point3D> = Vec::new();
    let mut last_pos = (0.0, 0.0, 0.0);

    for cmd in data {
        let end_pos = cmd.end_point();

        match cmd {
            Command::Move { .. } => {
                if !current_subpath.is_empty() {
                    subpaths.push(current_subpath);
                    current_subpath = Vec::new();
                }
                current_subpath.push(end_pos);
            }
            Command::Line { .. } => {
                current_subpath.push(end_pos);
            }
            Command::Arc {
                end,
                center_offset,
                clockwise,
                ..
            } => {
                let segments = linearize_arc(
                    *end,
                    *center_offset,
                    *clockwise,
                    last_pos,
                    resolution,
                );
                for (_, p_end) in segments {
                    current_subpath.push(p_end);
                }
            }
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => {
                let segments = linearize_bezier_from_params(
                    *end, *control1, *control2, last_pos, resolution,
                );
                for (_, p_end) in segments {
                    current_subpath.push(p_end);
                }
            }
        }

        last_pos = end_pos;
    }

    if !current_subpath.is_empty() {
        subpaths.push(current_subpath);
    }

    subpaths
}

/// Converts geometry data to a polyline approximation (Lines only),
/// reducing vertex count using the Ramer-Douglas-Peucker algorithm.
pub fn linearize_geometry(data: &[Command], tolerance: f64) -> Vec<Command> {
    if data.is_empty() {
        return vec![];
    }

    let resolution = tolerance * 0.25;
    let subpaths_points = flatten_to_points(data, resolution);

    let mut new_cmds: Vec<Command> = Vec::new();
    for points in &subpaths_points {
        if points.is_empty() {
            continue;
        }

        let simplified = simplify_polyline(points, tolerance);

        if !simplified.is_empty() {
            let p0 = simplified[0];
            new_cmds.push(Command::Move { end: p0 });

            for p in simplified.iter().skip(1) {
                new_cmds.push(Command::Line { end: *p });
            }
        }
    }

    new_cmds
}

/// Tests whether a sequence of points lies on a straight line within the given tolerance.
pub fn are_points_collinear(points: &[Point3D], tolerance: f64) -> bool {
    if points.len() < 3 {
        return true;
    }

    let p1 = points[0];
    let p2 = points[points.len() - 1];
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    let line_length = dx.hypot(dy);

    if line_length < 1e-9 {
        return points
            .iter()
            .all(|p| (p.0 - p1.0).hypot(p.1 - p1.1) < tolerance);
    }

    for p in points.iter().skip(1).take(points.len() - 2) {
        let vx = p.0 - p1.0;
        let vy = p.1 - p1.1;
        let dist = (vx * dy - vy * dx).abs() / line_length;
        if dist > tolerance {
            return false;
        }
    }
    true
}

/// Fits a circle through three points using the perpendicular bisector method.
pub fn fit_circle_to_3_points(
    p1: Point3D,
    p2: Point3D,
    p3: Point3D,
) -> Option<(Point, f64)> {
    let (x1, y1) = (p1.0, p1.1);
    let (x2, y2) = (p2.0, p2.1);
    let (x3, y3) = (p3.0, p3.1);

    let area = x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2);
    if area.abs() < 1e-9 {
        return None;
    }

    let d12 = 2.0 * ((y2 - y1) * (x3 - x2) - (y3 - y2) * (x2 - x1));
    if d12.abs() < 1e-9 {
        return None;
    }

    let sq1 = x1 * x1 + y1 * y1;
    let sq2 = x2 * x2 + y2 * y2;
    let sq3 = x3 * x3 + y3 * y3;

    let xc = ((sq1 - sq2) * (y3 - y2) - (sq2 - sq3) * (y2 - y1)) / d12;
    let yc = ((x2 - x1) * (sq2 - sq3) - (x3 - x2) * (sq1 - sq2)) / d12;

    let center = (xc, yc);
    let radius = (x1 - xc).hypot(y1 - yc);
    Some((center, radius))
}

fn solve_3x3(ata: [[f64; 3]; 3], atb: [f64; 3]) -> Option<[f64; 3]> {
    let mut a = ata;
    let mut b = atb;

    for col in 0..3 {
        let mut max_val = a[col][col].abs();
        let mut max_row = col;
        for (row, row_vals) in a.iter().enumerate().skip(col + 1) {
            let val = row_vals[col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }
        if max_val < 1e-15 {
            return None;
        }
        a.swap(col, max_row);
        b.swap(col, max_row);

        let pivot = a[col][col];
        for item in a[col].iter_mut().skip(col) {
            *item /= pivot;
        }
        b[col] /= pivot;

        let ref_row = a[col];
        for (row_idx, row_vals) in a.iter_mut().enumerate().skip(col + 1) {
            let factor = row_vals[col];
            for (j, item) in row_vals.iter_mut().enumerate().skip(col) {
                *item -= factor * ref_row[j];
            }
            b[row_idx] -= factor * b[col];
        }
    }

    let mut x = [0.0; 3];
    for i in (0..3).rev() {
        x[i] = b[i];
        for j in (i + 1)..3 {
            x[i] -= a[i][j] * x[j];
        }
    }
    Some(x)
}

/// Fits a circle to a set of points using Kasa's least-squares method.
pub fn fit_circle_to_points(points: &[Point3D]) -> Option<(Point, f64, f64)> {
    if points.len() < 3 || are_points_collinear(points, 0.01) {
        return None;
    }

    let n = points.len();
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut sx = 0.0;
    let mut syy = 0.0;
    let mut sy = 0.0;
    let mut sbx = 0.0;
    let mut sby = 0.0;
    let mut sb = 0.0;

    for p in points {
        let x = p.0;
        let y = p.1;
        let x2y2 = x * x + y * y;
        sxx += 2.0 * x * 2.0 * x;
        sxy += 2.0 * x * 2.0 * y;
        sx += 2.0 * x;
        syy += 2.0 * y * 2.0 * y;
        sy += 2.0 * y;
        sbx += 2.0 * x * x2y2;
        sby += 2.0 * y * x2y2;
        sb += x2y2;
    }
    let sn = n as f64;

    let ata = [[sxx, sxy, sx], [sxy, syy, sy], [sx, sy, sn]];
    let atb = [sbx, sby, sb];

    let result = solve_3x3(ata, atb)?;
    let (xc, yc, c) = (result[0], result[1], result[2]);

    let r_sq = xc * xc + yc * yc + c;
    if r_sq < 1e-10 {
        return None;
    }
    let r = r_sq.sqrt();
    let center = (xc, yc);

    let mut max_err = 0.0;
    for p in points {
        let dist = (p.0 - xc).hypot(p.1 - yc);
        let err = (dist - r).abs();
        if err > max_err {
            max_err = err;
        }
    }

    Some((center, r, max_err))
}

/// Projects a circle center onto the perpendicular bisector of chord `p1`–`p2`.
pub fn project_circle_center_to_bisector(
    p1: Point3D,
    p2: Point3D,
    center: Point,
) -> Point {
    let (x1, y1) = (p1.0, p1.1);
    let (x2, y2) = (p2.0, p2.1);
    let (cx, cy) = center;

    let dx = x2 - x1;
    let dy = y2 - y1;
    let chord_len_sq = dx * dx + dy * dy;

    if chord_len_sq < 1e-12 {
        return center;
    }

    let mx = (x1 + x2) / 2.0;
    let my = (y1 + y2) / 2.0;
    let vx = cx - mx;
    let vy = cy - my;
    let dot = vx * dx + vy * dy;
    let proj_factor = dot / chord_len_sq;
    let proj_x = dx * proj_factor;
    let proj_y = dy * proj_factor;

    (cx - proj_x, cy - proj_y)
}

/// Computes the maximum deviation of a polyline from a reference arc defined by
/// `center` and `radius`.
pub fn get_polyline_arc_deviation(
    points: &[Point3D],
    center: Point,
    radius: f64,
) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let (xc, yc) = center;
    let mut max_deviation = 0.0_f64;

    for i in 0..(points.len() - 1) {
        let p1 = points[i];
        let p2 = points[i + 1];
        let (x1, y1) = (p1.0, p1.1);
        let (x2, y2) = (p2.0, p2.1);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let seg_len = dx.hypot(dy);

        if seg_len < 1e-9 {
            let dev = ((x1 - xc).hypot(y1 - yc) - radius).abs();
            max_deviation = max_deviation.max(dev);
            continue;
        }

        let d1 = (x1 - xc).hypot(y1 - yc);
        let d2 = (x2 - xc).hypot(y2 - yc);

        if seg_len > 2.0 * radius {
            let dev = (d1 - radius).abs().max((d2 - radius).abs());
            max_deviation = max_deviation.max(dev);
        } else {
            let v1x = x1 - xc;
            let v1y = y1 - yc;
            let v2x = x2 - xc;
            let v2y = y2 - yc;
            let dot = v1x * v2x + v1y * v2y;
            let mag1 = v1x.hypot(v1y);
            let mag2 = v2x.hypot(v2y);

            let sagitta = if mag1 < 1e-9 || mag2 < 1e-9 {
                0.0
            } else {
                let cos_theta = (dot / (mag1 * mag2)).clamp(-1.0, 1.0);
                let theta = cos_theta.acos();
                radius * (1.0 - (theta / 2.0).cos())
            };
            let endpoint_dev = (d1 - radius).abs().max((d2 - radius).abs());
            max_deviation = max_deviation.max(sagitta.max(endpoint_dev));
        }
    }
    max_deviation
}

/// Converts a single arc command into one or more cubic bezier segments.
pub fn convert_arc_to_beziers_from_array(
    start_point: Point3D,
    end_point: Point3D,
    center_offset: (f64, f64),
    clockwise: bool,
) -> Vec<Command> {
    let p0_2d = (start_point.0, start_point.1);
    let p_end_2d = (end_point.0, end_point.1);
    let z_start = start_point.2;
    let z_end = end_point.2;

    let center = (p0_2d.0 + center_offset.0, p0_2d.1 + center_offset.1);
    let radius = center_offset.0.hypot(center_offset.1);
    let radius_end = (p_end_2d.0 - center.0).hypot(p_end_2d.1 - center.1);

    if radius < 1e-9 {
        return vec![];
    }

    let is_coincident = (start_point.0 - end_point.0).abs() < 1e-12
        && (start_point.1 - end_point.1).abs() < 1e-12;

    let (start_angle, total_sweep) = if is_coincident {
        let sa = (p0_2d.1 - center.1).atan2(p0_2d.0 - center.0);
        let sweep = if clockwise { -2.0 * PI } else { 2.0 * PI };
        (sa, sweep)
    } else {
        let (sa, _, sweep) = get_arc_angles(p0_2d, p_end_2d, center, clockwise);
        (sa, sweep)
    };

    if total_sweep.abs() < 1e-8 {
        return vec![];
    }

    let num_segments = 1.max((total_sweep.abs() / (PI / 2.0)).ceil() as usize);
    let segment_sweep = total_sweep / num_segments as f64;
    let kappa = (4.0 / 3.0) * (segment_sweep.abs() / 4.0).tan();

    let mut bezier_cmds: Vec<Command> = Vec::new();
    let mut current_p0 = start_point;

    for i in 0..num_segments {
        let angle1 = start_angle + (i + 1) as f64 * segment_sweep;

        let current_p3 = if i == num_segments - 1 {
            end_point
        } else {
            let t1 = (i + 1) as f64 / num_segments as f64;
            let radius1 = radius + t1 * (radius_end - radius);
            let p3x = center.0 + radius1 * angle1.cos();
            let p3y = center.1 + radius1 * angle1.sin();
            let p3z = z_start + t1 * (z_end - z_start);
            (p3x, p3y, p3z)
        };

        let r_vec0 = (current_p0.0 - center.0, current_p0.1 - center.1);
        let r_vec1 = (current_p3.0 - center.0, current_p3.1 - center.1);

        let t_vec0 = if clockwise {
            (r_vec0.1, -r_vec0.0)
        } else {
            (-r_vec0.1, r_vec0.0)
        };
        let t_vec1 = if clockwise {
            (r_vec1.1, -r_vec1.0)
        } else {
            (-r_vec1.1, r_vec1.0)
        };

        let c1 = (
            current_p0.0 + t_vec0.0 * kappa,
            current_p0.1 + t_vec0.1 * kappa,
        );
        let c2 = (
            current_p3.0 - t_vec1.0 * kappa,
            current_p3.1 - t_vec1.1 * kappa,
        );

        bezier_cmds.push(Command::Bezier {
            end: current_p3,
            control1: c1,
            control2: c2,
        });

        current_p0 = current_p3;
    }

    bezier_cmds
}

/// Returns the maximum perpendicular deviation from the chord `points[start]`–`points[end]`
/// and the index of the furthest point.
pub fn get_polyline_line_deviation(
    points: &[Point3D],
    start: usize,
    end: usize,
) -> (f64, usize) {
    let p_start = points[start];
    let p_end = points[end];
    let dx = p_end.0 - p_start.0;
    let dy = p_end.1 - p_start.1;
    let line_len_sq = dx * dx + dy * dy;

    let mut max_dist_sq = 0.0;
    let mut max_idx = start;

    if line_len_sq < 1e-12 {
        for (i, p) in points.iter().enumerate().take(end).skip(start + 1) {
            let d_sq = (p.0 - p_start.0).powi(2) + (p.1 - p_start.1).powi(2);
            if d_sq > max_dist_sq {
                max_dist_sq = d_sq;
                max_idx = i;
            }
        }
        return (max_dist_sq.sqrt(), max_idx);
    }

    for (i, p) in points.iter().enumerate().take(end).skip(start + 1) {
        let cross = (p.0 - p_start.0) * dy - (p.1 - p_start.1) * dx;
        let d_sq = (cross * cross) / line_len_sq;
        if d_sq > max_dist_sq {
            max_dist_sq = d_sq;
            max_idx = i;
        }
    }

    (max_dist_sq.sqrt(), max_idx)
}

/// Creates a `Command::Line` for the given endpoint.
pub fn create_line_cmd(end_point: Point3D) -> Command {
    Command::Line { end: end_point }
}

/// Creates a `Command::Arc` for the given endpoint. The clockwise flag is determined
/// automatically from the cross product of the start → center and end → center vectors.
pub fn create_arc_cmd(
    end_point: Point3D,
    center: Point,
    start_point: Point3D,
) -> Command {
    let (xc, yc) = center;
    let v1x = start_point.0 - xc;
    let v1y = start_point.1 - yc;
    let v2x = end_point.0 - xc;
    let v2y = end_point.1 - yc;
    let cross = v1x * v2y - v1y * v2x;
    let clockwise = cross < 0.0;

    Command::Arc {
        end: end_point,
        center_offset: (xc - start_point.0, yc - start_point.1),
        clockwise,
    }
}

/// Recursively fits line and arc primitives to a range of points.
pub fn fit_points_recursive(
    points: &[Point3D],
    tolerance: f64,
    start: usize,
    end: usize,
) -> Vec<Command> {
    if start >= end {
        return vec![];
    }

    let (max_dist, split_idx) = get_polyline_line_deviation(points, start, end);
    if max_dist < tolerance {
        return vec![create_line_cmd(points[end])];
    }

    let is_sharp = if start < split_idx && split_idx < end {
        let p_prev = points[split_idx - 1];
        let p_curr = points[split_idx];
        let p_next = points[split_idx + 1];
        let dx1 = p_curr.0 - p_prev.0;
        let dy1 = p_curr.1 - p_prev.1;
        let dx2 = p_next.0 - p_curr.0;
        let dy2 = p_next.1 - p_curr.1;
        let len1 = dx1.hypot(dy1);
        let len2 = dx2.hypot(dy2);
        if len1 > 1e-9 && len2 > 1e-9 {
            let dot = (dx1 * dx2 + dy1 * dy2) / (len1 * len2);
            dot < 0.5
        } else {
            false
        }
    } else {
        false
    };

    let is_closed_range = {
        let sp = points[start];
        let ep = points[end];
        (sp.0 - ep.0).abs() < 1e-6 && (sp.1 - ep.1).abs() < 1e-6
    };

    if !is_sharp && !is_closed_range && end - start == 2 {
        let p1 = points[start];
        let p2 = points[start + 1];
        let p3 = points[end];
        if let Some((center, _radius)) = fit_circle_to_3_points(p1, p2, p3) {
            let center = project_circle_center_to_bisector(p1, p3, center);
            let radius = (p1.0 - center.0).hypot(p1.1 - center.1);
            let three = [p1, p2, p3];
            let arc_dev =
                get_polyline_arc_deviation(three.as_slice(), center, radius);
            if arc_dev < tolerance {
                let mut row = create_arc_cmd(p3, center, p1);
                let pts = [(p1.0, p1.1), (p2.0, p2.1), (p3.0, p3.1)];
                let is_cw = is_arc_clockwise(pts.as_slice(), center);
                if let Command::Arc { clockwise, .. } = &mut row {
                    *clockwise = is_cw;
                }
                return vec![row];
            }
        }
    }

    if !is_sharp && !is_closed_range {
        let subset: Vec<Point3D> = points[start..=end].to_vec();
        if let Some((center, _, _)) = fit_circle_to_points(&subset) {
            let center = project_circle_center_to_bisector(
                points[start],
                points[end],
                center,
            );
            let radius =
                (points[start].0 - center.0).hypot(points[start].1 - center.1);
            let arc_dev = get_polyline_arc_deviation(&subset, center, radius);
            if arc_dev < tolerance {
                let mut row =
                    create_arc_cmd(points[end], center, points[start]);
                let is_cw = {
                    let pts2d: Vec<Point> =
                        subset.iter().map(|p| (p.0, p.1)).collect();
                    is_arc_clockwise(&pts2d, center)
                };
                if let Command::Arc { clockwise, .. } = &mut row {
                    *clockwise = is_cw;
                }
                return vec![row];
            }
        }
    }

    let split = if split_idx == start || split_idx == end {
        (start + end) / 2
    } else {
        split_idx
    };

    let left = fit_points_recursive(points, tolerance, start, split);
    let right = fit_points_recursive(points, tolerance, split, end);
    let mut result = left;
    result.extend(right);
    result
}

/// Entry point for recursive primitive fitting.
pub fn fit_points_with_primitives(
    points: &[Point3D],
    tolerance: f64,
) -> Vec<Command> {
    if points.len() < 2 {
        return vec![];
    }
    fit_points_recursive(points, tolerance, 0, points.len() - 1)
}

/// Fits line and arc primitives to a geometry data array.
pub fn fit_curves(
    data: &[Command],
    tolerance: f64,
    preserve_beziers: bool,
    preserve_arcs: bool,
    on_progress: Option<&dyn Fn(usize, usize)>,
) -> Vec<Command> {
    if data.is_empty() {
        return vec![];
    }

    let total = data.len();
    let mut new_cmds: Vec<Command> = Vec::new();
    let mut point_chain: Vec<Point3D> = Vec::new();

    let flush_chain = |chain: &mut Vec<Point3D>, cmds: &mut Vec<Command>| {
        if chain.len() > 1 {
            let simplified = simplify_polyline(chain, tolerance);
            let primitives = fit_points_with_primitives(&simplified, tolerance);
            cmds.extend(primitives);
        }
        chain.clear();
    };

    let mut last_pos = (0.0, 0.0, 0.0);

    for (i, cmd) in data.iter().enumerate() {
        let end_pos = cmd.end_point();

        if matches!(cmd, Command::Move { .. }) {
            flush_chain(&mut point_chain, &mut new_cmds);
            new_cmds.push(cmd.clone());
            last_pos = end_pos;
            continue;
        }

        if point_chain.is_empty() {
            point_chain.push(last_pos);
        }

        match cmd {
            Command::Line { .. } => {
                point_chain.push(end_pos);
            }
            Command::Arc {
                end,
                center_offset,
                clockwise,
                ..
            } => {
                if preserve_arcs {
                    flush_chain(&mut point_chain, &mut new_cmds);
                    new_cmds.push(cmd.clone());
                    point_chain.push(end_pos);
                } else {
                    let segments = linearize_arc(
                        *end,
                        *center_offset,
                        *clockwise,
                        last_pos,
                        tolerance * 0.25,
                    );
                    for (_, p_end) in segments {
                        point_chain.push(p_end);
                    }
                }
            }
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => {
                if preserve_beziers {
                    flush_chain(&mut point_chain, &mut new_cmds);
                    new_cmds.push(cmd.clone());
                    point_chain.push(end_pos);
                } else {
                    let segments = linearize_bezier_from_params(
                        *end,
                        *control1,
                        *control2,
                        last_pos,
                        tolerance * 0.25,
                    );
                    for (_, p_end) in segments {
                        point_chain.push(p_end);
                    }
                }
            }
            Command::Move { .. } => {
                flush_chain(&mut point_chain, &mut new_cmds);
                new_cmds.push(cmd.clone());
            }
        }

        last_pos = end_pos;

        if let Some(cb) = on_progress {
            cb(i + 1, total);
        }
    }

    flush_chain(&mut point_chain, &mut new_cmds);

    new_cmds
}

/// Optimises a geometry path by simplifying line chains and optionally fitting arcs.
pub fn optimize_path_from_array(
    data: &[Command],
    tolerance: f64,
    use_fit_arcs: bool,
) -> Vec<Command> {
    if data.is_empty() {
        return vec![];
    }

    let mut optimized_cmds: Vec<Command> = Vec::new();
    let mut point_chain: Vec<Point3D> = Vec::new();

    let flush_chain = |chain: &mut Vec<Point3D>, cmds: &mut Vec<Command>| {
        if chain.len() > 1 {
            if use_fit_arcs {
                let primitives = fit_points_with_primitives(chain, tolerance);
                cmds.extend(primitives);
            } else {
                let simplified = simplify_polyline(chain, tolerance);
                for p in simplified.iter().skip(1) {
                    cmds.push(create_line_cmd(*p));
                }
            }
        }
        chain.clear();
    };

    let mut last_pos = (0.0, 0.0, 0.0);

    for cmd in data {
        let end_pos = cmd.end_point();

        if matches!(cmd, Command::Line { .. }) {
            if point_chain.is_empty() {
                point_chain.push(last_pos);
            }
            point_chain.push(end_pos);
        } else {
            flush_chain(&mut point_chain, &mut optimized_cmds);
            optimized_cmds.push(cmd.clone());
            point_chain = vec![end_pos];
        }

        last_pos = end_pos;
    }

    flush_chain(&mut point_chain, &mut optimized_cmds);

    optimized_cmds
}

/// Fit arcs only (equivalent to fit_curves with preserve_beziers=false, preserve_arcs=true).
pub fn fit_arcs(data: &[Command], tolerance: f64) -> Vec<Command> {
    fit_curves(data, tolerance, false, true, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_are_points_collinear_trivial() {
        assert!(are_points_collinear(&[], 0.01));
        assert!(are_points_collinear(
            &[(0.0, 0.0, 0.0), (1.0, 1.0, 0.0)],
            0.01
        ));
    }

    #[test]
    fn test_are_points_collinear_true() {
        let points = vec![(0.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 2.0, 0.0)];
        assert!(are_points_collinear(&points, 0.01));
    }

    #[test]
    fn test_are_points_collinear_false() {
        let points = vec![(0.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 0.0, 0.0)];
        assert!(!are_points_collinear(&points, 0.01));
    }

    #[test]
    fn test_fit_circle_to_3_points_basic() {
        let p1 = (0.0, 1.0, 0.0);
        let p2 = (1.0, 0.0, 0.0);
        let p3 = (0.0, -1.0, 0.0);
        let result = fit_circle_to_3_points(p1, p2, p3);
        assert!(result.is_some());
        let (center, radius) = result.unwrap();
        assert!((center.0 - 0.0).abs() < 1e-6);
        assert!((center.1 - 0.0).abs() < 1e-6);
        assert!((radius - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_fit_circle_collinear() {
        let p1 = (0.0, 0.0, 0.0);
        let p2 = (1.0, 1.0, 0.0);
        let p3 = (2.0, 2.0, 0.0);
        assert!(fit_circle_to_3_points(p1, p2, p3).is_none());
    }

    #[test]
    fn test_get_polyline_line_deviation_trivial() {
        let points = vec![(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)];
        let (dist, idx) = get_polyline_line_deviation(&points, 0, 1);
        assert!((dist - 0.0).abs() < 1e-9);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_get_polyline_line_deviation_off_center() {
        let points = vec![(0.0, 0.0, 0.0), (5.0, 1.0, 0.0), (10.0, 0.0, 0.0)];
        let (dist, idx) = get_polyline_line_deviation(&points, 0, 2);
        assert!((dist - 1.0).abs() < 1e-6);
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_project_circle_center_to_bisector() {
        let p1 = (0.0, 0.0, 0.0);
        let p2 = (4.0, 0.0, 0.0);
        let center = (2.0, 2.0);
        let result = project_circle_center_to_bisector(p1, p2, center);
        assert!((result.0 - 2.0).abs() < 1e-9);
        assert!((result.1 - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_create_line_cmd_basic() {
        let cmd = create_line_cmd((10.0, 20.0, 5.0));
        match cmd {
            Command::Line { end } => {
                assert_eq!(end, (10.0, 20.0, 5.0));
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn test_create_arc_cmd_basic() {
        let cmd = create_arc_cmd((1.0, 0.0, 0.0), (0.0, 0.0), (0.0, 1.0, 0.0));
        match cmd {
            Command::Arc {
                end, center_offset, ..
            } => {
                assert_eq!(end, (1.0, 0.0, 0.0));
                assert!((center_offset.0 - 0.0).abs() < 1e-9);
                assert!((center_offset.1 - (-1.0)).abs() < 1e-9);
            }
            _ => panic!("expected Arc"),
        }
    }

    #[test]
    fn test_convert_arc_to_beziers() {
        let result = convert_arc_to_beziers_from_array(
            (1.0, 0.0, 0.0),
            (-1.0, 0.0, 0.0),
            (-1.0, 0.0),
            true,
        );
        assert!(!result.is_empty());
        for cmd in &result {
            assert!(matches!(cmd, Command::Bezier { .. }));
        }
    }

    #[test]
    fn test_fit_points_with_primitives_line() {
        let points = vec![(0.0, 0.0, 0.0), (5.0, 0.0, 0.0), (10.0, 0.0, 0.0)];
        let result = fit_points_with_primitives(&points, 0.1);
        assert!(!result.is_empty());
        match &result[0] {
            Command::Line { end } => {
                assert!((end.0 - 10.0).abs() < 1e-9);
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn test_fit_points_with_primitives_arc() {
        let points = vec![
            (2.0, 0.0, 0.0),
            (1.5, 1.5, 0.0),
            (0.0, 2.0, 0.0),
            (-1.5, 1.5, 0.0),
            (-2.0, 0.0, 0.0),
        ];
        let result = fit_points_with_primitives(&points, 1.0);
        assert!(!result.is_empty());
        assert!(matches!(&result[0], Command::Arc { .. }));
    }

    #[test]
    fn test_optimize_path() {
        let data: Vec<Command> = vec![
            Command::Move {
                end: (0.0, 0.0, 0.0),
            },
            Command::Line {
                end: (1.0, 0.0, 0.0),
            },
            Command::Line {
                end: (2.0, 0.0, 0.0),
            },
            Command::Line {
                end: (3.0, 0.0, 0.0),
            },
            Command::Line {
                end: (10.0, 0.0, 0.0),
            },
        ];
        let result = optimize_path_from_array(&data, 0.5, false);
        assert!(result.len() >= 2);
    }

    #[test]
    fn test_fit_curves_preserve_beziers() {
        let data: Vec<Command> = vec![
            Command::Move {
                end: (0.0, 0.0, 0.0),
            },
            Command::Bezier {
                end: (1.0, 1.0, 0.0),
                control1: (0.25, 0.5),
                control2: (0.75, 0.5),
            },
            Command::Line {
                end: (2.0, 0.0, 0.0),
            },
        ];
        let result = fit_curves(&data, 0.1, true, true, None);
        assert!(!result.is_empty());
        let has_bezier = result
            .iter()
            .any(|cmd| matches!(cmd, Command::Bezier { .. }));
        assert!(has_bezier);
    }
}
