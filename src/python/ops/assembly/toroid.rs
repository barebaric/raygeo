use crate::geo::algo::helix::HelixDirection;
use crate::ops::assembly::toroid::{self, ToroidOptions, ToroidalClearOptions};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use crate::types::{Point, Point3D};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "toroid")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_toroid_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        generate_toroidal_clear_py,
        m.clone()
    )?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.toroid", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def generate_toroid(
        part: raygeo.Part,
        carrier: collections.abc.Sequence[tuple[float, float]],
        tool_radius: float,
        step_over: float,
        target_z: float,
        direction: str = "CW",
        angular_step: float = 0.1,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Generate a toroidal (trochoidal) path along a carrier.

        Produces a trochoidal looping path that follows the *carrier*
        polyline, clearing a slot of width *tool_radius*.

        :param carrier: List of ``(x, y)`` waypoints defining the slot axis.
        :param tool_radius: Tool radius in mm.
        :param step_over: Forward advance per trochoid loop.
        :param target_z: Cutting Z height.
        :param direction: ``"CW"`` or ``"CCW"`` (default ``"CW"``).
        :param angular_step: Angular step in radians (default 0.1).
        :param state: Optional machine state to apply before the path.
        :returns: An :class:`AssemblyResult` with the toroidal path.
        """
    "#,
    module = "raygeo.ops.assembly.toroid"
)]
#[pyfunction(name = "generate_toroid")]
#[pyo3(signature = (
    part,
    carrier,
    tool_radius,
    step_over,
    target_z,
    direction = "CW",
    angular_step = 0.1,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_toroid_py(
    part: &crate::python::part::PyPart,
    carrier: Vec<(f64, f64)>,
    tool_radius: f64,
    step_over: f64,
    target_z: f64,
    direction: &str,
    angular_step: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let dir = match direction {
        "CW" => HelixDirection::Cw,
        "CCW" => HelixDirection::Ccw,
        _ => HelixDirection::Cw,
    };

    let cut_state = match state {
        Some(ref s) => s.borrow().0.clone(),
        None => State::default(),
    };

    let carrier_pts: Vec<Point> =
        carrier.into_iter().map(|(x, y)| Point::new(x, y)).collect();

    let opts = ToroidOptions {
        carrier: carrier_pts,
        tool_radius,
        step_over,
        target_z,
        direction: dir,
        angular_step,
    };

    let mut trace = Tracelet::new();
    let meta =
        toroid::generate_toroid(&part.inner, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def generate_toroidal_clear(
        part: raygeo.Part,
        carrier: collections.abc.Sequence[tuple[float, float]],
        start: tuple[float, float, float],
        target_z: float,
        tool_radius: float,
        step_over: float,
        max_ramp_angle_deg: float = 5.0,
        direction: str = "CW",
        angular_step: float = 0.1,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Generate a ramp-down toroidal clear path along a carrier.

        Descends Z linearly along the carrier's arc-length at a slope
        capped by ``max_ramp_angle_deg``, zig-zagging back-and-forth along
        the carrier until ``target_z`` is reached, then emits one final
        full forward pass at constant ``target_z``.

        :param carrier: 2D polyline ``(x, y)`` waypoints defining the slot axis.
        :param start: ``(x, y, z)`` entry point; ``x, y`` should match ``carrier[0]``,
            ``z`` is the entry height.
        :param target_z: Final cutting Z height.
        :param tool_radius: Tool radius in mm.
        :param step_over: Forward advance per trochoid loop.
        :param max_ramp_angle_deg: Maximum descent angle vs. the XY plane (default 5°).
        :param direction: ``"CW"`` or ``"CCW"`` (default ``"CW"``).
        :param angular_step: Angular step in radians (default 0.1).
        :param state: Optional machine state.
        :returns: An :class:`AssemblyResult` with the ramp-down + flat-final toroidal path.
        """
    "#,
    module = "raygeo.ops.assembly.toroid"
)]
#[pyfunction(name = "generate_toroidal_clear")]
#[pyo3(signature = (
    part,
    carrier,
    start,
    target_z,
    tool_radius,
    step_over,
    max_ramp_angle_deg = 5.0,
    direction = "CW",
    angular_step = 0.1,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_toroidal_clear_py(
    part: &crate::python::part::PyPart,
    carrier: Vec<(f64, f64)>,
    start: (f64, f64, f64),
    target_z: f64,
    tool_radius: f64,
    step_over: f64,
    max_ramp_angle_deg: f64,
    direction: &str,
    angular_step: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let dir = match direction {
        "CW" => HelixDirection::Cw,
        "CCW" => HelixDirection::Ccw,
        _ => HelixDirection::Cw,
    };

    let cut_state = match state {
        Some(ref s) => s.borrow().0.clone(),
        None => State::default(),
    };

    let carrier_pts: Vec<Point> =
        carrier.into_iter().map(|(x, y)| Point::new(x, y)).collect();

    let opts = ToroidalClearOptions {
        carrier: carrier_pts,
        start: Point3D::new(start.0, start.1, start.2),
        target_z,
        tool_radius,
        step_over,
        max_ramp_angle_deg,
        direction: dir,
        angular_step,
    };

    let mut trace = Tracelet::new();
    let meta = toroid::generate_toroidal_clear(
        &part.inner,
        &mut trace,
        &opts,
        &cut_state,
    )?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}
