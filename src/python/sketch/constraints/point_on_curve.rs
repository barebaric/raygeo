//! Bindings for the point-on-curve constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::point_on_curve::PointOnCurveConstraint as CoreConstraint;

/// Enforces a point lies on a cubic Bezier curve.  The control
/// points are stored as offsets from their owning endpoints; curves
/// without control points use the straight-segment residual
/// (`is_line`).
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_curve",
    name = "PointOnCurveConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointOnCurveConstraint {
    /// The constrained point.
    #[pyo3(get)]
    pub point: u64,
    /// Curve start point.
    #[pyo3(get)]
    pub start: u64,
    /// Curve end point.
    #[pyo3(get)]
    pub end: u64,
    /// First control point as offset from the start point.
    #[pyo3(get)]
    pub cp1_offset: (f64, f64),
    /// Second control point as offset from the end point.
    #[pyo3(get)]
    pub cp2_offset: (f64, f64),
    /// Whether the curve degenerates to a straight segment.
    #[pyo3(get)]
    pub is_line: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointOnCurveConstraint {
    /// Creates the point-on-curve constraint.
    #[new]
    #[pyo3(signature = (point, start, end, cp1_offset, cp2_offset, is_line))]
    fn new(
        point: u64,
        start: u64,
        end: u64,
        cp1_offset: (f64, f64),
        cp2_offset: (f64, f64),
        is_line: bool,
    ) -> Self {
        PyPointOnCurveConstraint {
            point,
            start,
            end,
            cp1_offset,
            cp2_offset,
            is_line,
        }
    }
}

impl PyPointOnCurveConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(
            self.point,
            self.start,
            self.end,
            [self.cp1_offset.0, self.cp1_offset.1],
            [self.cp2_offset.0, self.cp2_offset.1],
            self.is_line,
        )
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPointOnCurveConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.point_on_curve", m)?;
    Ok(())
}
