use crate::geo::algo::helix::HelixDirection;
use crate::ops::assembly::toroid::{self, ToroidOptions};
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "toroid")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_toroid_py, m.clone())?)?;
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
        carrier: collections.abc.Sequence[tuple[float, float]],
        tool_radius: float,
        step_distance: float,
        z: float,
        direction: str = "CW",
        angular_step: float = 0.1,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Generate a toroidal (trochoidal) path along a carrier.

        Produces a trochoidal looping path that follows the *carrier*
        polyline, clearing a slot of width *tool_radius*.

        :param carrier: List of ``(x, y)`` waypoints defining the slot axis.
        :param tool_radius: Tool radius in mm.
        :param step_distance: Forward advance per trochoid loop.
        :param z: Cutting Z height.
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
    carrier,
    tool_radius,
    step_distance,
    z,
    direction = "CW",
    angular_step = 0.1,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_toroid_py(
    carrier: Vec<(f64, f64)>,
    tool_radius: f64,
    step_distance: f64,
    z: f64,
    direction: &str,
    angular_step: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyAssemblyResult {
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
        step_distance,
        z,
        direction: dir,
        angular_step,
    };

    let result = toroid::generate_toroid(&opts, &cut_state);
    PyAssemblyResult::from_inner(result)
}
