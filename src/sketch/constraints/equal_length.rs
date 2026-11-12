//! Equal-length constraint between characteristic lengths.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_equal_symbol, screen_scale, CairoLike, DrawContext, DrawStyle,
    EQUAL_OFFSET_SCREEN,
};
use crate::sketch::registry::EntityRegistry;

use std::f64::consts::FRAC_PI_2;

/// Enforces equal characteristic lengths.  Each pair is
/// `(base_a, base_b, other_a, other_b)` with one residual per pair;
/// the caller expands the entities' characteristic-length pairs
/// (including the shorter-side fallback) into flat pairs.
#[derive(Clone, Debug, PartialEq)]
pub struct EqualLengthConstraint {
    /// Resolved characteristic-length pairs.
    pub pairs: Vec<[u64; 4]>,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl EqualLengthConstraint {
    /// Creates an equal-length constraint from resolved pairs.
    pub fn new(pairs: Vec<[u64; 4]>) -> Self {
        EqualLengthConstraint {
            user_visible: true,
            pairs,
        }
    }

    fn pair_length(&self, a: u64, b: u64, reg: &dyn PointContext) -> f64 {
        let (ax, ay) = reg.xy(a);
        let (bx, by) = reg.xy(b);
        (bx - ax).hypot(by - ay)
    }
}

impl SketchConstraint for EqualLengthConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = Vec::new();
        for pair in &self.pairs {
            for id in pair {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        self.pairs.len()
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let mut errors = Vec::with_capacity(self.pairs.len());
        for pair in &self.pairs {
            let base = self.pair_length(pair[0], pair[1], reg);
            let other = self.pair_length(pair[2], pair[3], reg);
            errors.push(other - base);
        }
        errors
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let mut entries = Vec::new();
        for (row, pair) in self.pairs.iter().enumerate() {
            let base_dx = reg.xy(pair[1]).0 - reg.xy(pair[0]).0;
            let base_dy = reg.xy(pair[1]).1 - reg.xy(pair[0]).1;
            let (bux, buy) = unit_or_zero(base_dx, base_dy);
            let other_dx = reg.xy(pair[3]).0 - reg.xy(pair[2]).0;
            let other_dy = reg.xy(pair[3]).1 - reg.xy(pair[2]).1;
            let (eux, euy) = unit_or_zero(other_dx, other_dy);
            entries.push((pair[0], row, bux, buy));
            entries.push((pair[1], row, -bux, -buy));
            entries.push((pair[2], row, -eux, -euy));
            entries.push((pair[3], row, eux, euy));
        }
        entries
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        // The reference walks the entity ids and offsets one equality
        // symbol per entity; each resolved pair carries two entity
        // segments (base and other), one symbol each (line-branch
        // math).
        let p0 = to_screen((0.0, 0.0));
        let p1 = to_screen((1.0, 0.0));
        let mut scale = (p1.0 - p0.0).hypot(p1.1 - p0.1);
        if scale < 1e-9 {
            scale = 1.0;
        }

        for pair in &self.pairs {
            for segment in [[pair[0], pair[1]], [pair[2], pair[3]]] {
                let (Some(a), Some(b)) =
                    (model.get_point(segment[0]), model.get_point(segment[1]))
                else {
                    continue;
                };
                let mid_x = (a.x + b.x) / 2.0;
                let mid_y = (a.y + b.y) / 2.0;
                let tangent_angle = (b.y - a.y).atan2(b.x - a.x);
                let normal_angle = tangent_angle - FRAC_PI_2;

                let offset_dist_model = 15.0 / scale;
                let final_x = mid_x + offset_dist_model * normal_angle.cos();
                let final_y = mid_y + offset_dist_model * normal_angle.sin();
                let (sym_sx, sym_sy) = to_screen((final_x, final_y));
                if (mx - sym_sx).hypot(my - sym_sy) < threshold {
                    return true;
                }
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
        // One equality symbol per resolved segment (line-branch
        // math; see is_hit).
        let scale = screen_scale(to_screen);
        for pair in &self.pairs {
            for segment in [[pair[0], pair[1]], [pair[2], pair[3]]] {
                let (Some(a), Some(b)) = (
                    context.get_point(segment[0]),
                    context.get_point(segment[1]),
                ) else {
                    continue;
                };
                let mid_x = (a.x + b.x) / 2.0;
                let mid_y = (a.y + b.y) / 2.0;
                let tangent_angle = (b.y - a.y).atan2(b.x - a.x);
                let normal_angle = tangent_angle - FRAC_PI_2;

                let offset_dist_model = EQUAL_OFFSET_SCREEN / scale;
                let final_x = mid_x + offset_dist_model * normal_angle.cos();
                let final_y = mid_y + offset_dist_model * normal_angle.sin();
                let (sym_sx, sym_sy) = to_screen((final_x, final_y));
                draw_equal_symbol(
                    ctx,
                    sym_sx,
                    sym_sy,
                    style,
                    screen_scale(to_screen),
                );
            }
        }
    }
}

/// Unit vector, defaulting to zero for degenerate segments.
fn unit_or_zero(dx: f64, dy: f64) -> (f64, f64) {
    let dist = dx.hypot(dy);
    if dist > 1e-9 {
        (dx / dist, dy / dist)
    } else {
        (0.0, 0.0)
    }
}

impl EqualLengthConstraint {
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
        let mut data = json!({
            "type": "EqualLengthConstraint",
            "user_visible": self.user_visible,
        });
        data["entity_ids"] = json!([]);
        let _ = &self.pairs;
        data
    }
}
