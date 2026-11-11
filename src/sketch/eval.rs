//! Residual and Jacobian evaluation for sketch constraints.
//!
//! Each constraint contributes its residuals and, for the solver's
//! Jacobian, partial derivatives as `(point_id, residual_row, d/dx,
//! d/dy)` entries.  Entries referencing points outside the solver's
//! state vector are skipped by the assembler, mirroring fixed points
//! acting as constants.

use std::collections::HashMap;

use crate::geo::shape::arc::normalize_angle;
use crate::geo::shape::bezier::{
    get_bezier_closest_point, get_bezier_endpoint_weights,
};
use crate::geo::shape::line::{
    get_line_line_intersection, get_line_segment_closest_point,
    get_point_line_signed_distance,
};
use crate::geo::types::{Point, Point3D};

use super::constraint::ConstraintSpec;

/// Weight of the angular residual of [`ConstraintSpec::Angle`].
pub const ANGLE_WEIGHT: f64 = 10.0;

/// Central-difference step used for the Angle Jacobian.
const GRAD_FD_EPS: f64 = 1e-6;

/// Evaluates residual values and Jacobian entries against a point
/// table.
pub trait PointLookup {
    /// Returns the `(x, y)` coordinates of a point id.
    fn xy(&self, id: u64) -> (f64, f64);
}

/// Unit vector and distance of `(p2 - p1)`.
fn unit_of(dx: f64, dy: f64) -> (f64, f64) {
    let dist = dx.hypot(dy);
    if dist > 1e-9 {
        (dx / dist, dy / dist)
    } else {
        (1.0, 0.0)
    }
}

/// Unit vector of `(p2 - p1)`, defaulting to zero for degenerate
/// segments (used by equal-length style gradients).
fn unit_or_zero(dx: f64, dy: f64) -> (f64, f64) {
    let dist = dx.hypot(dy);
    if dist > 1e-9 {
        (dx / dist, dy / dist)
    } else {
        (0.0, 0.0)
    }
}

/// Jacobian entry: derivative of residual `row` w.r.t. `point_id`.
pub type JacobianEntry = (u64, usize, f64, f64);

