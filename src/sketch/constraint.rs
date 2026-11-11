//! Typed constraint descriptors for the sketch solver.
//!
//! Every variant mirrors one constraint of the sketch model.  The
//! caller resolves entity references to concrete point ids (e.g. an
//! arc contributes its center and start point) and resolves parameter
//! expressions to plain floats before building a descriptor, so the
//! solver never needs entity knowledge.

/// One constraint of a sketch problem.
#[derive(Clone, Debug)]
pub enum ConstraintSpec {
    /// Two points share their Y coordinate.
    Horizontal { p1: u64, p2: u64 },
    /// Two points share their X coordinate.
    Vertical { p1: u64, p2: u64 },
    /// Two points are at the same location (two residuals).
    Coincident { p1: u64, p2: u64 },
    /// Fixed distance between two points.
    Distance { p1: u64, p2: u64, value: f64 },
    /// `dist(p1, p2) == ratio * dist(p3, p4)`.
    AspectRatio {
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        ratio: f64,
    },
    /// Angle between two lines, measured clockwise from the anchor
    /// line (p1, p2) towards (p3, p4) at their intersection.  The far
    /// point ids pick which ray of each line is used; `None` picks
    /// the endpoint farther from the intersection.  Value in degrees.
    Angle {
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        e1_far: Option<u64>,
        e2_far: Option<u64>,
        value_deg: f64,
    },
    /// Three points on one line (cross product residual).
    Collinear { p1: u64, p2: u64, p3: u64 },
    /// `dist(p1, p2) == dist(p3, p4)`.
    EqualDistance { p1: u64, p2: u64, p3: u64, p4: u64 },
    /// `(p_width - p_origin) == (p4 - p_height)` (two residuals).
    Parallelogram {
        p_origin: u64,
        p_width: u64,
        p_height: u64,
        p4: u64,
    },
    /// `p2` is `p1` rotated by `angle_rad` about `center`.
    Rotational {
        center: u64,
        p1: u64,
        p2: u64,
        angle_rad: f64,
    },
    /// `center` is the midpoint of `p1` and `p2` (two residuals).
    SymmetryPoint { p1: u64, p2: u64, center: u64 },
    /// `p1` and `p2` are mirrored across the line `(l1, l2)`
    /// (two residuals).
    SymmetryLine { p1: u64, p2: u64, l1: u64, l2: u64 },
    /// Radius of a circle/arc: `dist(center, radius_point) == value`.
    Radius {
        center: u64,
        radius_point: u64,
        value: f64,
    },
    /// Diameter: `2 * dist(center, radius_point) == value`.
    Diameter {
        center: u64,
        radius_point: u64,
        value: f64,
    },
    /// Two lines perpendicular (direction dot product residual).
    PerpLineLine { a1: u64, a2: u64, b1: u64, b2: u64 },
    /// A line passing through a circle/arc center (cross product
    /// residual).
    PerpLineCenter { l1: u64, l2: u64, center: u64 },
    /// Two circles intersecting at a right angle:
    /// `r1^2 + r2^2 == dist(c1, c2)^2`.
    PerpCircles {
        c1: u64,
        r1_point: u64,
        c2: u64,
        r2_point: u64,
    },
    /// A line tangent to a circle/arc.
    Tangent {
        l1: u64,
        l2: u64,
        center: u64,
        radius_point: u64,
    },
    /// A point on the (infinite) line `(l1, l2)`.
    PointOnLine { point: u64, l1: u64, l2: u64 },
    /// A point on a circle/arc circumference.
    PointOnRadiusCurve {
        point: u64,
        center: u64,
        radius_point: u64,
    },
    /// A point on a cubic Bezier curve.  The control points are
    /// stored as offsets from the endpoints; `is_line` selects the
    /// straight-segment residual used for curves without control
    /// points.
    PointOnBezier {
        point: u64,
        start: u64,
        end: u64,
        cp1_offset: [f64; 2],
        cp2_offset: [f64; 2],
        is_line: bool,
    },
    /// Equal characteristic lengths.  Each tuple is
    /// `(base_a, base_b, other_a, other_b)` with one residual per
    /// tuple; the caller expands entity characteristic-length pairs
    /// (including the shorter-side fallback) into flat tuples.
    EqualLength { pairs: Vec<[u64; 4]> },
    /// Soft pull of a point towards a target coordinate, scaled by
    /// `weight`.  Used for interactive dragging and for the weak
    /// stabilizer terms.
    Drag {
        point: u64,
        target_x: f64,
        target_y: f64,
        weight: f64,
    },
}

