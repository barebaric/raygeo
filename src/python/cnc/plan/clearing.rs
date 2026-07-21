pyo3_stub_gen::module_doc!("raygeo.cnc.plan.clearing", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC clearing plan builder.

Produces a :class:`~raygeo.cnc.plan.Plan` organised as a BFS traversal
of the pocket's region/passage graph.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::plan::clearing::{self, ClearingWorkplanOptions};
use crate::python::ops::part::part::PyPart;

pub(crate) fn register(plan_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = plan_mod.py();
    let m = PyModule::new(py, "clearing")?;
    register_functions!(m, plan_clearing_py,);
    plan_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan.clearing", &m)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[gen_stub_pyfunction(module = "raygeo.cnc.plan.clearing")]
#[pyfunction(name = "plan_clearing")]
#[pyo3(signature = (
    part,
    face_id = "",
    tool_radius = 3.0,
    step_over = 2.0,
    step_length = 0.6,
    target_z = -5.0,
    safe_z = 2.0,
    wall_margin = 0.0,
    safe_margin = 1.0,
    stock_to_leave = 0.0,
    plunge_pitch = 1.0,
    angular_step = 0.1,
    area_tolerance = 1.0,
    max_deflection_deg = 30.0,
    finishing = false,
))]
pub fn plan_clearing_py(
    part: &Bound<'_, PyPart>,
    face_id: &str,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    wall_margin: f64,
    safe_margin: f64,
    stock_to_leave: f64,
    plunge_pitch: f64,
    angular_step: f64,
    area_tolerance: f64,
    max_deflection_deg: f64,
    finishing: bool,
) -> PyResult<Py<super::PyPlan>> {
    let opts = ClearingWorkplanOptions {
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        wall_margin,
        safe_margin,
        stock_to_leave,
        plunge_pitch,
        angular_step,
        area_tolerance,
        max_deflection_deg,
        finishing,
    };

    let plan = clearing::plan_clearing(&part.borrow().inner, face_id, &opts)?;
    Py::new(part.py(), super::PyPlan { inner: plan })
}