/// Returns the residual value of a single-row constraint.
fn single_residual(spec: &ConstraintSpec, lookup: &dyn PointLookup) -> f64 {
    match spec {
        ConstraintSpec::Horizontal { p1, p2 } => {
            let (_, y1) = lookup.xy(*p1);
            let (_, y2) = lookup.xy(*p2);
            y1 - y2
        }
        ConstraintSpec::Vertical { p1, p2 } => {
            let (x1, _) = lookup.xy(*p1);
            let (x2, _) = lookup.xy(*p2);
            x1 - x2
        }
        ConstraintSpec::Distance { p1, p2, value } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            ((x2 - x1).hypot(y2 - y1)) - value
        }
        ConstraintSpec::AspectRatio {
            p1,
            p2,
            p3,
            p4,
            ratio,
        } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let dist1 = (x2 - x1).hypot(y2 - y1);
            let (x3, y3) = lookup.xy(*p3);
            let (x4, y4) = lookup.xy(*p4);
            let dist2 = (x4 - x3).hypot(y4 - y3);
            if dist2 < 1e-9 {
                dist1
            } else {
                dist1 - dist2 * ratio
            }
        }
        ConstraintSpec::Angle { .. } => angle_error(spec, lookup),
        ConstraintSpec::Collinear { p1, p2, p3 } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let (x3, y3) = lookup.xy(*p3);
            (x2 - x1) * (y3 - y1) - (y2 - y1) * (x3 - x1)
        }
        ConstraintSpec::EqualDistance { p1, p2, p3, p4 } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let dist1 = (x2 - x1).hypot(y2 - y1);
            let (x3, y3) = lookup.xy(*p3);
            let (x4, y4) = lookup.xy(*p4);
            let dist2 = (x4 - x3).hypot(y4 - y3);
            dist1 - dist2
        }
        ConstraintSpec::Radius {
            center,
            radius_point,
            value,
        }
        | ConstraintSpec::Diameter {
            center,
            radius_point,
            value,
        } => {
            let scale = match spec {
                ConstraintSpec::Diameter { .. } => 2.0,
                _ => 1.0,
            };
            let (cx, cy) = lookup.xy(*center);
            let (px, py) = lookup.xy(*radius_point);
            scale * ((px - cx).hypot(py - cy)) - value
        }
        ConstraintSpec::PerpLineLine { a1, a2, b1, b2 } => {
            let (ax1, ay1) = lookup.xy(*a1);
            let (ax2, ay2) = lookup.xy(*a2);
            let (bx1, by1) = lookup.xy(*b1);
            let (bx2, by2) = lookup.xy(*b2);
            let dx1 = ax2 - ax1;
            let dy1 = ay2 - ay1;
            let dx2 = bx2 - bx1;
            let dy2 = by2 - by1;
            dx1 * dx2 + dy1 * dy2
        }
        ConstraintSpec::PerpLineCenter { l1, l2, center } => {
            let (x1, y1) = lookup.xy(*l1);
            let (x2, y2) = lookup.xy(*l2);
            let (cx, cy) = lookup.xy(*center);
            (x2 - x1) * (cy - y1) - (cx - x1) * (y2 - y1)
        }
        ConstraintSpec::PerpCircles {
            c1,
            r1_point,
            c2,
            r2_point,
        } => {
            let (c1x, c1y) = lookup.xy(*c1);
            let (p1x, p1y) = lookup.xy(*r1_point);
            let (c2x, c2y) = lookup.xy(*c2);
            let (p2x, p2y) = lookup.xy(*r2_point);
            let r1_sq = (p1x - c1x).hypot(p1y - c1y).powi(2);
            let r2_sq = (p2x - c2x).hypot(p2y - c2y).powi(2);
            let dist_sq = (c2x - c1x).powi(2) + (c2y - c1y).powi(2);
            r1_sq + r2_sq - dist_sq
        }
        ConstraintSpec::Tangent {
            l1,
            l2,
            center,
            radius_point,
        } => {
            let (cx, cy) = lookup.xy(*center);
            let (px, py) = lookup.xy(*radius_point);
            let signed = get_point_line_signed_distance(
                Point::new(cx, cy),
                Point::new(lookup.xy(*l1).0, lookup.xy(*l1).1),
                Point::new(lookup.xy(*l2).0, lookup.xy(*l2).1),
            );
            signed.abs() - ((px - cx).hypot(py - cy))
        }
        ConstraintSpec::PointOnLine { point, l1, l2 } => {
            get_point_line_signed_distance(
                Point::new(lookup.xy(*point).0, lookup.xy(*point).1),
                Point::new(lookup.xy(*l1).0, lookup.xy(*l1).1),
                Point::new(lookup.xy(*l2).0, lookup.xy(*l2).1),
            )
        }
        ConstraintSpec::PointOnRadiusCurve {
            point,
            center,
            radius_point,
        } => {
            let (px, py) = lookup.xy(*point);
            let (cx, cy) = lookup.xy(*center);
            let (rx, ry) = lookup.xy(*radius_point);
            let radius = (rx - cx).hypot(ry - cy);
            ((px - cx).hypot(py - cy)) - radius
        }
        ConstraintSpec::PointOnBezier {
            point,
            start,
            end,
            cp1_offset,
            cp2_offset,
            is_line,
        } => {
            let (px, py) = lookup.xy(*point);
            let (sx, sy) = lookup.xy(*start);
            let (ex, ey) = lookup.xy(*end);
            if *is_line {
                let (_, _, dist_sq) = get_line_segment_closest_point(
                    Point::new(sx, sy),
                    Point::new(ex, ey),
                    px,
                    py,
                );
                dist_sq.sqrt()
            } else {
                let cp1x = sx + cp1_offset[0];
                let cp1y = sy + cp1_offset[1];
                let cp2x = ex + cp2_offset[0];
                let cp2y = ey + cp2_offset[1];
                match crate::geo::shape::bezier::get_bezier_closest_point(
                    Point3D::new(ex, ey, 0.0),
                    Point3D::new(cp1x, cp1y, 0.0),
                    Point3D::new(cp2x, cp2y, 0.0),
                    Point3D::new(sx, sy, 0.0),
                    px,
                    py,
                ) {
                    Some((_, _, dist_sq)) => dist_sq.sqrt(),
                    None => 0.0,
                }
            }
        }
        ConstraintSpec::EqualLength { .. } => 0.0,
        _ => 0.0,
    }
}

