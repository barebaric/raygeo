use crate::geo::algo::helix::HelixDirection;
use crate::geo::types::Point;
use crate::ops::assembly::helix::{self, HelixSpec};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "helix")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_helix_py, m.clone())?)?;
    m.add_class::<PyHelixSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.helix", &m)?;

    Ok(())
}

/// Parameters for the ``helix`` assembler.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.helix",
    name = "HelixSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyHelixSpec {
    #[pyo3(get)]
    pub center: (f64, f64),
    #[pyo3(get)]
    pub start_radius: f64,
    #[pyo3(get)]
    pub z_start: f64,
    #[pyo3(get)]
    pub z_end: f64,
    #[pyo3(get)]
    pub pitch: f64,
    /// ``"CW"`` or ``"CCW"``.
    #[pyo3(get)]
    pub direction: String,
    #[pyo3(get)]
    pub angular_step: f64,
}

impl PyHelixSpec {
    pub fn into_core(self) -> HelixSpec {
        let dir = match self.direction.as_str() {
            "CCW" => HelixDirection::Ccw,
            _ => HelixDirection::Cw,
        };
        HelixSpec {
            center: Point::new(self.center.0, self.center.1),
            start_radius: self.start_radius,
            z_start: self.z_start,
            z_end: self.z_end,
            pitch: self.pitch,
            direction: dir,
            angular_step: self.angular_step,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyHelixSpec {
    #[new]
    #[pyo3(signature = (
        center,
        start_radius,
        z_start,
        z_end,
        pitch,
        direction = "CW",
        angular_step = 0.1,
    ))]
    fn new(
        center: (f64, f64),
        start_radius: f64,
        z_start: f64,
        z_end: f64,
        pitch: f64,
        direction: &str,
        angular_step: f64,
    ) -> Self {
        PyHelixSpec {
            center,
            start_radius,
            z_start,
            z_end,
            pitch,
            direction: direction.to_string(),
            angular_step,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def generate_helix(
        part: raygeo.ops.part.Part,
        center: tuple[float, float],
        start_radius: float,
        z_start: float,
        z_end: float,
        pitch: float,
        direction: str = "CW",
        angular_step: float = 0.1,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
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
    part,
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
    part: &mut crate::python::ops::part::part::PyPart,
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

    let opts = HelixSpec {
        center: Point::new(center.0, center.1),
        start_radius,
        z_start,
        z_end,
        pitch,
        direction: dir,
        angular_step,
    };

    let mut trace = Tracelet::new();
    let meta = helix::generate_helix(
        part.inner.face_mut(""),
        &mut trace,
        &opts,
        &cut_state,
    )?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}
