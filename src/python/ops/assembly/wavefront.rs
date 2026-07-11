use crate::ops::assembly::wavefront;
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
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
        part: raygeo.ops.cut.Part,
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        z: float = 0.0,
        area_tolerance: float = 1.0,
        precision: float = 0.0,
        cut_feed_rate: int = 1200,
        cut_power: float = 1.0,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Inside-out adaptive wavefronts.

        Starting from the cleared state inside *part*, each iteration
        expands the cleared boundary outward by *step_over*, clips to
        the valid tool area (pocket boundary offset inward by
        *tool_radius*, with islands excluded), and adds the result
        back to the part's cleared state.  The loop terminates when
        the newly added area drops below *area_tolerance*.

        Each ring fragment is emitted as ``MoveTo`` + ``LineTo`` at
        height *z* with *cut_feed_rate* applied.

        :param part: The part whose ``cleared`` field tracks accumulated
                     workpiece state and whose geometry defines the
                     pocket boundary and islands.
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
    part,
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
    part: &mut crate::python::ops::cut::part::PyPart,
    tool_radius: f64,
    step_over: f64,
    z: f64,
    area_tolerance: f64,
    precision: f64,
    cut_feed_rate: i32,
    cut_power: f64,
) -> PyResult<PyAssemblyResult> {
    let opts = wavefront::AdaptiveWavefrontOptions {
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

    let mut trace = Tracelet::new();
    let meta = wavefront::adaptive_wavefronts(
        &mut part.inner,
        &mut trace,
        &opts,
        &cut_state,
    )?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}
