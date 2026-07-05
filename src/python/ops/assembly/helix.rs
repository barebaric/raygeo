use crate::geo::algo::helix::HelixDirection;
use crate::ops::assembly::helix::{self, HelixOptions};
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "helix")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_helix_py, m.clone())?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.helix", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def generate_helix(
        center: tuple[float, float],
        start_radius: float,
        z_start: float,
        z_end: float,
        pitch: float,
        direction: str = "CW",
        angular_step: float = 0.1,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Generate a helical entry path.

        Produces a helical toolpath from *z_start* to *z_end* at the
        given *center* and *start_radius*.

        :param center: ``(x, y)`` center of the helix.
        :param start_radius: Radius of the helix in mm.
        :param z_start: Starting Z height.
        :param z_end: Ending (target) Z depth.
        :param pitch: Vertical descent per full revolution.
        :param direction: ``"CW"`` or ``"CCW"`` (default ``"CW"``).
        :param angular_step: Angular step in radians (default 0.1).
        :param state: Optional machine state to apply before the path.
        :returns: An :class:`AssemblyResult` with the helical path.
        """
    "#,
    module = "raygeo.ops.assembly.helix"
)]
#[pyfunction(name = "generate_helix")]
#[pyo3(signature = (
    center,
    start_radius,
    z_start,
    z_end,
    pitch,
    direction = "CW",
    angular_step = 0.1,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_helix_py(
    center: (f64, f64),
    start_radius: f64,
    z_start: f64,
    z_end: f64,
    pitch: f64,
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

    let opts = HelixOptions {
        center: Point::new(center.0, center.1),
        start_radius,
        z_start,
        z_end,
        pitch,
        direction: dir,
        angular_step,
    };

    let result = helix::generate_helix(&opts, &cut_state)?;
    Ok(PyAssemblyResult::from_inner(result))
}
