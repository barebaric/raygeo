//! Interpolation helpers for the adaptive stepping solver.
//!
//! The `Interpolation`
//! struct maintains a min/max bracket of error values and interpolates
//! linearly to find the steering angle that achieves the target
//! cut-area-per-distance.

use crate::geo::shape::polygon::get_polygon_signed_area;
use crate::geo::shape::polygon::is_point_in_polygon;
use crate::types::{Point, Polygon};

/// Check whether `pt` lies in a valid tool area defined by polygon
/// shells and holes.  CCW-wound polygons are outer shells; CW-wound
/// polygons are holes.  A point is valid iff it is inside at least one
/// CCW polygon AND outside all CW polygons.
pub(crate) fn point_in_valid_area(pt: Point, area: &[Polygon]) -> bool {
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

pub(crate) fn rotate(v: Point, angle: f64) -> Point {
    let c = angle.cos();
    let s = angle.sin();
    Point::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct InterpItem {
    pub(crate) angle: f64,
    pub(crate) error: f64,
    pub(crate) pos: Point,
    pub(crate) is_conventional: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Interpolation {
    min: Option<InterpItem>,
    max: Option<InterpItem>,
}

impl Interpolation {
    pub(crate) fn new() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    pub(crate) fn min_angle(&self) -> f64 {
        -std::f64::consts::PI / 4.0
    }

    pub(crate) fn max_angle(&self) -> f64 {
        std::f64::consts::PI / 4.0
    }

    pub(crate) fn joint_is_valid(&self) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) => {
                min.error < 0.0
                    && max.error >= 0.0
                    && (!min.is_conventional || !max.is_conventional)
            }
            _ => false,
        }
    }

    pub(crate) fn has_pos(&self, pos: Point) -> bool {
        self.min.is_some_and(|m| m.pos == pos)
            || self.max.is_some_and(|m| m.pos == pos)
    }

    pub(crate) fn clamp_angle(&self, angle: f64, max_deflection: f64) -> f64 {
        angle.clamp(-max_deflection, max_deflection)
    }

    pub(crate) fn interpolate(&self) -> f64 {
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

    pub(crate) fn add(
        &mut self,
        error: f64,
        angle: f64,
        pos: Point,
        mut allow_skip: bool,
        is_conventional: bool,
    ) {
        loop {
            let item = InterpItem {
                angle,
                error,
                pos,
                is_conventional,
            };
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
            let min_c = self.min.unwrap().is_conventional;
            let max_c = self.max.unwrap().is_conventional;
            if is_conventional && (min_c ^ max_c) {
                if !allow_skip {
                    if min_c {
                        self.min = None;
                    } else {
                        self.max = None;
                    }
                    allow_skip = false;
                    continue;
                }
                return;
            }
            if self.joint_is_valid() {
                if error < 0.0 {
                    self.min = Some(item);
                } else {
                    self.max = Some(item);
                }
                return;
            }
            if allow_skip
                && error.abs() > self.min.unwrap().error.abs()
                && error.abs() > self.max.unwrap().error.abs()
                && (is_conventional || !min_c || !max_c)
            {
                return;
            }
            if min_c ^ max_c {
                if min_c {
                    self.min = None;
                } else {
                    self.max = None;
                }
            } else if self.min.unwrap().error.abs()
                > self.max.unwrap().error.abs()
            {
                self.min = None;
            } else {
                self.max = None;
            }
            allow_skip = false;
        }
    }
}
