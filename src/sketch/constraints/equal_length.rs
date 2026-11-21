//! Equal-length constraint between characteristic lengths.
//!
//! References the constrained entities by id and resolves their
//! characteristic-length pairs through the point context at residual
//! time, mirroring the reference model's entity-reference design.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_equal_symbol, screen_scale, CairoLike, DrawContext, DrawStyle,
    EQUAL_OFFSET_SCREEN,
};
use crate::sketch::entities::arc::Arc;

use std::f64::consts::FRAC_PI_2;

/// Enforces equal characteristic lengths across the referenced
/// entities: every other entity contributes one residual per
/// characteristic-length pair against the base entity's pairs.
#[derive(Clone, Debug, PartialEq)]
pub struct EqualLengthConstraint {
    /// The constrained entities.
    pub entity_ids: Vec<u64>,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

/// Unit vector of a segment, zero for degenerate segments.
fn unit_or_zero(dx: f64, dy: f64) -> (f64, f64) {
    let dist = dx.hypot(dy);
    if dist > 1e-9 {
        (dx / dist, dy / dist)
    } else {
        (0.0, 0.0)
    }
}

/// Anchor and normal direction of the equality symbol for one
/// entity: lines anchor at the segment midpoint with a perpendicular
/// normal, round shapes at a point on the shape with a radial
/// normal.  `arc_mid` overrides the arc anchor with the true arc
/// midpoint when the caller can resolve it.
type SymbolAnchor = (f64, f64, f64);

fn symbol_anchor(
    entity_type: &str,
    ids: &[u64],
    points: &dyn Fn(u64) -> Option<(f64, f64)>,
    arc_mid: Option<(f64, f64)>,
) -> Option<SymbolAnchor> {
    match entity_type {
        "line" => {
            let (ax, ay) = points(ids[0])?;
            let (bx, by) = points(ids[1])?;
            let tangent_angle = (by - ay).atan2(bx - ax);
            Some(((ax + bx) / 2.0, (ay + by) / 2.0, tangent_angle - FRAC_PI_2))
        }
        "arc" => {
            let (mx, my) = arc_mid.or_else(|| points(ids[0]))?;
            let (cx, cy) = points(ids[2])?;
            Some((mx, my, (my - cy).atan2(mx - cx)))
        }
        "circle" | "ellipse" => {
            let (mx, my) = points(ids[1])?;
            let (cx, cy) = points(ids[0])?;
            Some((mx, my, (my - cy).atan2(mx - cx)))
        }
        _ => None,
    }
}

/// Screen position of the equality symbol for one entity, mirroring
/// the reference `_get_symbol_pos`.
fn symbol_pos(
    entity_type: &str,
    ids: &[u64],
    points: &dyn Fn(u64) -> Option<(f64, f64)>,
    arc_mid: Option<(f64, f64)>,
    scale: f64,
    to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
) -> Option<(f64, f64)> {
    let (mid_x, mid_y, normal_angle) =
        symbol_anchor(entity_type, ids, points, arc_mid)?;
    let offset_dist_model = EQUAL_OFFSET_SCREEN / scale;
    let final_x = mid_x + offset_dist_model * normal_angle.cos();
    let final_y = mid_y + offset_dist_model * normal_angle.sin();
    Some(to_screen((final_x, final_y)))
}

impl EqualLengthConstraint {
    /// Creates an equal-length constraint for the given entities.
    pub fn new(entity_ids: Vec<u64>) -> Self {
        EqualLengthConstraint {
            entity_ids,
            user_visible: true,
        }
    }