/// Error of the angle constraint between two lines, in radians
/// (weighted).  Returns 0.0 for parallel lines or degenerate rays.
fn angle_error(spec: &ConstraintSpec, lookup: &dyn PointLookup) -> f64 {
    let ConstraintSpec::Angle {
        p1,
        p2,
        p3,
        p4,
        e1_far,
        e2_far,
        value_deg,
    } = spec
    else {
        return 0.0;
    };
    let lookup_xy = |id: u64| lookup.xy(id);
    let intersection = get_line_line_intersection(
        Point::from(lookup_xy(*p1)),
        Point::from(lookup_xy(*p2)),
        Point::from(lookup_xy(*p3)),
        Point::from(lookup_xy(*p4)),
    );
    let (ix, iy) = match intersection {
        Some(point) => (point.x, point.y),
        None => return 0.0,
    };

    let far1 = match e1_far {
        Some(id) if *id == *p1 => lookup_xy(*p1),
        Some(id) if *id == *p2 => lookup_xy(*p2),
        _ => {
            let a = lookup_xy(*p1);
            let b = lookup_xy(*p2);
            let d1 = (a.0 - ix).powi(2) + (a.1 - iy).powi(2);
            let d2 = (b.0 - ix).powi(2) + (b.1 - iy).powi(2);
            if d1 > d2 {
                a
            } else {
                b
            }
        }
    };
    let far2 = match e2_far {
        Some(id) if *id == *p3 => lookup_xy(*p3),
        Some(id) if *id == *p4 => lookup_xy(*p4),
        _ => {
            let a = lookup_xy(*p3);
            let b = lookup_xy(*p4);
            let d1 = (a.0 - ix).powi(2) + (a.1 - iy).powi(2);
            let d2 = (b.0 - ix).powi(2) + (b.1 - iy).powi(2);
            if d1 > d2 {
                a
            } else {
                b
            }
        }
    };

    let dx1 = far1.0 - ix;
    let dy1 = far1.1 - iy;
    let dx2 = far2.0 - ix;
    let dy2 = far2.1 - iy;
    let len1_sq = dx1 * dx1 + dy1 * dy1;
    let len2_sq = dx2 * dx2 + dy2 * dy2;
    if len1_sq < 1e-12 || len2_sq < 1e-12 {
        return 0.0;
    }

    let anchor_dir = dy1.atan2(dx1);
    let other_dir = dy2.atan2(dx2);
    let current = normalize_angle(anchor_dir - other_dir);
    let target = (*value_deg).to_radians();
    let mut diff = current - target;
    while diff > std::f64::consts::PI {
        diff -= 2.0 * std::f64::consts::PI;
    }
    while diff <= -std::f64::consts::PI {
        diff += 2.0 * std::f64::consts::PI;
    }
    diff * ANGLE_WEIGHT
}

/// Appends the Jacobian entries of one multi-row constraint.
fn multi_residuals(
    spec: &ConstraintSpec,
    lookup: &dyn PointLookup,
    out: &mut [f64],
) {
    match spec {
        ConstraintSpec::Coincident { p1, p2 } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            out[0] = x1 - x2;
            out[1] = y1 - y2;
        }
        ConstraintSpec::Parallelogram {
            p_origin,
            p_width,
            p_height,
            p4,
        } => {
            let (ox, oy) = lookup.xy(*p_origin);
            let (wx, wy) = lookup.xy(*p_width);
            let (hx, hy) = lookup.xy(*p_height);
            let (fx, fy) = lookup.xy(*p4);
            out[0] = (wx - ox) - (fx - hx);
            out[1] = (wy - oy) - (fy - hy);
        }
        ConstraintSpec::Rotational {
            center,
            p1,
            p2,
            angle_rad,
        } => {
            let (cx, cy) = lookup.xy(*center);
            let (sx, sy) = lookup.xy(*p1);
            let (tx, ty) = lookup.xy(*p2);
            let ca = angle_rad.cos();
            let sa = angle_rad.sin();
            let dx = sx - cx;
            let dy = sy - cy;
            out[0] = tx - (cx + ca * dx - sa * dy);
            out[1] = ty - (cy + sa * dx + ca * dy);
        }
        ConstraintSpec::SymmetryPoint { p1, p2, center } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let (cx, cy) = lookup.xy(*center);
            out[0] = (x1 + x2) - 2.0 * cx;
            out[1] = (y1 + y2) - 2.0 * cy;
        }
        ConstraintSpec::SymmetryLine { p1, p2, l1, l2 } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let (lx1, ly1) = lookup.xy(*l1);
            let (lx2, ly2) = lookup.xy(*l2);
            let dx_l = lx2 - lx1;
            let dy_l = ly2 - ly1;
            let dx_p = x2 - x1;
            let dy_p = y2 - y1;
            out[0] = dx_p * dx_l + dy_p * dy_l;
            let mx = (x1 + x2) * 0.5;
            let my = (y1 + y2) * 0.5;
            out[1] = (mx - lx1) * dy_l - (my - ly1) * dx_l;
        }
        ConstraintSpec::EqualLength { pairs } => {
            for (row, pair) in pairs.iter().enumerate() {
                let (ax, ay) = lookup.xy(pair[0]);
                let (bx, by) = lookup.xy(pair[1]);
                let base = (bx - ax).hypot(by - ay);
                let (cx, cy) = lookup.xy(pair[2]);
                let (dx, dy) = lookup.xy(pair[3]);
                let other = (dx - cx).hypot(dy - cy);
                out[row] = other - base;
            }
        }
        ConstraintSpec::Drag {
            point,
            target_x,
            target_y,
            weight,
        } => {
            let (x, y) = lookup.xy(*point);
            out[0] = (x - target_x) * weight;
            out[1] = (y - target_y) * weight;
        }
        _ => {
            if out.len() == 1 {
                out[0] = single_residual(spec, lookup);
            }
        }
    }
}

