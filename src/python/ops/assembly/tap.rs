use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::geo::types::Point;
use crate::ops::assembly::tap::{self, TapSpec};
use crate::ops::assembly::thread_mill::ThreadHandedness;
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "tap")?;
    m.setattr(
        "__doc__",
        "Rigid tapping via spindle-synchronized feed moves.",
    )?;
    m.add_function(pyo3::wrap_pyfunction!(generate_tap_py, m.clone())?)?;
    m.add_class::<PyTapSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.tap", &m)?;

    Ok(())
}

/// Parameters for the ``tap`` assembler: one rigid-tapping cycle.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.tap",
    name = "TapSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyTapSpec {
    #[pyo3(get)]
    pub center: (f64, f64),
    /// Nominal (major) diameter of the thread (mm).
    #[pyo3(get)]
    pub nominal_diameter: f64,
    /// Thread pitch (lead): axial travel per spindle revolution (mm).
    #[pyo3(get)]
    pub pitch: f64,
    /// ``"RH"`` or ``"LH"``.
    #[pyo3(get)]
    pub handedness: String,
    /// Surface Z the hole starts at.
    #[pyo3(get)]
    pub z_surface: f64,
    /// Depth of the thread.
    #[pyo3(get)]
    pub target_z: f64,
    /// Rapid plane just above the surface.
    #[pyo3(get)]
    pub rapid_z: f64,
    #[pyo3(get)]
    pub safe_z: f64,
    /// Peck increment (mm); ``None`` taps in a single pass.
    #[pyo3(get)]
    pub peck: Option<f64>,
    /// Dwell at the bottom before the spindle reversal (ms).
    #[pyo3(get)]
    pub dwell_ms: f64,
}

impl PyTapSpec {
    pub fn into_core(self) -> TapSpec {
        let handedness = if self.handedness == "LH" {
            ThreadHandedness::Lh
        } else {
            ThreadHandedness::Rh
        };
        TapSpec {
            center: Point::new(self.center.0, self.center.1),
            nominal_diameter: self.nominal_diameter,
            pitch: self.pitch,
            handedness,
            z_surface: self.z_surface,
            target_z: self.target_z,
            rapid_z: self.rapid_z,
            safe_z: self.safe_z,
            peck: self.peck,
            dwell_ms: self.dwell_ms,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyTapSpec {
    #[new]
    #[pyo3(signature = (
        center,
        nominal_diameter,
        pitch,
        target_z,
        z_surface = 0.0,
        rapid_z = 1.0,
        safe_z = 5.0,
        handedness = "RH",
        peck = None,
        dwell_ms = 0.0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        center: (f64, f64),
        nominal_diameter: f64,
        pitch: f64,
        target_z: f64,
        z_surface: f64,
        rapid_z: f64,
        safe_z: f64,
        handedness: &str,
        peck: Option<f64>,
        dwell_ms: f64,
    ) -> Self {
        PyTapSpec {
            center,
            nominal_diameter,
            pitch,
            handedness: handedness.to_string(),
            z_surface,
            target_z,
            rapid_z,
            safe_z,
            peck,
            dwell_ms,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_tap(
        part: raygeo.ops.part.Part,
        center: tuple[float, float],
        nominal_diameter: float,
        pitch: float,
        target_z: float,
        z_surface: float = 0.0,
        rapid_z: float = 1.0,
        safe_z: float = 5.0,
        handedness: str = "RH",
        peck: float | None = None,
        dwell_ms: float = 0.0,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate one rigid-tapping cycle.

        The tap rapids to the pilot bore (assumed pre-drilled to the
        tap drill diameter) and descends with the spindle synchronized
        to the Z feed — the moves travel ``pitch`` mm per spindle
        revolution, i.e. at ``pitch * rpm`` mm/min, so ``state`` must
        set ``spindle_rpm``. At the bottom the spindle reverses and the
        synchronized retract unscrews the tap. A right-hand thread taps
        clockwise (G84), a left-hand thread counter-clockwise (G74).

        :param part: part owning the face the cycle is assembled on.
        :param center: ``(x, y)`` of the pilot bore.
        :param nominal_diameter: nominal (major) thread diameter (mm).
        :param pitch: thread pitch (lead) (mm).
        :param target_z: depth of the thread.
        :param z_surface: surface Z the hole starts at (default 0).
        :param rapid_z: rapid plane above the surface (default 1).
        :param safe_z: safe rapid height (default 5).
        :param handedness: ``"RH"`` (default) or ``"LH"``.
        :param peck: optional peck increment; full synchronized retract
            to ``rapid_z`` between pecks (default None).
        :param dwell_ms: dwell at the bottom before the reversal (default 0).
        :param state: optional machine state; ``spindle_rpm`` is
            required for the synchronization.
        :returns: An :class:`AssemblyResult` with the cycle's moves.
        """
    "#,
    module = "raygeo.ops.assembly.tap"
)]
#[pyfunction(name = "generate_tap")]
#[pyo3(signature = (
    part,
    center,
    nominal_diameter,
    pitch,
    target_z,
    z_surface = 0.0,
    rapid_z = 1.0,
    safe_z = 5.0,
    handedness = "RH",
    peck = None,
    dwell_ms = 0.0,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_tap_py(
    part: &mut crate::python::ops::part::part::PyPart,
    center: (f64, f64),
    nominal_diameter: f64,
    pitch: f64,
    target_z: f64,
    z_surface: f64,
    rapid_z: f64,
    safe_z: f64,
    handedness: &str,
    peck: Option<f64>,
    dwell_ms: f64,
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
    let opts = TapSpec {
        center: Point::new(center.0, center.1),
        nominal_diameter,
        pitch,
        handedness,
        z_surface,
        target_z,
        rapid_z,
        safe_z,
        peck,
        dwell_ms,
    };

    let mut trace = Tracelet::new();
    let meta = tap::generate_tap(
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
