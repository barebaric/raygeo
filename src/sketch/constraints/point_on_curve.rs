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
use crate::sketch::entities::bezier::Bezier;

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

/// Resolves the constrained bezier: `(point on curve ok, start, end,
/// cp1 offset, cp2 offset)`.
#[allow(clippy::type_complexity)]
fn resolve_bezier(
    ctx: &dyn PointContext,
    shape_id: u64,
) -> Option<(u64, u64, Option<[f64; 2]>, Option<[f64; 2]>)> {
    let entity = ctx.get_entity(shape_id)?;
    if entity.type_name() != "bezier" {
        return None;
    }
    let bezier = entity.as_any().downcast_ref::<Bezier>()?;
    Some((bezier.start, bezier.end, bezier.cp1, bezier.cp2))
}

/// Enforces a point lies on a cubic Bezier curve.
///
/// References the point and the curve by id and resolves them
/// through the point context at residual time, mirroring the
/// reference model's entity-reference design.  Curves without
/// control points degenerate to the straight-segment residual.
#[derive(Clone, Debug, PartialEq)]
pub struct PointOnCurveConstraint {
    /// The constrained point.
    pub point_id: u64,
    /// The constrained curve.
    pub shape_id: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl PointOnCurveConstraint {
    /// Creates a point-on-curve constraint.
    pub fn new(point_id: u64, shape_id: u64) -> Self {
        PointOnCurveConstraint {
            point_id,
            shape_id,
            user_visible: true,
        }
    }

    /// Absolute distance from the constrained point to the curve.
    fn distance(&self, reg: &dyn PointContext) -> f64 {
        let Some((start, end, cp1, cp2)) = resolve_bezier(reg, self.shape_id)
        else {
            return 0.0;
        };
        let (px, py) = reg.xy(self.point_id);
        let (sx, sy) = reg.xy(start);
        let (ex, ey) = reg.xy(end);
        if cp1.is_none() && cp2.is_none() {
            let (_, _, dist_sq) = get_line_segment_closest_point(
                Point::new(sx, sy),
                Point::new(ex, ey),
                px,
                py,
            );
            return dist_sq.sqrt();
        }
        let cp1x = sx + cp1.unwrap_or([0.0, 0.0])[0];
        let cp1y = sy + cp1.unwrap_or([0.0, 0.0])[1];
        let cp2x = ex + cp2.unwrap_or([0.0, 0.0])[0];
        let cp2y = ey + cp2.unwrap_or([0.0, 0.0])[1];
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

    fn referenced_points(&self, ctx: &dyn PointContext) -> Vec<u64> {
        match resolve_bezier(ctx, self.shape_id) {
            Some((start, end, _, _)) => {
                vec![self.point_id, start, end]
            }
            None => vec![],
        }
    }

    fn residual_count(&self, _ctx: &dyn PointContext) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        vec![self.distance(reg)]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let Some((start, end, cp1, cp2)) = resolve_bezier(reg, self.shape_id)
        else {
            return Vec::new();
        };
        let (px, py) = reg.xy(self.point_id);
        let (sx, sy) = reg.xy(start);
        let (ex, ey) = reg.xy(end);
        let cp1x = sx + cp1.unwrap_or([0.0, 0.0])[0];
        let cp1y = sy + cp1.unwrap_or([0.0, 0.0])[1];
        let cp2x = ex + cp2.unwrap_or([0.0, 0.0])[0];
        let cp2y = ey + cp2.unwrap_or([0.0, 0.0])[1];

        let closest = if cp1.is_none() && cp2.is_none() {
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
            (self.point_id, 0, -ux, -uy),
            (start, 0, w_start * ux, w_start * uy),
            (end, 0, w_end * ux, w_end * uy),
        ]
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &dyn PointContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let Some(pt) = model.get_point(self.point_id) else {
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
        if context.point_in_text_box_frame(self.point_id) {
            return;
        }
        let Some(pt) = context.get_point(self.point_id) else {
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
        json!({
            "type": "PointOnCurveConstraint",
            "point_id": self.point_id,
            "shape_id": self.shape_id,
            "user_visible": self.user_visible,
        })
    }
}