/// Returns all residual values of the given constraints, in order.
pub fn compute_residuals(
    specs: &[&ConstraintSpec],
    lookup: &dyn PointLookup,
) -> Vec<f64> {
    let mut residuals = Vec::new();
    for spec in specs {
        let count = spec.residual_count();
        if count == 0 {
            continue;
        }
        let start = residuals.len();
        residuals.resize(start + count, 0.0);
        multi_residuals(spec, lookup, &mut residuals[start..]);
    }
    if residuals.is_empty() {
        residuals.push(0.0);
    }
    residuals
}

/// Appends the Jacobian entries of a single-row constraint.
fn single_jacobian(
    spec: &ConstraintSpec,
    lookup: &dyn PointLookup,
    entries: &mut Vec<JacobianEntry>,
) {
    let add = |entries: &mut Vec<JacobianEntry>, pid: u64, gx: f64, gy: f64| {
        entries.push((pid, 0, gx, gy));
    };
    match spec {
        ConstraintSpec::Horizontal { p1, p2 } => {
            add(entries, *p1, 0.0, 1.0);
            add(entries, *p2, 0.0, -1.0);
        }
        ConstraintSpec::Vertical { p1, p2 } => {
            add(entries, *p1, 1.0, 0.0);
            add(entries, *p2, -1.0, 0.0);
        }
        ConstraintSpec::Distance { p1, p2, .. } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let (ux, uy) = unit_of(x2 - x1, y2 - y1);
            add(entries, *p1, -ux, -uy);
            add(entries, *p2, ux, uy);
        }
        ConstraintSpec::AspectRatio {
            p1,
            p2,
            p3,
            p4,
            ratio,
        } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let dx1 = x2 - x1;
            let dy1 = y2 - y1;
            let dist1 = dx1.hypot(dy1);
            let (x3, y3) = lookup.xy(*p3);
            let (x4, y4) = lookup.xy(*p4);
            let dx2 = x4 - x3;
            let dy2 = y4 - y3;
            let dist2 = dx2.hypot(dy2);
            if dist1 > 1e-9 {
                let (ux, uy) = (dx1 / dist1, dy1 / dist1);
                add(entries, *p1, -ux, -uy);
                add(entries, *p2, ux, uy);
            }
            if dist2 > 1e-9 {
                let (ux, uy) = (dx2 / dist2, dy2 / dist2);
                add(entries, *p3, ratio * ux, ratio * uy);
                add(entries, *p4, -ratio * ux, -ratio * uy);
            }
        }
        ConstraintSpec::Collinear { p1, p2, p3 } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let (x3, y3) = lookup.xy(*p3);
            add(entries, *p1, y2 - y3, x3 - x2);
            add(entries, *p2, y3 - y1, -(x3 - x1));
            add(entries, *p3, -(y2 - y1), x2 - x1);
        }
        ConstraintSpec::EqualDistance { p1, p2, p3, p4 } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let dx1 = x2 - x1;
            let dy1 = y2 - y1;
            let dist1 = dx1.hypot(dy1);
            let (x3, y3) = lookup.xy(*p3);
            let (x4, y4) = lookup.xy(*p4);
            let dx2 = x4 - x3;
            let dy2 = y4 - y3;
            let dist2 = dx2.hypot(dy2);
            if dist1 > 1e-9 {
                let (ux, uy) = (dx1 / dist1, dy1 / dist1);
                add(entries, *p1, -ux, -uy);
                add(entries, *p2, ux, uy);
            }
            if dist2 > 1e-9 {
                let (ux, uy) = (dx2 / dist2, dy2 / dist2);
                add(entries, *p3, ux, uy);
                add(entries, *p4, -ux, -uy);
            }
        }
        ConstraintSpec::Radius {
            center,
            radius_point,
            ..
        } => {
            let (cx, cy) = lookup.xy(*center);
            let (px, py) = lookup.xy(*radius_point);
            let (ux, uy) = unit_of(px - cx, py - cy);
            add(entries, *radius_point, ux, uy);
            add(entries, *center, -ux, -uy);
        }
        ConstraintSpec::Diameter {
            center,
            radius_point,
            ..
        } => {
            let (cx, cy) = lookup.xy(*center);
            let (px, py) = lookup.xy(*radius_point);
            let (ux, uy) = unit_of(px - cx, py - cy);
            add(entries, *radius_point, 2.0 * ux, 2.0 * uy);
            add(entries, *center, -2.0 * ux, -2.0 * uy);
        }
        ConstraintSpec::PerpLineLine { a1, a2, b1, b2 } => {
            let (ax1, ay1) = lookup.xy(*a1);
            let (ax2, ay2) = lookup.xy(*a2);
            let (bx1, by1) = lookup.xy(*b1);
            let (bx2, by2) = lookup.xy(*b2);
            let dx1 = ax2 - ax1;
            let dy1 = ay2 - ay1;
            let dx2 = bx2 - bx1;
            let dy2 = by2 - by1;
            add(entries, *a1, -dx2, -dy2);
            add(entries, *a2, dx2, dy2);
            add(entries, *b1, -dx1, -dy1);
            add(entries, *b2, dx1, dy1);
        }
        ConstraintSpec::PerpLineCenter { l1, l2, center } => {
            let (x1, y1) = lookup.xy(*l1);
            let (x2, y2) = lookup.xy(*l2);
            let (cx, cy) = lookup.xy(*center);
            let dx = x2 - x1;
            let dy = y2 - y1;
            add(entries, *center, -dy, dx);
            add(entries, *l1, dy - (cy - y1), -dx + (cx - x1));
            add(entries, *l2, cy - y1, -(cx - x1));
        }
        ConstraintSpec::PerpCircles {
            c1,
            r1_point,
            c2,
            r2_point,
        } => {
            let (c1x, c1y) = lookup.xy(*c1);
            let (p1x, p1y) = lookup.xy(*r1_point);
            let (c2x, c2y) = lookup.xy(*c2);
            let (p2x, p2y) = lookup.xy(*r2_point);
            let d_dist_x = 2.0 * (c2x - c1x);
            let d_dist_y = 2.0 * (c2y - c1y);
            add(entries, *r1_point, 2.0 * (p1x - c1x), 2.0 * (p1y - c1y));
            add(
                entries,
                *c1,
                d_dist_x - 2.0 * (p1x - c1x),
                d_dist_y - 2.0 * (p1y - c1y),
            );
            add(entries, *r2_point, 2.0 * (p2x - c2x), 2.0 * (p2y - c2y));
            add(
                entries,
                *c2,
                -d_dist_x - 2.0 * (p2x - c2x),
                -d_dist_y - 2.0 * (p2y - c2y),
            );
        }
        ConstraintSpec::Tangent {
            l1,
            l2,
            center,
            radius_point,
        } => {
            let (cx, cy) = lookup.xy(*center);
            let (px, py) = lookup.xy(*radius_point);

            let rad_dx = px - cx;
            let rad_dy = py - cy;
            let radius = rad_dx.hypot(rad_dy);
            let (rad_ux, rad_uy) = if radius > 1e-9 {
                (rad_dx / radius, rad_dy / radius)
            } else {
                (0.0, 0.0)
            };
            add(entries, *radius_point, -rad_ux, -rad_uy);

            let (lx1, ly1) = lookup.xy(*l1);
            let (lx2, ly2) = lookup.xy(*l2);
            let line_dx = lx2 - lx1;
            let line_dy = ly2 - ly1;
            let len_sq = line_dx * line_dx + line_dy * line_dy;
            if len_sq < 1e-18 {
                let dist_c_l1 = (cx - lx1).hypot(cy - ly1);
                if dist_c_l1 < 1e-9 {
                    return;
                }
                let ux = (cx - lx1) / dist_c_l1;
                let uy = (cy - ly1) / dist_c_l1;
                add(entries, *center, ux, uy);
                add(entries, *l1, -ux, -uy);
                return;
            }

            let length = len_sq.sqrt();
            let inv_len = 1.0 / length;
            let cross = line_dx * (cy - ly1) - line_dy * (cx - lx1);
            let sign = cross.copysign(1.0);

            let nx = -line_dy * inv_len;
            let ny = line_dx * inv_len;
            let pol_err = cross * inv_len;
            let inv_len_sq = inv_len * inv_len;

            let grad_pol_c_x = nx;
            let grad_pol_c_y = ny;
            let grad_pol_l2_x =
                (cy - ly1) * inv_len - pol_err * line_dx * inv_len_sq;
            let grad_pol_l2_y =
                -(cx - lx1) * inv_len - pol_err * line_dy * inv_len_sq;
            let grad_pol_l1_x = -grad_pol_c_x - grad_pol_l2_x;
            let grad_pol_l1_y = -grad_pol_c_y - grad_pol_l2_y;

            add(
                entries,
                *center,
                rad_ux + sign * grad_pol_c_x,
                rad_uy + sign * grad_pol_c_y,
            );
            add(entries, *l1, sign * grad_pol_l1_x, sign * grad_pol_l1_y);
            add(entries, *l2, sign * grad_pol_l2_x, sign * grad_pol_l2_y);
        }
        ConstraintSpec::PointOnLine { point, l1, l2 } => {
            let (px, py) = lookup.xy(*point);
            let (lx1, ly1) = lookup.xy(*l1);
            let (lx2, ly2) = lookup.xy(*l2);
            let dx = lx2 - lx1;
            let dy = ly2 - ly1;
            let len_sq = dx * dx + dy * dy;
            if len_sq < 1e-18 {
                let dpx = px - lx1;
                let dpy = py - ly1;
                let dist = dpx.hypot(dpy);
                if dist < 1e-9 {
                    return;
                }
                let ux = dpx / dist;
                let uy = dpy / dist;
                add(entries, *point, ux, uy);
                add(entries, *l1, -ux, -uy);
                return;
            }
            let length = len_sq.sqrt();
            let inv_len = 1.0 / length;
            let inv_len_sq = inv_len * inv_len;
            let nx = -dy * inv_len;
            let ny = dx * inv_len;
            add(entries, *point, nx, ny);
            let err = get_point_line_signed_distance(
                Point::new(px, py),
                Point::new(lx1, ly1),
                Point::new(lx2, ly2),
            );
            let grad_l2_x = (py - ly1) * inv_len - err * dx * inv_len_sq;
            let grad_l2_y = -(px - lx1) * inv_len - err * dy * inv_len_sq;
            add(entries, *l2, grad_l2_x, grad_l2_y);
            add(entries, *l1, -nx - grad_l2_x, -ny - grad_l2_y);
        }
        ConstraintSpec::PointOnRadiusCurve {
            point,
            center,
            radius_point,
        } => {
            let (px, py) = lookup.xy(*point);
            let (cx, cy) = lookup.xy(*center);
            let dist_pc = (px - cx).hypot(py - cy);
            let (ux, uy) = if dist_pc > 1e-9 {
                ((px - cx) / dist_pc, (py - cy) / dist_pc)
            } else {
                (0.0, 0.0)
            };
            add(entries, *point, ux, uy);

            let (rx, ry) = lookup.xy(*radius_point);
            let radius = (rx - cx).hypot(ry - cy);
            let (rux, ruy) = if radius > 1e-9 {
                ((rx - cx) / radius, (ry - cy) / radius)
            } else {
                (0.0, 0.0)
            };
            add(entries, *center, rux - ux, ruy - uy);
            add(entries, *radius_point, -rux, -ruy);
        }
        ConstraintSpec::PointOnBezier {
            point,
            start,
            end,
            cp1_offset,
            cp2_offset,
            ..
        } => {
            let (px, py) = lookup.xy(*point);
            let (sx, sy) = lookup.xy(*start);
            let (ex, ey) = lookup.xy(*end);
            let cp1x = sx + cp1_offset[0];
            let cp1y = sy + cp1_offset[1];
            let cp2x = ex + cp2_offset[0];
            let cp2y = ey + cp2_offset[1];
            let closest = get_bezier_closest_point(
                Point3D::new(ex, ey, 0.0),
                Point3D::new(cp1x, cp1y, 0.0),
                Point3D::new(cp2x, cp2y, 0.0),
                Point3D::new(sx, sy, 0.0),
                px,
                py,
            );
            let (t, bx, by) = match closest {
                Some((t, closest_point, dist_sq)) => {
                    if dist_sq.sqrt() < 1e-9 {
                        return;
                    }
                    (t, closest_point.x, closest_point.y)
                }
                None => return,
            };
            let dist = (bx - px).hypot(by - py);
            if dist < 1e-9 {
                return;
            }
            let ux = (bx - px) / dist;
            let uy = (by - py) / dist;

            // The distance to the closest curve point only depends on
            // the curve through the offset direction at the solved
            // parameter (envelope theorem), so the plain endpoint
            // weights of the Bernstein basis suffice here.
            let (w_start, w_end) = get_bezier_endpoint_weights(t);
            add(entries, *point, -ux, -uy);
            add(entries, *start, w_start * ux, w_start * uy);
            add(entries, *end, w_end * ux, w_end * uy);
        }
        _ => {}
    }
}

