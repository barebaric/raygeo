//! Point-on-curve constraint: a point on a cubic Bezier curve.

use crate::geo::shape::bezier::get_bezier_closest_point;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::types::{Point, Point3D};
use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_point_marker, CairoLike, DrawContext, DrawStyle,
};
use crate::sketch::registry::EntityRegistry;

/// Minimum distance below which the closest-point direction is
/// considered degenerate and the gradient empty.
const MIN_DISTANCE: f64 = 1e-9;

/// Weights of the two curve endpoints in the cubic Bernstein basis:
/// `(B0(t) + B1(t), B2(t) + B3(t))`.
///
/// The distance to the closest curve point only depends on the curve
/// through the offset direction at the solved parameter (envelope
/// theorem), so these plain weights turn endpoint translations into
/// Jacobian entries when the control points are stored as endpoint
/// offsets.
fn get_bezier_endpoint_weights(t: f64) -> (f64, f64) {
    let mt = 1.0 - t;
    let w_start = mt * mt * mt + 3.0 * mt * mt * t;
    let w_end = 3.0 * mt * t * t + t * t * t;
    (w_start, w_end)
}

/// Enforces a point lies on a cubic Bezier curve.  The control
/// points are stored as offsets from their owning endpoints; curves
/// without control points use the straight-segment residual
/// (`is_line`).
#[derive(Clone, Debug, PartialEq)]
pub struct PointOnCurveConstraint {
    /// The constrained point.
    pub point: u64,
    /// Curve start point.
    pub start: u64,
    /// Curve end point.
    pub end: u64,
    /// First control point as offset from the start point.
    pub cp1_offset: [f64; 2],
    /// Second control point as offset from the end point.
    pub cp2_offset: [f64; 2],
    /// Whether the curve degenerates to a straight segment.
    pub is_line: bool,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl PointOnCurveConstraint {
    /// Creates a point-on-curve constraint.
    pub fn new(
        point: u64,
        start: u64,
        end: u64,
        cp1_offset: [f64; 2],
        cp2_offset: [f64; 2],
        is_line: bool,
    ) -> Self {
        PointOnCurveConstraint {
            point,
            start,
            end,
            cp1_offset,
            cp2_offset,
            is_line,
            user_visible: true,
        }
    }

    /// Absolute distance from the constrained point to the curve.
    fn distance(&self, reg: &dyn PointContext) -> f64 {
        let (px, py) = reg.xy(self.point);
        let (sx, sy) = reg.xy(self.start);
        let (ex, ey) = reg.xy(self.end);
        if self.is_line {
            let (_, _, dist_sq) = get_line_segment_closest_point(
                Point::new(sx, sy),
                Point::new(ex, ey),
                px,
                py,
            );
            return dist_sq.sqrt();
        }
        let cp1x = sx + self.cp1_offset[0];
        let cp1y = sy + self.cp1_offset[1];
        let cp2x = ex + self.cp2_offset[0];
        let cp2y = ey + self.cp2_offset[1];
        match get_bezier_closest_point(
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

impl SketchConstraint for PointOnCurveConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        PointOnCurveConstraint::to_json(self)
    }

    fn referenced_points(&self) -> Vec<u64> {
        vec![self.point, self.start, self.end]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        vec![self.distance(reg)]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (px, py) = reg.xy(self.point);
        let (sx, sy) = reg.xy(self.start);
        let (ex, ey) = reg.xy(self.end);
        let cp1x = sx + self.cp1_offset[0];
        let cp1y = sy + self.cp1_offset[1];
        let cp2x = ex + self.cp2_offset[0];
        let cp2y = ey + self.cp2_offset[1];

        let closest = if self.is_line {
            let (_, closest, dist_sq) = get_line_segment_closest_point(
                Point::new(sx, sy),
                Point::new(ex, ey),
                px,
                py,
            );
            if dist_sq.sqrt() < MIN_DISTANCE {
                return Vec::new();
            }
            (0.0, closest.x, closest.y)
        } else {
            match get_bezier_closest_point(
                Point3D::new(ex, ey, 0.0),
                Point3D::new(cp1x, cp1y, 0.0),
                Point3D::new(cp2x, cp2y, 0.0),
                Point3D::new(sx, sy, 0.0),
                px,
                py,
            ) {
                Some((t, closest_point, dist_sq)) => {
                    if dist_sq.sqrt() < MIN_DISTANCE {
                        return Vec::new();
                    }
                    (t, closest_point.x, closest_point.y)
                }
                None => return Vec::new(),
            }
        };

        let (t, bx, by) = closest;
        let dist = (bx - px).hypot(by - py);
        if dist < MIN_DISTANCE {
            return Vec::new();
        }
        let ux = (bx - px) / dist;
        let uy = (by - py) / dist;

        let (w_start, w_end) = get_bezier_endpoint_weights(t);
        vec![
            (self.point, 0, -ux, -uy),
            (self.start, 0, w_start * ux, w_start * uy),
            (self.end, 0, w_end * ux, w_end * uy),
        ]
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let Some(pt) = model.get_point(self.point) else {
            return false;
        };
        let s_pt = to_screen((pt.x, pt.y));
        (mx - s_pt.0).hypot(my - s_pt.1) < threshold
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        // Hide the constraint if its point is part of a text box.
        if context.point_in_text_box_frame(self.point) {
            return;
        }
        let Some(pt) = context.get_point(self.point) else {
            return;
        };
        let (sx, sy) = to_screen((pt.x, pt.y));
        draw_point_marker(ctx, sx, sy, style);
    }
}

impl PointOnCurveConstraint {
    /// Reference user-visibility flag.
    pub fn user_visible(&self) -> bool {
        self.user_visible
    }

    /// Sets the reference user-visibility flag.
    pub fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// Serializes to the reference constraint dictionary shape.
    pub fn to_json(&self) -> Value {
        let data = json!({
            "type": "PointOnCurveConstraint",
            "user_visible": self.user_visible,
        });
        let _ = (self.point, self.start, self.end);
        data
    }
}
