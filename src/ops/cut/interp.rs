//! Interpolation helpers for the adaptive stepping solver.
//!
//! The `Interpolation`
//! struct maintains a min/max bracket of error values and interpolates
//! linearly to find the steering angle that achieves the target
//! cut-area-per-distance.

use prof_macros::prof;

use crate::geo::shape::polygon::get_polygon_signed_area;
use crate::geo::shape::polygon::is_point_in_polygon;
use crate::ops::cut::stepper::STEP_ANGLE_BOUND;
use crate::types::{Point, Polygon};

/// Check whether `pt` lies in a valid tool area defined by polygon
/// shells and holes.  CCW-wound polygons are outer shells; CW-wound
/// polygons are holes.  A point is valid iff it is inside at least one
/// CCW polygon AND outside all CW polygons.
pub fn point_in_valid_area(pt: Point, area: &[Polygon]) -> bool {
    let mut inside_outer = false;
    let mut inside_hole = false;
    for poly in area {
        if poly.len() < 3 {
            continue;
        }
        let is_ccw = get_polygon_signed_area(poly) > 0.0;
        let inside = is_point_in_polygon(pt, poly);
        if is_ccw && inside {
            inside_outer = true;
        } else if !is_ccw && inside {
            inside_hole = true;
        }
    }
    inside_outer && !inside_hole
}

pub fn rotate(v: Point, angle: f64) -> Point {
    let c = angle.cos();
    let s = angle.sin();
    Point::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

#[derive(Clone, Copy, Debug)]
pub struct InterpItem {
    pub angle: f64,
    pub error: f64,
    pub pos: Point,
}

#[derive(Clone, Copy, Debug)]
pub struct Interpolation {
    min: Option<InterpItem>,
    max: Option<InterpItem>,
    min_bound: f64,
    max_bound: f64,
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::new(-STEP_ANGLE_BOUND, STEP_ANGLE_BOUND)
    }
}

impl Interpolation {
    pub fn new(min_bound: f64, max_bound: f64) -> Self {
        Self {
            min: None,
            max: None,
            min_bound,
            max_bound,
        }
    }

    pub fn min_angle(&self) -> f64 {
        self.min_bound
    }

    pub fn max_angle(&self) -> f64 {
        self.max_bound
    }

    pub fn joint_is_valid(&self) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) => min.error < 0.0 && max.error >= 0.0,
            _ => false,
        }
    }

    pub fn has_pos(&self, pos: Point) -> bool {
        self.min.is_some_and(|m| m.pos == pos)
            || self.max.is_some_and(|m| m.pos == pos)
    }

    pub fn clamp_angle(&self, angle: f64, max_deflection: f64) -> f64 {
        angle.clamp(-max_deflection, max_deflection)
    }

    #[prof]
    pub fn interpolate(&self) -> f64 {
        let min = match self.min {
            Some(m) => m,
            None => return self.min_angle(),
        };
        let max = match self.max {
            Some(m) => m,
            None => return self.max_angle(),
        };
        let mut p = (0.0 - min.error) / (max.error - min.error);
        p = p.clamp(0.2, 0.8);
        min.angle * (1.0 - p) + max.angle * p
    }

    /// Add a new sample to the bracket.
    ///
    /// Maintains the invariant that `min.error <= max.error`.  The
    /// goal is to bracket the root (zero crossing), so we keep the
    /// samples closest to zero on each side:
    /// - If the new sample has a different sign than one endpoint, it
    ///   can establish or refine a bracket by replacing that endpoint.
    /// - If all samples share the same sign, keep the two closest to
    ///   zero (discard the worst).
    #[prof]
    pub fn add(&mut self, error: f64, angle: f64, pos: Point) {
        let item = InterpItem { angle, error, pos };
        if self.min.is_none() {
            self.min = Some(item);
            return;
        }
        if self.max.is_none() {
            self.max = Some(item);
            if self.min.unwrap().error > self.max.unwrap().error {
                std::mem::swap(&mut self.min, &mut self.max);
            }
            return;
        }
        // Both sides populated; invariant: min.error <= max.error.
        let min_err = self.min.unwrap().error;
        let max_err = self.max.unwrap().error;

        // Case 1: valid bracket exists (min < 0, max >= 0).
        if min_err < 0.0 && max_err >= 0.0 {
            if error < 0.0 {
                // Refine the negative side: replace min if closer to zero.
                if error >= min_err {
                    self.min = Some(item);
                }
            } else {
                // Refine the positive side: replace max if closer to zero.
                if error <= max_err {
                    self.max = Some(item);
                }
            }
            return;
        }

        // Case 2: no bracket yet — all same sign.  Keep the two
        // samples closest to zero.
        let candidates = [
            (min_err, self.min.unwrap()),
            (max_err, self.max.unwrap()),
            (error, item),
        ];
        // Sort by |error| ascending; keep the two smallest.
        let mut idx = [0usize, 1, 2];
        idx.sort_by(|&a, &b| {
            candidates[a]
                .0
                .abs()
                .partial_cmp(&candidates[b].0.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let a = candidates[idx[0]].1;
        let b = candidates[idx[1]].1;
        if a.error <= b.error {
            self.min = Some(a);
            self.max = Some(b);
        } else {
            self.min = Some(b);
            self.max = Some(a);
        }
    }
}
