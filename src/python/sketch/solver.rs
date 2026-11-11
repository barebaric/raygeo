//! Python bindings for the sketch constraint solver.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::sketch::constraint::ConstraintSpec;
use crate::sketch::solver::SketchSolver;

/// Definition of one sketch point.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch",
    name = "PointDef"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointDef {
    /// Unique point id, stable across solves.
    #[pyo3(get)]
    pub id: u64,
    /// Current X coordinate.
    #[pyo3(get)]
    pub x: f64,
    /// Current Y coordinate.
    #[pyo3(get)]
    pub y: f64,
    /// Whether the point is pinned and never moves.
    #[pyo3(get)]
    pub fixed: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointDef {
    /// Creates a point definition.
    #[new]
    fn new(id: u64, x: f64, y: f64, fixed: bool) -> Self {
        PyPointDef { id, x, y, fixed }
    }
}

/// One constraint of a sketch problem.  Point ids refer to
/// [`PointDef`] ids; dimensional values must already be resolved
/// (parameter expressions evaluated) by the caller.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch",
    name = "Constraint"
)]
#[derive(Clone, PartialEq)]
pub enum PyConstraint {
    /// Two points share their Y coordinate.
    Horizontal { p1: u64, p2: u64 },
    /// Two points share their X coordinate.
    Vertical { p1: u64, p2: u64 },
    /// Two points are at the same location.
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
    /// Clockwise angle in degrees between two lines at their
    /// intersection; `e1_far`/`e2_far` select the ray of each line.
    Angle {
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        e1_far: Option<u64>,
        e2_far: Option<u64>,
        value_deg: f64,
    },
    /// Three points on one line.
    Collinear { p1: u64, p2: u64, p3: u64 },
    /// `dist(p1, p2) == dist(p3, p4)`.
    EqualDistance { p1: u64, p2: u64, p3: u64, p4: u64 },
    /// `(p_width - p_origin) == (p4 - p_height)`.
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
    /// `center` is the midpoint of `p1` and `p2`.
    SymmetryPoint { p1: u64, p2: u64, center: u64 },
    /// `p1` and `p2` are mirrored across the line `(l1, l2)`.
    SymmetryLine { p1: u64, p2: u64, l1: u64, l2: u64 },
    /// Radius of a circle/arc.
    Radius {
        center: u64,
        radius_point: u64,
        value: f64,
    },
    /// Diameter of a circle.
    Diameter {
        center: u64,
        radius_point: u64,
        value: f64,
    },
    /// Two lines perpendicular.
    PerpLineLine { a1: u64, a2: u64, b1: u64, b2: u64 },
    /// A line passing through a circle/arc center.
    PerpLineCenter { l1: u64, l2: u64, center: u64 },
    /// Two circles intersecting at a right angle.
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
    /// A point on the infinite line `(l1, l2)`.
    PointOnLine { point: u64, l1: u64, l2: u64 },
    /// A point on a circle/arc circumference.
    PointOnRadiusCurve {
        point: u64,
        center: u64,
        radius_point: u64,
    },
    /// A point on a cubic Bezier; control points as endpoint offsets,
    /// `is_line` selects the straight-segment residual.
    PointOnBezier {
        point: u64,
        start: u64,
        end: u64,
        cp1_offset: (f64, f64),
        cp2_offset: (f64, f64),
        is_line: bool,
    },
    /// Equal characteristic lengths, one residual per
    /// `(base_a, base_b, other_a, other_b)` tuple.
    EqualLength { pairs: Vec<[u64; 4]> },
    /// Soft pull of a point towards a target, scaled by `weight`.
    Drag {
        point: u64,
        target_x: f64,
        target_y: f64,
        weight: f64,
    },
}