/// Appends the Jacobian entries of a multi-row constraint.
fn multi_jacobian(
    spec: &ConstraintSpec,
    lookup: &dyn PointLookup,
    entries: &mut Vec<JacobianEntry>,
) {
    match spec {
        ConstraintSpec::Coincident { p1, p2 } => {
            entries.push((*p1, 0, 1.0, 0.0));
            entries.push((*p1, 1, 0.0, 1.0));
            entries.push((*p2, 0, -1.0, 0.0));
            entries.push((*p2, 1, 0.0, -1.0));
        }
        ConstraintSpec::Parallelogram {
            p_origin,
            p_width,
            p_height,
            p4,
        } => {
            entries.push((*p_origin, 0, -1.0, 0.0));
            entries.push((*p_origin, 1, 0.0, -1.0));
            entries.push((*p_width, 0, 1.0, 0.0));
            entries.push((*p_width, 1, 0.0, 1.0));
            entries.push((*p_height, 0, 1.0, 0.0));
            entries.push((*p_height, 1, 0.0, 1.0));
            entries.push((*p4, 0, -1.0, 0.0));
            entries.push((*p4, 1, 0.0, -1.0));
        }
        ConstraintSpec::Rotational {
            center,
            p1,
            p2,
            angle_rad,
        } => {
            let ca = angle_rad.cos();
            let sa = angle_rad.sin();
            entries.push((*p2, 0, 1.0, 0.0));
            entries.push((*p2, 1, 0.0, 1.0));
            entries.push((*p1, 0, -ca, sa));
            entries.push((*p1, 1, -sa, -ca));
            entries.push((*center, 0, -1.0 + ca, -sa));
            entries.push((*center, 1, sa, -1.0 + ca));
        }
        ConstraintSpec::SymmetryPoint { p1, p2, center } => {
            entries.push((*p1, 0, 1.0, 0.0));
            entries.push((*p1, 1, 0.0, 1.0));
            entries.push((*p2, 0, 1.0, 0.0));
            entries.push((*p2, 1, 0.0, 1.0));
            entries.push((*center, 0, -2.0, 0.0));
            entries.push((*center, 1, 0.0, -2.0));
        }
        ConstraintSpec::SymmetryLine { p1, p2, l1, l2 } => {
            let (x1, y1) = lookup.xy(*p1);
            let (x2, y2) = lookup.xy(*p2);
            let (lx1, ly1) = lookup.xy(*l1);
            let (lx2, ly2) = lookup.xy(*l2);
            let dxl = lx2 - lx1;
            let dyl = ly2 - ly1;
            let dxp = x2 - x1;
            let dyp = y2 - y1;
            let mx = (x1 + x2) * 0.5;
            let my = (y1 + y2) * 0.5;

            entries.push((*p1, 0, -dxl, -dyl));
            entries.push((*p2, 0, dxl, dyl));
            entries.push((*l1, 0, -dxp, -dyp));
            entries.push((*l2, 0, dxp, dyp));

            entries.push((*p1, 1, 0.5 * dyl, -0.5 * dxl));
            entries.push((*p2, 1, 0.5 * dyl, -0.5 * dxl));
            entries.push((*l1, 1, my - ly1 - dyl, dxl - (mx - lx1)));
            entries.push((*l2, 1, -(my - ly1), mx - lx1));
        }
        ConstraintSpec::EqualLength { pairs } => {
            for (row, pair) in pairs.iter().enumerate() {
                let (bax, bay) = lookup.xy(pair[0]);
                let (bbx, bby) = lookup.xy(pair[1]);
                let (bux, buy) = unit_or_zero(bbx - bax, bby - bay);
                let (eax, eay) = lookup.xy(pair[2]);
                let (ebx, eby) = lookup.xy(pair[3]);
                let (eux, euy) = unit_or_zero(ebx - eax, eby - eay);
                entries.push((pair[0], row, bux, buy));
                entries.push((pair[1], row, -bux, -buy));
                entries.push((pair[2], row, -eux, -euy));
                entries.push((pair[3], row, eux, euy));
            }
        }
        ConstraintSpec::Drag { point, weight, .. } => {
            entries.push((*point, 0, *weight, 0.0));
            entries.push((*point, 1, 0.0, *weight));
        }
        _ => {
            single_jacobian(spec, lookup, entries);
        }
    }
}

