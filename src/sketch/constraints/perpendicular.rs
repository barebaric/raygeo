//! Perpendicular constraint between two entities.

use serde_json::{json, Value};

use crate::geo::shape::circle::get_circle_circle_intersections;
use crate::geo::shape::line::get_line_line_intersection;
use crate::geo::types::Point;
use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_selection_underlay, draw_status_color,
    status_color, CairoLike, DrawContext, DrawStyle,
};

/// Screen-space radius of the line-line marker arc, matching the
/// reference renderer.
const VISUAL_RADIUS: f64 = 16.0;

/// The resolved geometry of the two perpendicular operands,
/// mirroring the reference constraint's entity-type cases: two
/// lines, a line through a circle/arc center, or two circles
/// meeting at a right angle.
#[derive(Clone, Debug, PartialEq)]
pub enum PerpendicularTarget {
    /// Line-line: direction dot product.
    LineLine { a1: u64, a2: u64, b1: u64, b2: u64 },
    /// Line and circle/arc: the line passes through the center.
    LineCenter { l1: u64, l2: u64, center: u64 },
    /// Circle/arc pair: `r1^2 + r2^2 == dist(c1, c2)^2`.
    Circles {
        c1: u64,
        r1_point: u64,
        c2: u64,
        r2_point: u64,
    },
}

/// Resolves the operand pair to the constraint's geometry cases:
/// two lines, a line through a circle/arc center, or two circles
/// meeting at a right angle.
fn resolve_target(
    ctx: &dyn PointContext,
    e1_id: u64,
    e2_id: u64,
) -> Option<PerpendicularTarget> {
    let e1 = ctx.get_entity(e1_id)?;
    let e2 = ctx.get_entity(e2_id)?;
    let t1 = e1.type_name();
    let t2 = e2.type_name();
    let ids1 = e1.get_point_ids();
    let ids2 = e2.get_point_ids();
    if t1 == "line" && t2 == "line" {
        return Some(PerpendicularTarget::LineLine {
            a1: ids1[0],
            a2: ids1[1],
            b1: ids2[0],
            b2: ids2[1],
        });
    }
    let (line_ids, shape_ids) = if t1 == "line" {
        (ids1.clone(), ids2.clone())
    } else {
        (ids2.clone(), ids1.clone())
    };
    let shape_type = if t1 == "line" { t2 } else { t1 };
    match shape_type {
        "circle" | "arc" => {
            let (center, _radius_pt) = if shape_type == "circle" {
                (shape_ids[0], shape_ids[1])
            } else {
                (shape_ids[2], shape_ids[0])
            };
            return Some(PerpendicularTarget::LineCenter {
                l1: line_ids[0],
                l2: line_ids[1],
                center,
            });
        }
        _ => {}
    }
    let (c1, r1, c2, r2) = radius_points(t1, &ids1, t2, &ids2)?;
    Some(PerpendicularTarget::Circles {
        c1,
        r1_point: r1,
        c2,
        r2_point: r2,
    })
}

/// Resolves the operand pair for the draw path, where entities are
/// inspected through the draw context's duck-typed surface.
fn resolve_target_of_context(
    context: &dyn DrawContext,
    e1_id: u64,
    e2_id: u64,
) -> Option<PerpendicularTarget> {
    let t1 = context.entity_type(e1_id)?.as_ref().to_owned();
    let t2 = context.entity_type(e2_id)?.as_ref().to_owned();
    let ids1 = context.entity_point_ids(e1_id)?;
    let ids2 = context.entity_point_ids(e2_id)?;
    if t1 == "line" && t2 == "line" {
        return Some(PerpendicularTarget::LineLine {
            a1: ids1[0],
            a2: ids1[1],
            b1: ids2[0],
            b2: ids2[1],
        });
    }
    let (line_ids, shape_ids) = if t1 == "line" {
        (ids1.clone(), ids2.clone())
    } else {
        (ids2.clone(), ids1.clone())
    };
    let shape_type = if t1 == "line" { t2.clone() } else { t1.clone() };
    match shape_type.as_str() {
        "circle" => {
            return Some(PerpendicularTarget::LineCenter {
                l1: line_ids[0],
                l2: line_ids[1],
                center: shape_ids[0],
            });
        }
        "arc" => {
            return Some(PerpendicularTarget::LineCenter {
                l1: line_ids[0],
                l2: line_ids[1],
                center: shape_ids[2],
            });
        }
        _ => {}
    }
    let (c1, r1, c2, r2) = radius_points(&t1, &ids1, &t2, &ids2)?;
    Some(PerpendicularTarget::Circles {
        c1,
        r1_point: r1,
        c2,
        r2_point: r2,
    })
}

