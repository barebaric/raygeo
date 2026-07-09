use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::machining::wavefront::{self, WavefrontWorkplanOptions};
use crate::types::Point;

pub(crate) fn register(machining_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = machining_mod.py();
    let m = PyModule::new(py, "wavefront")?;
    register_functions!(m, build_wavefront_workplan_py,);
    machining_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining.wavefront", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def build_wavefront_workplan(
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        target_z: float = -5.0,
        safe_margin: float = 1.0,
        angular_step: float = 0.1,
        area_tolerance: float = 1.0,
        precision: float = 0.0,
    ) -> list[dict]:
        """Build a spiral-seed + wavefront-expansion workplan.

        Finds the largest inscribed circle, emits a ``FlatSpiral`` step
        that seeds the pocket centre with a cleared disk, then a
        ``Wavefront`` step that expands outward. No helical plunge is
        produced: the spiral disk already covers the area the helix used
        to, so the wavefront seed is identical to the legacy
        ``adaptive_entry`` path.

        Combine with :func:`raygeo.cnc.machining.plan.execute_workplan`
        to turn the steps into a toolpath.

        :param pocket_boundary: Outer boundary as [(x, y), ...].
        :param islands: List of island polygons (default None).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over (default 2.0).
        :param target_z: Target cutting depth (default -5.0).
        :param safe_margin: Safety margin from tool edge (default 1.0).
        :param angular_step: Angular step in radians (default 0.1).
        :param area_tolerance: Convergence area tolerance (default 1.0).
        :param precision: Vertex resampling spacing, 0 to disable (default 0.0).
        :returns: List of WorkplanStep dicts (``FlatSpiral`` then ``Wavefront``).
        """
    "#,
    module = "raygeo.cnc.machining.wavefront"
)]
#[pyfunction(name = "build_wavefront_workplan")]
#[pyo3(signature = (
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    target_z = -5.0,
    safe_margin = 1.0,
    angular_step = 0.1,
    area_tolerance = 1.0,
    precision = 0.0,
))]
#[allow(clippy::too_many_arguments)]
fn build_wavefront_workplan_py(
    py: Python<'_>,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    target_z: f64,
    safe_margin: f64,
    angular_step: f64,
    area_tolerance: f64,
    precision: f64,
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

    let opts = WavefrontWorkplanOptions {
        pocket_boundary: boundary,
        islands: islands_vec,
        tool_radius,
        step_over,
        target_z,
        safe_margin,
        angular_step,
        area_tolerance,
        precision,
    };

    let steps = wavefront::build_wavefront_workplan(&opts)?;
    let mut result: Vec<Bound<'_, PyDict>> = Vec::with_capacity(steps.len());
    for step in &steps {
        result.push(super::plan::step_to_dict(py, step)?);
    }
    Ok(result)
}