impl From<&PyConstraint> for ConstraintSpec {
    fn from(value: &PyConstraint) -> Self {
        match value {
            PyConstraint::Horizontal { p1, p2 } => {
                ConstraintSpec::Horizontal { p1: *p1, p2: *p2 }
            }
            PyConstraint::Vertical { p1, p2 } => {
                ConstraintSpec::Vertical { p1: *p1, p2: *p2 }
            }
            PyConstraint::Coincident { p1, p2 } => {
                ConstraintSpec::Coincident { p1: *p1, p2: *p2 }
            }
            PyConstraint::Distance { p1, p2, value } => {
                ConstraintSpec::Distance {
                    p1: *p1,
                    p2: *p2,
                    value: *value,
                }
            }
            PyConstraint::AspectRatio {
                p1,
                p2,
                p3,
                p4,
                ratio,
            } => ConstraintSpec::AspectRatio {
                p1: *p1,
                p2: *p2,
                p3: *p3,
                p4: *p4,
                ratio: *ratio,
            },
            PyConstraint::Angle {
                p1,
                p2,
                p3,
                p4,
                e1_far,
                e2_far,
                value_deg,
            } => ConstraintSpec::Angle {
                p1: *p1,
                p2: *p2,
                p3: *p3,
                p4: *p4,
                e1_far: *e1_far,
                e2_far: *e2_far,
                value_deg: *value_deg,
            },
            PyConstraint::Collinear { p1, p2, p3 } => {
                ConstraintSpec::Collinear {
                    p1: *p1,
                    p2: *p2,
                    p3: *p3,
                }
            }
            PyConstraint::EqualDistance { p1, p2, p3, p4 } => {
                ConstraintSpec::EqualDistance {
                    p1: *p1,
                    p2: *p2,
                    p3: *p3,
                    p4: *p4,
                }
            }
            PyConstraint::Parallelogram {
                p_origin,
                p_width,
                p_height,
                p4,
            } => ConstraintSpec::Parallelogram {
                p_origin: *p_origin,
                p_width: *p_width,
                p_height: *p_height,
                p4: *p4,
            },
            PyConstraint::Rotational {
                center,
                p1,
                p2,
                angle_rad,
            } => ConstraintSpec::Rotational {
                center: *center,
                p1: *p1,
                p2: *p2,
                angle_rad: *angle_rad,
            },
            PyConstraint::SymmetryPoint { p1, p2, center } => {
                ConstraintSpec::SymmetryPoint {
                    p1: *p1,
                    p2: *p2,
                    center: *center,
                }
            }
            PyConstraint::SymmetryLine { p1, p2, l1, l2 } => {
                ConstraintSpec::SymmetryLine {
                    p1: *p1,
                    p2: *p2,
                    l1: *l1,
                    l2: *l2,
                }
            }
            PyConstraint::Radius {
                center,
                radius_point,
                value,
            } => ConstraintSpec::Radius {
                center: *center,
                radius_point: *radius_point,
                value: *value,
            },
            PyConstraint::Diameter {
                center,
                radius_point,
                value,
            } => ConstraintSpec::Diameter {
                center: *center,
                radius_point: *radius_point,
                value: *value,
            },
            PyConstraint::PerpLineLine { a1, a2, b1, b2 } => {
                ConstraintSpec::PerpLineLine {
                    a1: *a1,
                    a2: *a2,
                    b1: *b1,
                    b2: *b2,
                }
            }
            PyConstraint::PerpLineCenter { l1, l2, center } => {
                ConstraintSpec::PerpLineCenter {
                    l1: *l1,
                    l2: *l2,
                    center: *center,
                }
            }
            PyConstraint::PerpCircles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => ConstraintSpec::PerpCircles {
                c1: *c1,
                r1_point: *r1_point,
                c2: *c2,
                r2_point: *r2_point,
            },
            PyConstraint::Tangent {
                l1,
                l2,
                center,
                radius_point,
            } => ConstraintSpec::Tangent {
                l1: *l1,
                l2: *l2,
                center: *center,
                radius_point: *radius_point,
            },
            PyConstraint::PointOnLine { point, l1, l2 } => {
                ConstraintSpec::PointOnLine {
                    point: *point,
                    l1: *l1,
                    l2: *l2,
                }
            }
            PyConstraint::PointOnRadiusCurve {
                point,
                center,
                radius_point,
            } => ConstraintSpec::PointOnRadiusCurve {
                point: *point,
                center: *center,
                radius_point: *radius_point,
            },
            PyConstraint::PointOnBezier {
                point,
                start,
                end,
                cp1_offset,
                cp2_offset,
                is_line,
            } => ConstraintSpec::PointOnBezier {
                point: *point,
                start: *start,
                end: *end,
                cp1_offset: [cp1_offset.0, cp1_offset.1],
                cp2_offset: [cp2_offset.0, cp2_offset.1],
                is_line: *is_line,
            },
            PyConstraint::EqualLength { pairs } => {
                ConstraintSpec::EqualLength {
                    pairs: pairs.clone(),
                }
            }
            PyConstraint::Drag {
                point,
                target_x,
                target_y,
                weight,
            } => ConstraintSpec::Drag {
                point: *point,
                target_x: *target_x,
                target_y: *target_y,
                weight: *weight,
            },
        }
    }
}

/// Trust-region least-squares solver for sketch constraint systems.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch", name = "Solver", skip_from_py_object)]
pub struct PySketchSolver {
    inner: SketchSolver,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySketchSolver {
    /// Builds a solver from point definitions, hard constraints,
    /// auxiliary (weak) constraints, and an optional point filter.
    #[new]
    #[pyo3(signature = (points, hard, auxiliary = None, point_filter = None))]
    fn new(
        points: Vec<PyPointDef>,
        hard: Vec<PyConstraint>,
        auxiliary: Option<Vec<PyConstraint>>,
        point_filter: Option<Vec<u64>>,
    ) -> Self {
        let point_defs =
            points.iter().map(|p| (p.id, p.x, p.y, p.fixed)).collect();
        let hard_specs: Vec<ConstraintSpec> =
            hard.iter().map(ConstraintSpec::from).collect();
        let aux_specs: Vec<ConstraintSpec> = auxiliary
            .unwrap_or_default()
            .iter()
            .map(ConstraintSpec::from)
            .collect();
        PySketchSolver {
            inner: SketchSolver::new(
                point_defs,
                hard_specs,
                aux_specs,
                point_filter,
            ),
        }
    }

    /// Runs the solve.  Returns True when all components converged.
    ///
    /// :param tolerance: Cost threshold and ftol of the optimizer.
    /// :param update_dof: Whether to re-analyze per-point constrained
    ///     flags after a successful solve.
    fn solve(&mut self, tolerance: f64, update_dof: bool) -> bool {
        self.inner.solve(tolerance, update_dof)
    }

    /// Indices of hard constraints whose residual exceeds
    /// `threshold` at the final positions.
    fn conflicting(&self, threshold: f64) -> Vec<usize> {
        self.inner.conflicting(threshold)
    }

    /// Final `(x, y)` positions, aligned with the input point order.
    fn positions(&self) -> Vec<(f64, f64)> {
        self.inner.positions()
    }

    /// Per-point fully-constrained flags, aligned with the input
    /// point order.
    fn constrained(&self) -> Vec<bool> {
        self.inner.constrained_flags()
    }

    /// Maximum absolute residual per hard constraint.
    fn max_residuals(&self) -> Vec<f64> {
        self.inner.max_residuals()
    }
}
