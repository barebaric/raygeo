use crate::geo::algo::ramp::RampStyle;
use crate::ops::assembly::ramp::{self, RampSpec};
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
    let m = PyModule::new(py, "ramp")?;
    m.add_function(pyo3::wrap_pyfunction!(generate_ramp_py, m.clone())?)?;
    m.add_class::<PyRampSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.ramp", &m)?;

    Ok(())
}

/// Parameters for the ``ramp`` assembler.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.ramp",
    name = "RampSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyRampSpec {
    #[pyo3(get)]
    pub start: (f64, f64),
    #[pyo3(get)]
    pub end: (f64, f64),
    #[pyo3(get)]
    pub z_start: f64,
    #[pyo3(get)]
    pub z_end: f64,
    #[pyo3(get)]
    pub max_ramp_angle_deg: f64,
    /// ``"linear"`` or ``"zigzag"``.
    #[pyo3(get)]
    pub style: String,
    #[pyo3(get)]
    pub lateral_amplitude: f64,
}

impl PyRampSpec {
    pub fn into_core(self) -> RampSpec {
        let style = match self.style.as_str() {
            "linear" => RampStyle::Linear,
            _ => RampStyle::ZigZag,
        };
        RampSpec {
            start: Point::new(self.start.0, self.start.1),
            end: Point::new(self.end.0, self.end.1),
            z_start: self.z_start,
            z_end: self.z_end,
            max_ramp_angle_deg: self.max_ramp_angle_deg,
            style,
            lateral_amplitude: self.lateral_amplitude,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyRampSpec {
    #[new]
    #[pyo3(signature = (
        start,
        end,
        z_start,
        z_end,
        max_ramp_angle_deg = 45.0,
        style = "zigzag",
        lateral_amplitude = 2.0,
    ))]
    fn new(
        start: (f64, f64),
        end: (f64, f64),
        z_start: f64,
        z_end: f64,
        max_ramp_angle_deg: f64,
        style: &str,
        lateral_amplitude: f64,
    ) -> Self {
        PyRampSpec {
            start,
            end,
            z_start,
            z_end,
            max_ramp_angle_deg,
            style: style.to_string(),
            lateral_amplitude,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_ramp(
        part: raygeo.ops.part.Part,
        start: tuple[float, float],
        end: tuple[float, float],
        z_start: float,
        z_end: float,
        max_ramp_angle_deg: float = 45.0,
        style: str = "zigzag",
        lateral_amplitude: float = 2.0,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate a ramp entry path.

        Produces a ramp (linear or zigzag) from *start* to *end* while
        descending from *z_start* to *z_end*.

        :param start: ``(x, y)`` start point.
        :param end: ``(x, y)`` end point.
        :param z_start: Starting Z height.
        :param z_end: Ending (target) Z depth.
        :param max_ramp_angle_deg: Maximum ramp angle in degrees (default 45).
        :param style: ``"linear"`` or ``"zigzag"`` (default ``"zigzag"``).
        :param lateral_amplitude: Lateral oscillation amplitude for zigzag (default 2.0).
        :param state: Optional machine state to apply before the path.
        :returns: An :class:`AssemblyResult` with the ramp path.
        """
    "#,
    module = "raygeo.ops.assembly.ramp"
)]
#[pyfunction(name = "generate_ramp")]
#[pyo3(signature = (
    part,
    start,
    end,
    z_start,
    z_end,
    max_ramp_angle_deg = 45.0,
    style = "zigzag",
    lateral_amplitude = 2.0,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_ramp_py(
    part: &mut crate::python::ops::part::part::PyPart,
    start: (f64, f64),
    end: (f64, f64),
    z_start: f64,
    z_end: f64,
    max_ramp_angle_deg: f64,
    style: &str,
    lateral_amplitude: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let ramp_style = match style.to_lowercase().as_str() {
        "linear" => RampStyle::Linear,
        _ => RampStyle::ZigZag,
    };

    let cut_state = match state {
        Some(ref s) => s.borrow().0.clone(),
        None => State::default(),
    };

    let opts = RampSpec {
        start: Point::new(start.0, start.1),
        end: Point::new(end.0, end.1),
        z_start,
        z_end,
        max_ramp_angle_deg,
        style: ramp_style,
        lateral_amplitude,
    };

    let mut trace = Tracelet::new();
    let face = part.inner.face_mut("");
    let meta = ramp::generate_ramp(face, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}