    /// The entities' characteristic-length pairs, or None when any
    /// entity is missing.
    fn resolve_pairs(
        &self,
        ctx: &dyn PointContext,
    ) -> Option<Vec<Vec<[u64; 2]>>> {
        if self.entity_ids.len() < 2 {
            return None;
        }
        let mut per_entity = Vec::with_capacity(self.entity_ids.len());
        for entity_id in &self.entity_ids {
            let entity = ctx.get_entity(*entity_id)?;
            per_entity.push(entity.characteristic_length_pairs());
        }
        Some(per_entity)
    }
}

impl SketchConstraint for EqualLengthConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        EqualLengthConstraint::to_json(self)
    }

    fn referenced_points(&self, ctx: &dyn PointContext) -> Vec<u64> {
        let Some(per_entity) = self.resolve_pairs(ctx) else {
            return vec![];
        };
        let mut ids = Vec::new();
        for pairs in per_entity {
            for pair in pairs {
                for id in pair {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
        }
        ids
    }

    fn residual_count(&self, ctx: &dyn PointContext) -> usize {
        let Some(per_entity) = self.resolve_pairs(ctx) else {
            return 0;
        };
        let base_len = per_entity[0].len();
        per_entity[1..]
            .iter()
            .map(|pairs| base_len.max(pairs.len()))
            .sum()
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let Some(per_entity) = self.resolve_pairs(reg) else {
            return vec![];
        };
        let pair_length = |pair: &[u64; 2]| -> f64 {
            let (ax, ay) = reg.xy(pair[0]);
            let (bx, by) = reg.xy(pair[1]);
            (bx - ax).hypot(by - ay)
        };
        let base_pairs = &per_entity[0];
        let base_lens: Vec<f64> = base_pairs.iter().map(&pair_length).collect();

        let mut errors = Vec::new();
        for pairs in &per_entity[1..] {
            let ent_lens: Vec<f64> = pairs.iter().map(&pair_length).collect();
            let n = base_lens.len().max(ent_lens.len());
            for j in 0..n {
                let bl = if j < base_lens.len() {
                    base_lens[j]
                } else {
                    base_lens[0]
                };
                let el = if j < ent_lens.len() {
                    ent_lens[j]
                } else {
                    ent_lens[0]
                };
                errors.push(el - bl);
            }
        }
        errors
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let Some(per_entity) = self.resolve_pairs(reg) else {
            return vec![];
        };
        let base_pairs = &per_entity[0];

        let mut entries = Vec::new();
        let mut row = 0;
        for pairs in &per_entity[1..] {
            let n = base_pairs.len().max(pairs.len());
            for j in 0..n {
                let bp = if j < base_pairs.len() {
                    base_pairs[j]
                } else {
                    base_pairs[0]
                };
                let ep = if j < pairs.len() { pairs[j] } else { pairs[0] };

                let (bax, bay) = reg.xy(bp[0]);
                let (bbx, bby) = reg.xy(bp[1]);
                let (bux, buy) = unit_or_zero(bbx - bax, bby - bay);
                let (eax, eay) = reg.xy(ep[0]);
                let (ebx, eby) = reg.xy(ep[1]);
                let (eux, euy) = unit_or_zero(ebx - eax, eby - eay);

                entries.push((bp[0], row, bux, buy));
                entries.push((bp[1], row, -bux, -buy));
                entries.push((ep[0], row, -eux, -euy));
                entries.push((ep[1], row, eux, euy));

                row += 1;
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
        let scale = screen_scale(to_screen);
        let points = |id: u64| -> Option<(f64, f64)> {
            model.get_point(id).map(|point| (point.x, point.y))
        };
        for entity_id in &self.entity_ids {
            let Some(entity) = model.get_entity(*entity_id) else {
                continue;
            };
            let arc_mid =
                entity.as_any().downcast_ref::<Arc>().and_then(|arc| {
                    let (sx, sy) = points(arc.start)?;
                    let (ex, ey) = points(arc.end)?;
                    let (cx, cy) = points(arc.center)?;
                    let mid = crate::geo::shape::arc::get_arc_midpoint(
                        crate::geo::types::Point::new(sx, sy),
                        crate::geo::types::Point::new(ex, ey),
                        crate::geo::types::Point::new(cx, cy),
                        arc.clockwise,
                    );
                    Some((mid.x, mid.y))
                });
            let Some(pos) = symbol_pos(
                entity.type_name(),
                &entity.get_point_ids(),
                &points,
                arc_mid,
                scale,
                to_screen,
            ) else {
                continue;
            };
            if (mx - pos.0).hypot(my - pos.1) < threshold {
                return true;
            }
        }
        false
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let scale = screen_scale(to_screen);
        let points = |id: u64| -> Option<(f64, f64)> {
            context.get_point(id).map(|point| (point.x, point.y))
        };
        for entity_id in &self.entity_ids {
            let Some(ids) = context.entity_point_ids(*entity_id) else {
                continue;
            };
            let entity_type = match context.entity_type(*entity_id) {
                Some(name) => name,
                None => continue,
            };
            let Some(pos) =
                symbol_pos(&entity_type, &ids, &points, None, scale, to_screen)
            else {
                continue;
            };
            draw_equal_symbol(
                ctx,
                pos.0,
                pos.1,
                style,
                screen_scale(to_screen),
            );
        }
    }
}

impl EqualLengthConstraint {
    /// Serializes to the reference constraint dictionary shape.
    pub fn to_json(&self) -> Value {
        json!({
            "type": "EqualLengthConstraint",
            "entity_ids": self.entity_ids,
            "user_visible": self.user_visible,
        })
    }
}