/// mutable point-table over a flat coordinate array, addressed by
/// point id.
pub struct Evaluator {
    slot_of: HashMap<u64, usize>,
    pub(crate) coords: Vec<f64>,
}

impl Evaluator {
    /// Builds an evaluator over `coords` (two floats per point,
    /// indexed through `slot_of`).
    pub fn new(slot_of: HashMap<u64, usize>, coords: Vec<f64>) -> Self {
        Evaluator { slot_of, coords }
    }

    /// Overwrites the coordinates of the given slots from the flat
    /// state vector (two floats per slot).
    pub fn set_state(&mut self, state: &[f64], state_slots: &[usize]) {
        for (i, &slot) in state_slots.iter().enumerate() {
            self.coords[2 * slot] = state[2 * i];
            self.coords[2 * slot + 1] = state[2 * i + 1];
        }
    }

    fn as_lookup(&self) -> &dyn PointLookup {
        self
    }

    /// Returns all residual values of the given constraints.
    pub fn residuals(&self, specs: &[&ConstraintSpec]) -> Vec<f64> {
        compute_residuals(specs, self.as_lookup())
    }

    /// Residual of a single constraint (used by the finite-difference
    /// Angle Jacobian).
    fn single_constraint_residuals(&self, spec: &ConstraintSpec) -> Vec<f64> {
        let count = spec.residual_count();
        let mut out = vec![0.0; count.max(1)];
        multi_residuals(spec, self.as_lookup(), &mut out[..count]);
        out
    }

