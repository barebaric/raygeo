use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::geo::types::Point;
use crate::ops::assembly::thread_mill::{
    self, ThreadHandedness, ThreadMillSpec,
};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "thread_mill")?;
    m.setattr("__doc__", "Thread milling via helical interpolation.")?;
    m.add_function(pyo3::wrap_pyfunction!(
        generate_thread_mill_py,
        m.clone()
    )?)?;
    m.add_class::<PyThreadMillSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.thread_mill", &m)?;

    Ok(())
}

/// Parameters for the ``thread_mill`` assembler: one internal thread.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.thread_mill",
    name = "ThreadMillSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyThreadMillSpec {
    #[pyo3(get)]
    pub center: (f64, f64),
    /// Nominal (major) diameter of the thread (mm).
    #[pyo3(get)]
    pub nominal_diameter: f64,
    /// Thread pitch (lead): axial travel per revolution (mm).
    #[pyo3(get)]
    pub pitch: f64,
    /// Diameter of the thread mill cutter (mm).
    #[pyo3(get)]
    pub tool_diameter: f64,
    /// Z of the top end of the thread (milling runs bottom-up).
    #[pyo3(get)]
    pub z_top: f64,
    /// Z of the bottom end of the thread.
    #[pyo3(get)]
    pub z_bottom: f64,
    /// ``"RH"`` or ``"LH"``.
    #[pyo3(get)]
    pub handedness: String,
    #[pyo3(get)]
    pub safe_z: f64,
}

impl PyThreadMillSpec {
    pub fn into_core(self) -> ThreadMillSpec {
        let handedness = if self.handedness == "LH" {
            ThreadHandedness::Lh
        } else {
            ThreadHandedness::Rh
        };
        ThreadMillSpec {
            center: Point::new(self.center.0, self.center.1),
            nominal_diameter: self.nominal_diameter,
            pitch: self.pitch,
            tool_diameter: self.tool_diameter,
            z_top: self.z_top,
            z_bottom: self.z_bottom,
            handedness,
            safe_z: self.safe_z,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyThreadMillSpec {
    #[new]
    #[pyo3(signature = (
        center,
        nominal_diameter,
        pitch,
        tool_diameter,
        z_top,
        z_bottom,
        handedness = "RH",
        safe_z = 5.0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        center: (f64, f64),
        nominal_diameter: f64,
        pitch: f64,
        tool_diameter: f64,
        z_top: f64,
        z_bottom: f64,
        handedness: &str,
        safe_z: f64,
    ) -> Self {
        PyThreadMillSpec {
            center,
            nominal_diameter,
            pitch,
            tool_diameter,
            z_top,
            z_bottom,
            handedness: handedness.to_string(),
            safe_z,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_thread_mill(
        part: raygeo.ops.part.Part,
        center: tuple[float, float],
        nominal_diameter: float,
        pitch: float,
        tool_diameter: float,
        z_top: float,
        z_bottom: float,
        handedness: str = "RH",
        safe_z: float = 5.0,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate one internal thread-milling cycle.

        The tool plunges at the bore center to ``z_bottom`` (the pilot
        bore is assumed pre-drilled), swings out on a 180° tangential
        lead-in arc to the orbit radius, climbs the thread as a helical
        interpolation (one pitch per revolution), and swings back to
        the center on a lead-out arc. The thread length must be an
        integer multiple of the pitch. A right-hand thread climbs
        counter-clockwise, a left-hand thread clockwise.

        :param part: part owning the face the cycle is assembled on.
        :param center: ``(x, y)`` of the bore.
        :param nominal_diameter: nominal (major) thread diameter (mm).
        :param pitch: thread pitch (lead) (mm).
        :param tool_diameter: thread mill cutter diameter (mm).
        :param z_top: Z of the top end of the thread.
        :param z_bottom: Z of the bottom end of the thread.
        :param handedness: ``"RH"`` (default) or ``"LH"``.
        :param safe_z: safe rapid height (default 5).
        :param state: optional machine state (feeds, spindle, coolant).
        :returns: An :class:`AssemblyResult` with the cycle's moves.
        """
    "#,
    module = "raygeo.ops.assembly.thread_mill"
)]
#[pyfunction(name = "generate_thread_mill")]
#[pyo3(signature = (
    part,
    center,
    nominal_diameter,
    pitch,
    tool_diameter,
    z_top,
    z_bottom,
    handedness = "RH",
    safe_z = 5.0,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_thread_mill_py(
    part: &mut crate::python::ops::part::part::PyPart,
    center: (f64, f64),
    nominal_diameter: f64,
    pitch: f64,
    tool_diameter: f64,
    z_top: f64,
    z_bottom: f64,
    handedness: &str,
    safe_z: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let cut_state = match &state {
        Some(s) => s.borrow().0.clone(),
        None => State::default(),
    };
    let handedness = if handedness == "LH" {
        ThreadHandedness::Lh
    } else {
        ThreadHandedness::Rh
    };
    let opts = ThreadMillSpec {
        center: Point::new(center.0, center.1),
        nominal_diameter,
        pitch,
        tool_diameter,
        z_top,
        z_bottom,
        handedness,
        safe_z,
    };

    let mut trace = Tracelet::new();
    let meta = thread_mill::generate_thread_mill(
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
