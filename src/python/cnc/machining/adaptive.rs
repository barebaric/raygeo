use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::machining::adaptive::{self, ClearingWorkplanOptions};
use crate::types::Point;

pub(crate) fn register(machining_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = machining_mod.py();
    let m = PyModule::new(py, "adaptive")?;
    register_functions!(m, build_clearing_workplan_py,);
    machining_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining.adaptive", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def build_clearing_workplan(
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        step_length: float = 0.6,
        target_z: float = -5.0,
        safe_z: float = 2.0,
        wall_margin: float = 0.0,
        safe_margin: float = 1.0,
        stock_to_leave: float = 0.0,
        plunge_pitch: float = 1.0,
        angular_step: float = 0.1,
        area_tolerance: float = 1.0,
        max_deflection_deg: float = 30.0,
        finishing: bool = False,
    ) -> list[dict]:
        """Build a clearing workplan for the given pocket.

        Produces entry steps for the largest wide region, an AdaptiveClear
        step covering the whole pocket, narrow-passage-specific steps
        (ToroidalClear or Slot) for each classified passage, and an optional
        ProfileInner finishing pass.

        Combine with :class:`raygeo.cnc.machining.plan.Workplan`
        to turn the steps into a toolpath.

        :param pocket_boundary: Outer boundary as [(x, y), ...].
        :param islands: List of island polygons (default None).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over (default 2.0).
        :param step_length: Forward step length (default 0.6).
        :param target_z: Target cutting depth (default -5.0).
        :param safe_z: Safe Z height (default 2.0).
        :param wall_margin: Wall margin (default 0.0).
        :param safe_margin: Safety margin from tool edge (default 1.0).
        :param stock_to_leave: Stock to leave for finishing (default 0.0).
        :param plunge_pitch: Helix pitch per revolution (default 1.0).
        :param angular_step: Angular step in radians (default 0.1).
        :param area_tolerance: Convergence area tolerance (default 1.0).
        :param max_deflection_deg: Max deflection per step in degrees (default 30.0).
        :param finishing: Whether to add a ProfileInner finishing pass (default False).
        :returns: List of WorkplanStep dicts with a "kind" key.
        """
    "#,
    module = "raygeo.cnc.machining.adaptive"
)]
#[pyfunction(name = "build_clearing_workplan")]
#[pyo3(signature = (
    pocket_boundary,
    islands = None,
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
#[allow(clippy::too_many_arguments)]
fn build_clearing_workplan_py(
    py: Python<'_>,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
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
) -> PyResult<Vec<Bound<'_, PyDict>>> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_vec: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = ClearingWorkplanOptions {
        pocket_boundary: boundary,
        islands: islands_vec,
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

    let steps = adaptive::build_clearing_workplan(&opts)?;
    let mut result: Vec<Bound<'_, PyDict>> = Vec::with_capacity(steps.len());
    for step in &steps {
        result.push(super::plan::step_to_dict(py, step)?);
    }
    Ok(result)
}