/// Center and radius-bearing point ids of a circle/arc.
fn radius_points(
    t1: &str,
    ids1: &[u64],
    t2: &str,
    ids2: &[u64],
) -> Option<(u64, u64, u64, u64)> {
    let shape_points = |t: &str, ids: &[u64]| -> Option<(u64, u64)> {
        match t {
            "circle" => Some((ids[0], ids[1])),
            "arc" => Some((ids[2], ids[0])),
            _ => None,
        }
    };
    let (c1, r1) = shape_points(t1, ids1)?;
    let (c2, r2) = shape_points(t2, ids2)?;
    Some((c1, r1, c2, r2))
}

/// Enforces perpendicularity between two entities.
#[derive(Clone, Debug, PartialEq)]
pub struct PerpendicularConstraint {
    /// First operand entity.
    pub e1_id: u64,
    /// Second operand entity.
    pub e2_id: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl PerpendicularConstraint {
    /// Creates a perpendicular constraint between two entities.
    pub fn new(e1_id: u64, e2_id: u64) -> Self {
        PerpendicularConstraint {
            e1_id,
            e2_id,
            user_visible: true,
        }
    }
}

impl SketchConstraint for PerpendicularConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        PerpendicularConstraint::to_json(self)
    }

    fn referenced_points(&self, ctx: &dyn PointContext) -> Vec<u64> {
        let mut ids = Vec::new();
        let mut push = |id: u64| {
            if !ids.contains(&id) {
                ids.push(id);
            }
        };
        if let Some(target) = resolve_target(ctx, self.e1_id, self.e2_id) {
            match target {
                PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                    push(a1);
                    push(a2);
                    push(b1);
                    push(b2);
                }
                PerpendicularTarget::LineCenter { l1, l2, center } => {
                    push(l1);
                    push(l2);
                    push(center);
                }
                PerpendicularTarget::Circles {
                    c1,
                    r1_point,
                    c2,
                    r2_point,
                } => {
                    push(c1);
                    push(r1_point);
                    push(c2);
                    push(r2_point);
                }
            }
        }
        ids
    }

    fn residual_count(&self, _ctx: &dyn PointContext) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let Some(target) = resolve_target(reg, self.e1_id, self.e2_id) else {
            return vec![0.0];
        };
        match &target {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                let (ax1, ay1) = reg.xy(*a1);
                let (ax2, ay2) = reg.xy(*a2);
                let (bx1, by1) = reg.xy(*b1);
                let (bx2, by2) = reg.xy(*b2);
                let dx1 = ax2 - ax1;
                let dy1 = ay2 - ay1;
                let dx2 = bx2 - bx1;
                let dy2 = by2 - by1;
                vec![dx1 * dx2 + dy1 * dy2]
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                let (x1, y1) = reg.xy(*l1);
                let (x2, y2) = reg.xy(*l2);
                let (cx, cy) = reg.xy(*center);
                vec![(x2 - x1) * (cy - y1) - (cx - x1) * (y2 - y1)]
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                let (c1x, c1y) = reg.xy(*c1);
                let (p1x, p1y) = reg.xy(*r1_point);
                let (c2x, c2y) = reg.xy(*c2);
                let (p2x, p2y) = reg.xy(*r2_point);
                let r1_sq = (p1x - c1x).hypot(p1y - c1y).powi(2);
                let r2_sq = (p2x - c2x).hypot(p2y - c2y).powi(2);
                let dist_sq = (c2x - c1x).powi(2) + (c2y - c1y).powi(2);
                vec![r1_sq + r2_sq - dist_sq]
            }
        }
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let Some(target) = resolve_target(reg, self.e1_id, self.e2_id) else {
            return vec![];
        };
        let mut entries = Vec::new();
        match &target {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                let (ax1, ay1) = reg.xy(*a1);
                let (ax2, ay2) = reg.xy(*a2);
                let (bx1, by1) = reg.xy(*b1);
                let (bx2, by2) = reg.xy(*b2);
                let dx1 = ax2 - ax1;
                let dy1 = ay2 - ay1;
                let dx2 = bx2 - bx1;
                let dy2 = by2 - by1;
                entries.push((*a1, 0, -dx2, -dy2));
                entries.push((*a2, 0, dx2, dy2));
                entries.push((*b1, 0, -dx1, -dy1));
                entries.push((*b2, 0, dx1, dy1));
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                let (x1, y1) = reg.xy(*l1);
                let (x2, y2) = reg.xy(*l2);
                let (cx, cy) = reg.xy(*center);
                let dx = x2 - x1;
                let dy = y2 - y1;
                entries.push((*center, 0, -dy, dx));
                entries.push((*l1, 0, dy - (cy - y1), -dx + (cx - x1)));
                entries.push((*l2, 0, cy - y1, -(cx - x1)));
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                let (c1x, c1y) = reg.xy(*c1);
                let (p1x, p1y) = reg.xy(*r1_point);
                let (c2x, c2y) = reg.xy(*c2);
                let (p2x, p2y) = reg.xy(*r2_point);
                let d_dist_x = 2.0 * (c2x - c1x);
                let d_dist_y = 2.0 * (c2y - c1y);
                entries.push((
                    *r1_point,
                    0,
                    2.0 * (p1x - c1x),
                    2.0 * (p1y - c1y),
                ));
                entries.push((
                    *c1,
                    0,
                    d_dist_x - 2.0 * (p1x - c1x),
                    d_dist_y - 2.0 * (p1y - c1y),
                ));
                entries.push((
                    *r2_point,
                    0,
                    2.0 * (p2x - c2x),
                    2.0 * (p2y - c2y),
                ));
                entries.push((
                    *c2,
                    0,
                    -d_dist_x - 2.0 * (p2x - c2x),
                    -d_dist_y - 2.0 * (p2y - c2y),
                ));
            }
        }
        entries
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &dyn PointContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let Some(target) = resolve_target(model, self.e1_id, self.e2_id) else {
            return false;
        };
        match &target {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                let (Some(p1), Some(p2), Some(p3), Some(p4)) = (
                    model.get_point(*a1),
                    model.get_point(*a2),
                    model.get_point(*b1),
                    model.get_point(*b2),
                ) else {
                    return false;
                };
                let pt = get_line_line_intersection(
                    Point::new(p1.x, p1.y),
                    Point::new(p2.x, p2.y),
                    Point::new(p3.x, p3.y),
                    Point::new(p4.x, p4.y),
                );
                let (ix, iy) = match pt {
                    Some(point) => (point.x, point.y),
                    None => (
                        (p1.x + p2.x + p3.x + p4.x) / 4.0,
                        (p1.y + p2.y + p3.y + p4.y) / 4.0,
                    ),
                };
                let (sx, sy) = to_screen((ix, iy));
                let s_p1 = to_screen((p1.x, p1.y));
                let s_p2 = to_screen((p2.x, p2.y));
                let s_p3 = to_screen((p3.x, p3.y));
                let s_p4 = to_screen((p4.x, p4.y));

                let dist_sq = |a: (f64, f64), b: (f64, f64)| {
                    (a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)
                };
                let here = (sx, sy);
                let v1p = if dist_sq(s_p1, here) > dist_sq(s_p2, here) {
                    s_p1
                } else {
                    s_p2
                };
                let v2p = if dist_sq(s_p3, here) > dist_sq(s_p4, here) {
                    s_p3
                } else {
                    s_p4
                };
                let ang1 = (v1p.1 - sy).atan2(v1p.0 - sx);
                let ang2 = (v2p.1 - sy).atan2(v2p.0 - sx);

                let mut diff = ang2 - ang1;
                while diff <= -std::f64::consts::PI {
                    diff += 2.0 * std::f64::consts::PI;
                }
                while diff > std::f64::consts::PI {
                    diff -= 2.0 * std::f64::consts::PI;
                }
                let mid_angle = ang1 + diff / 2.0;
                let target_x = sx + mid_angle.cos() * VISUAL_RADIUS * 0.6;
                let target_y = sy + mid_angle.sin() * VISUAL_RADIUS * 0.6;
                (mx - target_x).hypot(my - target_y) < threshold
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                // Box-style marker.  The reference anchors it at the
                // line/circle intersection when one lies on the
                // segment and falls back to the center otherwise; the
                // circle radius is not resolvable from the resolved
                // points alone, so the center fallback always applies.
                let (Some(lp1), Some(lp2), Some(center)) = (
                    model.get_point(*l1),
                    model.get_point(*l2),
                    model.get_point(*center),
                ) else {
                    return false;
                };
                let dx = lp2.x - lp1.x;
                let dy = lp2.y - lp1.y;
                if dx.hypot(dy) < 1e-9 {
                    return false;
                }
                let (sx, sy) = to_screen((center.x, center.y));
                (mx - sx).hypot(my - sy) < threshold
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                let (Some(c1p), Some(r1p), Some(c2p), Some(r2p)) = (
                    model.get_point(*c1),
                    model.get_point(*r1_point),
                    model.get_point(*c2),
                    model.get_point(*r2_point),
                ) else {
                    return false;
                };
                let r1 = (r1p.x - c1p.x).hypot(r1p.y - c1p.y);
                let r2 = (r2p.x - c2p.x).hypot(r2p.y - c2p.y);
                let intersections = get_circle_circle_intersections(
                    Point::new(c1p.x, c1p.y),
                    r1,
                    Point::new(c2p.x, c2p.y),
                    r2,
                );
                let Some(point) = intersections.first() else {
                    return false;
                };
                let (sx, sy) = to_screen((point.x, point.y));
                (mx - sx).hypot(my - sy) < threshold
            }
        }
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        ctx.save();
        ctx.set_line_width(1.5);
        let target =
            match resolve_target_of_context(context, self.e1_id, self.e2_id) {
                Some(target) => target,
                None => {
                    ctx.restore();
                    return;
                }
            };
        match &target {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                let (Some(p1), Some(p2), Some(p3), Some(p4)) = (
                    context.get_point(*a1),
                    context.get_point(*a2),
                    context.get_point(*b1),
                    context.get_point(*b2),
                ) else {
                    ctx.restore();
                    return;
                };
                let pt = get_line_line_intersection(
                    Point::new(p1.x, p1.y),
                    Point::new(p2.x, p2.y),
                    Point::new(p3.x, p3.y),
                    Point::new(p4.x, p4.y),
                );
                let (ix, iy) = match pt {
                    Some(point) => (point.x, point.y),
                    None => (
                        (p1.x + p2.x + p3.x + p4.x) / 4.0,
                        (p1.y + p2.y + p3.y + p4.y) / 4.0,
                    ),
                };
                let (sx, sy) = to_screen((ix, iy));
                let s_p1 = to_screen((p1.x, p1.y));
                let s_p2 = to_screen((p2.x, p2.y));
                let s_p3 = to_screen((p3.x, p3.y));
                let s_p4 = to_screen((p4.x, p4.y));

                let dist_sq = |a: (f64, f64), b: (f64, f64)| {
                    (a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)
                };
                let here = (sx, sy);
                let v1p = if dist_sq(s_p1, here) > dist_sq(s_p2, here) {
                    s_p1
                } else {
                    s_p2
                };
                let v2p = if dist_sq(s_p3, here) > dist_sq(s_p4, here) {
                    s_p3
                } else {
                    s_p4
                };
                let ang1 = (v1p.1 - sy).atan2(v1p.0 - sx);
                let ang2 = (v2p.1 - sy).atan2(v2p.0 - sx);

                let mut diff = ang2 - ang1;
                while diff <= -std::f64::consts::PI {
                    diff += 2.0 * std::f64::consts::PI;
                }
                while diff > std::f64::consts::PI {
                    diff -= 2.0 * std::f64::consts::PI;
                }

                if diff > 0.0 {
                    ctx.arc(sx, sy, VISUAL_RADIUS, ang1, ang2);
                } else {
                    ctx.arc_negative(sx, sy, VISUAL_RADIUS, ang1, ang2);
                }
                if style.is_selected {
                    draw_selection_underlay(ctx);
                }
                draw_conflict_underlay(ctx, style);
                draw_status_color(ctx, style);
                ctx.stroke();

                let mid = ang1 + diff / 2.0;
                let dx = sx + mid.cos() * VISUAL_RADIUS * 0.6;
                let dy = sy + mid.sin() * VISUAL_RADIUS * 0.6;
                let (r, g, b) = status_color(style);
                ctx.set_source_rgba(r, g, b, 1.0);
                ctx.arc(dx, dy, 2.0, 0.0, 2.0 * std::f64::consts::PI);
                ctx.fill();
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                // Box-style marker anchored at the center (the
                // line/circle intersection fallback; see is_hit).
                let (Some(lp1), Some(lp2), Some(center)) = (
                    context.get_point(*l1),
                    context.get_point(*l2),
                    context.get_point(*center),
                ) else {
                    ctx.restore();
                    return;
                };
                if (lp2.x - lp1.x).hypot(lp2.y - lp1.y) < 1e-9 {
                    ctx.restore();
                    return;
                }
                let (sx, sy) = to_screen((center.x, center.y));
                let sz = 8.0;
                ctx.rectangle(sx - sz, sy - sz, sz * 2.0, sz * 2.0);
                if style.is_selected {
                    draw_selection_underlay(ctx);
                }
                draw_conflict_underlay(ctx, style);
                draw_status_color(ctx, style);
                ctx.stroke();
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                // Box-style marker at the first circle-circle
                // intersection.
                let (Some(c1p), Some(r1p), Some(c2p), Some(r2p)) = (
                    context.get_point(*c1),
                    context.get_point(*r1_point),
                    context.get_point(*c2),
                    context.get_point(*r2_point),
                ) else {
                    ctx.restore();
                    return;
                };
                let r1 = (r1p.x - c1p.x).hypot(r1p.y - c1p.y);
                let r2 = (r2p.x - c2p.x).hypot(r2p.y - c2p.y);
                let intersections = get_circle_circle_intersections(
                    Point::new(c1p.x, c1p.y),
                    r1,
                    Point::new(c2p.x, c2p.y),
                    r2,
                );
                let Some(point) = intersections.first() else {
                    ctx.restore();
                    return;
                };
                let (sx, sy) = to_screen((point.x, point.y));
                let sz = 8.0;
                ctx.rectangle(sx - sz, sy - sz, sz * 2.0, sz * 2.0);
                if style.is_selected {
                    draw_selection_underlay(ctx);
                }
                draw_conflict_underlay(ctx, style);
                draw_status_color(ctx, style);
                ctx.stroke();
            }
        }
        ctx.restore();
    }
}

impl PerpendicularConstraint {
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
            "type": "PerpendicularConstraint",
            "e1_id": self.e1_id,
            "e2_id": self.e2_id,
            "user_visible": self.user_visible,
        })
    }
}