impl ConstraintSpec {
    /// Returns all point ids this constraint couples.
    pub fn referenced_points(&self) -> Vec<u64> {
        let mut ids = Vec::new();
        let mut push = |id: u64| {
            if !ids.contains(&id) {
                ids.push(id);
            }
        };
        match self {
            ConstraintSpec::Horizontal { p1, p2 }
            | ConstraintSpec::Vertical { p1, p2 }
            | ConstraintSpec::Coincident { p1, p2 }
            | ConstraintSpec::Distance { p1, p2, .. } => {
                push(*p1);
                push(*p2);
            }
            ConstraintSpec::AspectRatio { p1, p2, p3, p4, .. } => {
                push(*p1);
                push(*p2);
                push(*p3);
                push(*p4);
            }
            ConstraintSpec::Angle {
                p1,
                p2,
                p3,
                p4,
                e1_far,
                e2_far,
                ..
            } => {
                push(*p1);
                push(*p2);
                push(*p3);
                push(*p4);
                if let Some(id) = e1_far {
                    push(*id);
                }
                if let Some(id) = e2_far {
                    push(*id);
                }
            }
            ConstraintSpec::Collinear { p1, p2, p3 } => {
                push(*p1);
                push(*p2);
                push(*p3);
            }
            ConstraintSpec::EqualDistance { p1, p2, p3, p4 } => {
                push(*p1);
                push(*p2);
                push(*p3);
                push(*p4);
            }
            ConstraintSpec::Parallelogram {
                p_origin,
                p_width,
                p_height,
                p4,
            } => {
                push(*p_origin);
                push(*p_width);
                push(*p_height);
                push(*p4);
            }
            ConstraintSpec::Rotational { center, p1, p2, .. } => {
                push(*center);
                push(*p1);
                push(*p2);
            }
            ConstraintSpec::SymmetryPoint { p1, p2, center } => {
                push(*p1);
                push(*p2);
                push(*center);
            }
            ConstraintSpec::SymmetryLine { p1, p2, l1, l2 } => {
                push(*p1);
                push(*p2);
                push(*l1);
                push(*l2);
            }
            ConstraintSpec::Radius {
                center,
                radius_point,
                ..
            }
            | ConstraintSpec::Diameter {
                center,
                radius_point,
                ..
            } => {
                push(*center);
                push(*radius_point);
            }
            ConstraintSpec::PerpLineLine { a1, a2, b1, b2 } => {
                push(*a1);
                push(*a2);
                push(*b1);
                push(*b2);
            }
            ConstraintSpec::PerpLineCenter { l1, l2, center } => {
                push(*l1);
                push(*l2);
                push(*center);
            }
            ConstraintSpec::PerpCircles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                push(*c1);
                push(*r1_point);
                push(*c2);
                push(*r2_point);
            }
            ConstraintSpec::Tangent {
                l1,
                l2,
                center,
                radius_point,
            } => {
                push(*l1);
                push(*l2);
                push(*center);
                push(*radius_point);
            }
            ConstraintSpec::PointOnLine { point, l1, l2 } => {
                push(*point);
                push(*l1);
                push(*l2);
            }
            ConstraintSpec::PointOnRadiusCurve {
                point,
                center,
                radius_point,
            } => {
                push(*point);
                push(*center);
                push(*radius_point);
            }
            ConstraintSpec::PointOnBezier {
                point, start, end, ..
            } => {
                push(*point);
                push(*start);
                push(*end);
            }
            ConstraintSpec::EqualLength { pairs } => {
                for pair in pairs {
                    for id in pair {
                        push(*id);
                    }
                }
            }
            ConstraintSpec::Drag { point, .. } => push(*point),
        }
        ids
    }

    /// Returns the number of scalar residuals this constraint
    /// contributes.
    pub fn residual_count(&self) -> usize {
        match self {
            ConstraintSpec::Horizontal { .. }
            | ConstraintSpec::Vertical { .. }
            | ConstraintSpec::Distance { .. }
            | ConstraintSpec::AspectRatio { .. }
            | ConstraintSpec::Angle { .. }
            | ConstraintSpec::Collinear { .. }
            | ConstraintSpec::EqualDistance { .. }
            | ConstraintSpec::Radius { .. }
            | ConstraintSpec::Diameter { .. }
            | ConstraintSpec::PerpLineLine { .. }
            | ConstraintSpec::PerpLineCenter { .. }
            | ConstraintSpec::PerpCircles { .. }
            | ConstraintSpec::Tangent { .. }
            | ConstraintSpec::PointOnLine { .. }
            | ConstraintSpec::PointOnRadiusCurve { .. }
            | ConstraintSpec::PointOnBezier { .. } => 1,
            ConstraintSpec::Coincident { .. }
            | ConstraintSpec::Parallelogram { .. }
            | ConstraintSpec::Rotational { .. }
            | ConstraintSpec::SymmetryPoint { .. }
            | ConstraintSpec::SymmetryLine { .. }
            | ConstraintSpec::Drag { .. } => 2,
            ConstraintSpec::EqualLength { pairs } => pairs.len(),
        }
    }
}
