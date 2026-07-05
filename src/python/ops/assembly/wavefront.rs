use crate::ops::assembly::wavefront;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "wavefront")?;
    register_functions!(m, adaptive_wavefronts_py,);
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.wavefront", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def adaptive_wavefronts(
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        z: float = 0.0,
        area_tolerance: float = 1.0,
        precision: float = 0.0,
        cut_feed_rate: int = 1200,
        cut_power: float = 1.0,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Inside-out adaptive wavefronts.

        Starting from the *cleared* state, each iteration expands the
        cleared boundary outward by *step_over*, clips to the valid tool
        area (pocket boundary offset inward by *tool_radius*, with
        islands excluded), and adds the result back to *cleared*.
        The loop terminates when the newly added area drops below
        *area_tolerance*.

        Each ring fragment is emitted as ``MoveTo`` + ``LineTo`` at
        height *z* with *cut_feed_rate* applied.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial expansion per iteration (default 2.0).
        :param z: Z height for generated commands (default 0.0).
        :param area_tolerance: Minimum area increase to continue (default 1.0).
        :param precision: Edge tolerance for frontier simplification and vertex
                          resampling; smaller values produce denser edges
                          (default 0.0 = use internal default).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param cut_power: Laser power for cutting moves (0.0-1.0, default 1.0).
        :returns: An :class:`AssemblyResult` with wavefront cutting commands.
        """
    "#,
    module = "raygeo.ops.assembly.wavefront"
)]
#[pyfunction(name = "adaptive_wavefronts")]
#[pyo3(signature = (
    cleared,
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    z = 0.0,
    area_tolerance = 1.0,
    precision = 0.0,
    cut_feed_rate = 1200,
    cut_power = 1.0,
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_wavefronts_py(
    cleared: &mut PyClearedArea,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    z: f64,
    area_tolerance: f64,
    precision: f64,
    cut_feed_rate: i32,
    cut_power: f64,
) -> PyResult<PyAssemblyResult> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = wavefront::AdaptiveWavefrontOptions {
        pocket_boundary: boundary,
        islands: islands_pts,
        tool_radius,
        step_over,
        z,
        area_tolerance,
        precision,
    };

    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let result =
        wavefront::adaptive_wavefronts(&mut cleared.inner, &opts, &cut_state)?;

    Ok(PyAssemblyResult::from_inner(result))
}
