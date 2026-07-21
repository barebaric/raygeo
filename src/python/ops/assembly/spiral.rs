use crate::geo::algo::helix::HelixDirection;
use crate::ops::assembly::spiral::{self, SpiralSpec};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "spiral")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_spiral_py, m.clone())?)?;
    m.add_class::<PySpiralSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.spiral", &m)?;

    Ok(())
}

/// Parameters for the ``spiral`` assembler.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.spiral",
    name = "SpiralSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PySpiralSpec {
    #[pyo3(get)]
    pub center: (f64, f64),
    #[pyo3(get)]
    pub z: f64,
    #[pyo3(get)]
    pub start_radius: f64,
    #[pyo3(get)]
    pub end_radius: f64,
    #[pyo3(get)]
    pub revolutions: f64,
    /// ``"CW"`` or ``"CCW"``.
    #[pyo3(get)]
    pub direction: String,
    #[pyo3(get)]
    pub angular_step: f64,
    #[pyo3(get)]
    pub start_angle: f64,
}

impl PySpiralSpec {
    pub fn into_core(self) -> SpiralSpec {
        let dir = match self.direction.as_str() {
            "CCW" => HelixDirection::Ccw,
            _ => HelixDirection::Cw,
        };
        SpiralSpec {
            center: Point::new(self.center.0, self.center.1),
            z: self.z,
            start_radius: self.start_radius,
            end_radius: self.end_radius,
            revolutions: self.revolutions,
            direction: dir,
            angular_step: self.angular_step,
            start_angle: self.start_angle,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PySpiralSpec {
    #[new]
    #[pyo3(signature = (
        center,
        z,
        start_radius,
        end_radius,
        revolutions,
        direction = "CW",
        angular_step = 0.1,
        start_angle = 0.0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        center: (f64, f64),
        z: f64,
        start_radius: f64,
        end_radius: f64,
        revolutions: f64,
        direction: &str,
        angular_step: f64,
        start_angle: f64,
    ) -> Self {
        PySpiralSpec {
            center,
            z,
            start_radius,
            end_radius,
            revolutions,
            direction: direction.to_string(),
            angular_step,
            start_angle,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_spiral(
        part: raygeo.ops.part.Part,
        center: tuple[float, float],
        z: float,
        start_radius: float,
        end_radius: float,
        revolutions: float,
        direction: str = "CW",
        angular_step: float = 0.1,
        start_angle: float = 0.0,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a flat spiral entry path.

        Produces an Archimedean spiral from *start_radius* to *end_radius*
        at constant Z, followed by a smoothing full-circle pass at
        *end_radius*.

        :param center: ``(x, y)`` center of the spiral.
        :param z: Cutting Z height.
        :param start_radius: Starting radius in mm.
        :param end_radius: Ending radius in mm.
        :param revolutions: Number of full turns (may be fractional).
        :param direction: ``"CW"`` or ``"CCW"`` (default ``"CW"``).
        :param angular_step: Angular step in radians (default 0.1).
        :param start_angle: Starting angle in radians (default 0.0).
        :param state: Optional machine state to apply before the path.
        :returns: An :class:`AssemblyResult` with the spiral path.
        """
    "#,
    module = "raygeo.ops.assembly.spiral"
)]
#[pyfunction(name = "generate_spiral")]
#[pyo3(signature = (
    part,
    center,
    z,
    start_radius,
    end_radius,
    revolutions,
    direction = "CW",
    angular_step = 0.1,
    start_angle = 0.0,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_spiral_py(
    part: &mut crate::python::ops::part::part::PyPart,
    center: (f64, f64),
    z: f64,
    start_radius: f64,
    end_radius: f64,
    revolutions: f64,
    direction: &str,
    angular_step: f64,
    start_angle: f64,
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

    let opts = SpiralSpec {
        center: Point::new(center.0, center.1),
        z,
        start_radius,
        end_radius,
        revolutions,
        direction: dir,
        angular_step,
        start_angle,
    };

    let mut trace = Tracelet::new();
    let face = part.inner.face_mut("");
    let meta = spiral::generate_spiral(face, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}