    /// Builds the dense Jacobian (row-major, `rows * n_vars`) of the
    /// given constraints at the current coordinates.  Entries for
    /// points outside `state_col_of` are skipped; `n_vars` is twice
    /// the number of state points.
    pub fn jacobian(
        &mut self,
        specs: &[&ConstraintSpec],
        n_vars: usize,
        state_col_of: &HashMap<u64, usize>,
    ) -> Vec<f64> {
        let mut rows = 0;
        for spec in specs {
            rows += spec.residual_count();
        }
        if rows == 0 {
            rows = 1;
        }
        let mut jac = vec![0.0; rows * n_vars];
        let mut row_offset = 0;
        for spec in specs {
            let count = spec.residual_count();
            if count == 0 {
                continue;
            }
            if let ConstraintSpec::Angle { p1, p2, p3, p4, .. } = spec {
                let pids = [*p1, *p2, *p3, *p4];
                for &pid in &pids {
                    let row = row_offset;
                    for axis in 0..2 {
                        let slot = self.slot_of[&pid];
                        let idx = 2 * slot + axis;
                        let orig = self.coords[idx];
                        self.coords[idx] = orig + GRAD_FD_EPS;
                        let err_plus =
                            self.single_constraint_residuals(spec)[0];
                        self.coords[idx] = orig - GRAD_FD_EPS;
                        let err_minus =
                            self.single_constraint_residuals(spec)[0];
                        self.coords[idx] = orig;
                        let g = (err_plus - err_minus) / (2.0 * GRAD_FD_EPS);
                        if let Some(&col) = state_col_of.get(&pid) {
                            jac[row * n_vars + col + axis] += g;
                        }
                    }
                }
            } else {
                let mut entries = Vec::new();
                multi_jacobian(spec, self.as_lookup(), &mut entries);
                for (pid, row, gx, gy) in entries {
                    if let Some(&col) = state_col_of.get(&pid) {
                        let r = row_offset + row;
                        jac[r * n_vars + col] += gx;
                        jac[r * n_vars + col + 1] += gy;
                    }
                }
            }
            row_offset += count;
        }
        jac
    }
}

impl PointLookup for Evaluator {
    fn xy(&self, id: u64) -> (f64, f64) {
        let slot = self.slot_of[&id];
        (self.coords[2 * slot], self.coords[2 * slot + 1])
    }
}

impl PointLookup for &Evaluator {
    fn xy(&self, id: u64) -> (f64, f64) {
        let slot = self.slot_of[&id];
        (self.coords[2 * slot], self.coords[2 * slot + 1])
    }
}

/// Conflicting-constraint helper: max absolute residual per
/// constraint (in list order).  Empty for constraints without
/// residuals.
pub fn compute_max_residuals(
    specs: &[&ConstraintSpec],
    lookup: &dyn PointLookup,
) -> Vec<f64> {
    specs
        .iter()
        .map(|spec| {
            let count = spec.residual_count();
            if count == 0 {
                return 0.0;
            }
            let mut out = vec![0.0; count];
            multi_residuals(spec, lookup, &mut out);
            out.iter().fold(0.0f64, |acc, v| acc.max(v.abs()))
        })
        .collect()
}
