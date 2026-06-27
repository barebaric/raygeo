use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::cut;
use crate::python::geo::flex_point::polygons_from_tuples;
use crate::types::Point;

/// Bracket of error values for adaptive-stepping interpolation.
///
/// Maintains a min/max bracket around the target cut-area per distance
/// and linearly interpolates to find the steering angle that achieves it.
#[gen_stub_pyclass(module = "raygeo.ops.cut.interp")]
#[pyclass(name = "Interpolation", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyInterpolation {
    pub inner: cut::interp::Interpolation,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyInterpolation {
    /// Create a new empty bracket.
    #[new]
    pub fn new() -> Self {
        PyInterpolation {
            inner: cut::interp::Interpolation::new(),
        }
    }

    /// Minimum steering angle: -π/4.
    pub fn min_angle(&self) -> f64 {
        self.inner.min_angle()
    }

    /// Maximum steering angle: +π/4.
    pub fn max_angle(&self) -> f64 {
        self.inner.max_angle()
    }

    /// Whether a valid bracket around the root exists
    /// (min.error < 0 <= max.error).
    pub fn joint_is_valid(&self) -> bool {
        self.inner.joint_is_valid()
    }

    /// Whether either endpoint was sampled at *pos*.
    pub fn has_pos(&self, pos: (f64, f64)) -> bool {
        self.inner.has_pos(Point::new(pos.0, pos.1))
    }

    /// Clamp *angle* to ±max_deflection.
    pub fn clamp_angle(&self, angle: f64, max_deflection: f64) -> f64 {
        self.inner.clamp_angle(angle, max_deflection)
    }

    /// Linearly interpolate between min and max to find the angle
    /// where error = 0, clamped to [0.2, 0.8] in parameter space.
    pub fn interpolate(&self) -> f64 {
        self.inner.interpolate()
    }

    /// Add a new sample to the bracket.
    ///
    /// Maintains the invariant ``min.error <= max.error`` and keeps
    /// samples closest to zero on each side of the root.
    pub fn add(
        &mut self,
        error: f64,
        angle: f64,
        pos: (f64, f64),
        allow_skip: bool,
        is_conventional: bool,
    ) {
        self.inner.add(
            error,
            angle,
            Point::new(pos.0, pos.1),
            allow_skip,
            is_conventional,
        );
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Check whether *pt* lies in a valid tool area defined by polygon
/// shells and holes.
///
/// CCW-wound polygons are outer shells; CW-wound polygons are holes.
/// A point is valid iff it is inside at least one CCW polygon AND
/// outside all CW polygons.
///
/// :param pt: Query point ``(x, y)``.
/// :param area: List of polygon rings (each a list of ``(x, y)`` tuples).
/// :returns: ``True`` if the point is in a valid region.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.interp")]
#[pyfunction(name = "point_in_valid_area")]
fn point_in_valid_area_py(pt: (f64, f64), area: Vec<Vec<(f64, f64)>>) -> bool {
    let polygons = polygons_from_tuples(area);
    cut::interp::point_in_valid_area(Point::new(pt.0, pt.1), &polygons)
}

/// Rotate a 2D vector by *angle* radians.
///
/// :param v: Vector ``(x, y)``.
/// :param angle: Rotation angle in radians.
/// :returns: Rotated vector ``(x', y')``.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.interp")]
#[pyfunction(name = "rotate")]
fn rotate_py(v: (f64, f64), angle: f64) -> (f64, f64) {
    let r = cut::interp::rotate(Point::new(v.0, v.1), angle);
    (r.x, r.y)
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = cut_mod.py();
    let m = PyModule::new(py, "interp")?;

    m.add_class::<PyInterpolation>()?;
    m.add_function(wrap_pyfunction!(point_in_valid_area_py, &m)?)?;
    m.add_function(wrap_pyfunction!(rotate_py, &m)?)?;

    cut_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.cut.interp", &m)?;

    Ok(())
}
