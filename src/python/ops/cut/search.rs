use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::cut;
use crate::ops::cut::ToolPose;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::types::Point;

#[gen_stub_pyclass(module = "raygeo.ops.cut.search")]
#[pyclass(name = "ToolPose", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyToolPose {
    #[pyo3(get)]
    pub pos: (f64, f64),
    #[pyo3(get)]
    pub heading: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyToolPose {
    #[new]
    pub fn new(pos: (f64, f64), heading: f64) -> Self {
        PyToolPose { pos, heading }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "ToolPose(pos=({:.3},{:.3}), heading={:.3})",
            self.pos.0, self.pos.1, self.heading,
        )
    }
}

/// Walk the frontier forward from ``start_pos``, skipping the closest
/// vertex.  Returns the first vertex whose outward cut-area probe
/// falls in ``[min, max]``.
///
/// :param cleared: ``ClearedArea`` instance.
/// :param start_pos: Seed position ``(x, y)``.
/// :param radius: Disk radius (mm).
/// :param step_length: Forward step distance (mm) for the probe.
/// :param min_cut_area: Minimum cut area (mm²).
/// :param max_cut_area: Maximum cut area (mm²), e.g. ``float("inf")``.
/// :returns: ``ToolPose`` or ``None``.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.search")]
#[pyfunction(name = "search_frontier_engagement")]
fn search_frontier_engagement_py(
    cleared: &PyClearedArea,
    start: &PyToolPose,
    radius: f64,
    step_length: f64,
    min_cut_area: f64,
    max_cut_area: f64,
) -> Option<PyToolPose> {
    let r = cut::search_frontier_engagement(
        &cleared.inner,
        ToolPose {
            pos: Point::new(start.pos.0, start.pos.1),
            heading: start.heading,
        },
        radius,
        step_length,
        min_cut_area,
        max_cut_area,
    )?;
    Some(PyToolPose {
        pos: (r.pos.x, r.pos.y),
        heading: r.heading,
    })
}

/// Walk the frontier backward from ``start_pos``, skipping the closest
/// vertex.  Returns the first vertex (going backward) whose outward
/// cut-area probe is at least ``min_cut_area``.
///
/// :param cleared: ``ClearedArea`` instance.
/// :param start_pos: Seed position ``(x, y)``.
/// :param heading: Current tool heading (radians).
/// :param radius: Disk radius (mm).
/// :param step_length: Forward step distance (mm).
/// :param min_cut_area: Minimum cut area (mm²).
/// :returns: ``ToolPose`` or ``None``.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.search")]
#[pyfunction(name = "search_reengagement")]
fn search_reengagement_py(
    cleared: &PyClearedArea,
    start: &PyToolPose,
    radius: f64,
    step_length: f64,
    min_cut_area: f64,
) -> Option<PyToolPose> {
    let r = cut::search_reengagement(
        &cleared.inner,
        ToolPose {
            pos: Point::new(start.pos.0, start.pos.1),
            heading: start.heading,
        },
        radius,
        step_length,
        min_cut_area,
    )?;
    Some(PyToolPose {
        pos: (r.pos.x, r.pos.y),
        heading: r.heading,
    })
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = cut_mod.py();
    let m = PyModule::new(py, "search")?;

    m.add_class::<PyToolPose>()?;
    m.add_function(wrap_pyfunction!(search_frontier_engagement_py, &m)?)?;
    m.add_function(wrap_pyfunction!(search_reengagement_py, &m)?)?;

    cut_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.cut.search", &m)?;

    Ok(())
}
