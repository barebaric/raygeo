pyo3_stub_gen::module_doc!("raygeo.cnc.plan.entry", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC entry strategy orchestration.

Plan entry moves to a pocket entry point using the most efficient
assembler (helix, spiral, ramp or toroidal-clear).
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::plan::entry::{self, EntryWorkplanOptions};
use crate::ops::feature::region::Region;
use crate::types::{Point, Polygon};

pub(crate) fn register(plan_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = plan_mod.py();
    let m = PyModule::new(py, "entry")?;
    register_functions!(m, plan_entry_py,);
    plan_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan.entry", &m)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[gen_stub_pyfunction(module = "raygeo.cnc.plan.entry")]
#[pyfunction(name = "plan_entry")]
#[pyo3(signature = (
    region_polygon,
    entry_point,
    r_max,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    safe_z = 2.0,
    target_z = -5.0,
    plunge_pitch = 1.0,
    safe_margin = 1.0,
    angular_step = 0.1,
))]
fn plan_entry_py(
    py: Python<'_>,
    region_polygon: Vec<(f64, f64)>,
    entry_point: (f64, f64),
    r_max: f64,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    safe_z: f64,
    target_z: f64,
    plunge_pitch: f64,
    safe_margin: f64,
    angular_step: f64,
) -> PyResult<Vec<Py<super::PyPlanStep>>> {
    let polygon: Vec<Point> = region_polygon
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_vec: Vec<Polygon> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let region = Region {
        polygon,
        area: 0.0,
        entry_pt: Point::new(entry_point.0, entry_point.1),
        r_max,
    };

    let opts = EntryWorkplanOptions {
        islands: islands_vec,
        tool_radius,
        step_over,
        safe_z,
        target_z,
        plunge_pitch,
        safe_margin,
        angular_step,
    };

    let steps = entry::plan_entry(&region, &opts, "")?;
    let mut result = Vec::with_capacity(steps.len());
    for step in steps {
        result.push(Py::new(py, super::PyPlanStep { inner: step })?);
    }
    Ok(result)
}
