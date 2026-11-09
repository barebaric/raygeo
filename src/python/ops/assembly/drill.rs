use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::geo::types::Point;
use crate::ops::assembly::drill::{
    self, CountersinkSpec, DrillSpec, PeckMode, PeckParams,
};
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "drill")?;
    m.setattr(
        "__doc__",
        "Hole-making cycles: drilling, pecking and countersinking.",
    )?;
    m.add_function(pyo3::wrap_pyfunction!(generate_drill_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        generate_countersink_py,
        m.clone()
    )?)?;
    m.add_class::<PyDrillSpec>()?;
    m.add_class::<PyPeckParams>()?;
    m.add_class::<PyCountersinkSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.drill", &m)?;

    Ok(())
}

/// Peck cycle parameters.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.drill",
    name = "PeckParams",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyPeckParams {
    /// ``"FULL_RETRACT"`` (clear the flutes from the hole, G83) or
    /// ``"CHIP_BREAK"`` (small retract, G73).
    #[pyo3(get)]
    pub mode: String,
    /// Axial depth per peck, measured from the surface (mm).
    #[pyo3(get)]
    pub increment: f64,
    /// Chip-break retract distance above the current depth (mm);
    /// only used with ``"CHIP_BREAK"``.
    #[pyo3(get)]
    pub retract_amount: f64,
}

impl PyPeckParams {
    pub fn into_core(self) -> PeckParams {
        let mode = if self.mode == "CHIP_BREAK" {
            PeckMode::ChipBreak
        } else {
            PeckMode::FullRetract
        };
        PeckParams {
            mode,
            increment: self.increment,
            retract_amount: self.retract_amount,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyPeckParams {
    #[new]
    #[pyo3(signature = (increment, mode = "FULL_RETRACT", retract_amount = 0.5))]
    fn new(increment: f64, mode: &str, retract_amount: f64) -> Self {
        PyPeckParams {
            mode: mode.to_string(),
            increment,
            retract_amount,
        }
    }
}

/// Parameters for the ``drill`` assembler: one cycle at one hole.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.drill",
    name = "DrillSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyDrillSpec {
    #[pyo3(get)]
    pub center: (f64, f64),
    /// Drill (and hole) diameter (mm).
    #[pyo3(get)]
    pub diameter: f64,
    /// Full-diameter depth of the hole (mm).
    #[pyo3(get)]
    pub target_z: f64,
    /// Drill point angle in degrees (tip compensation).
    #[pyo3(get)]
    pub tip_angle: f64,
    /// Surface Z the hole starts at.
    #[pyo3(get)]
    pub z_surface: f64,
    /// Rapid plane just above the surface.
    #[pyo3(get)]
    pub rapid_z: f64,
    /// Safe rapid height.
    #[pyo3(get)]
    pub safe_z: f64,
    #[pyo3(get)]
    pub peck: Option<PyPeckParams>,
    /// Dwell at the bottom of every plunge (ms).
    #[pyo3(get)]
    pub dwell_ms: f64,
    /// Marks the cycle as a spot drill.
    #[pyo3(get)]
    pub spot: bool,
    /// When true (default), ``target_z`` is the full-diameter depth and
    /// the tip descends deeper by the tip compensation. When false,
    /// ``target_z`` is the depth the tip reaches.
    #[pyo3(get)]
    pub tip_compensate: bool,
}

impl PyDrillSpec {
    pub fn into_core(self) -> DrillSpec {
        DrillSpec {
            center: Point::new(self.center.0, self.center.1),
            diameter: self.diameter,
            tip_angle: self.tip_angle,
            z_surface: self.z_surface,
            target_z: self.target_z,
            rapid_z: self.rapid_z,
            safe_z: self.safe_z,
            peck: self.peck.map(PyPeckParams::into_core),
            dwell_ms: self.dwell_ms,
            spot: self.spot,
            tip_compensate: self.tip_compensate,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyDrillSpec {
    #[new]
    #[pyo3(signature = (
        center,
        diameter,
        target_z,
        tip_angle = 118.0,
        z_surface = 0.0,
        rapid_z = 1.0,
        safe_z = 5.0,
        peck = None,
        dwell_ms = 0.0,
        spot = false,
        tip_compensate = true,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        center: (f64, f64),
        diameter: f64,
        target_z: f64,
        tip_angle: f64,
        z_surface: f64,
        rapid_z: f64,
        safe_z: f64,
        peck: Option<PyPeckParams>,
        dwell_ms: f64,
        spot: bool,
        tip_compensate: bool,
    ) -> Self {
        PyDrillSpec {
            center,
            diameter,
            target_z,
            tip_angle,
            z_surface,
            rapid_z,
            safe_z,
            peck,
            dwell_ms,
            spot,
            tip_compensate,
        }
    }
}

/// Parameters for the ``countersink`` assembler: one conical plunge
/// at one hole.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.drill",
    name = "CountersinkSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyCountersinkSpec {
    #[pyo3(get)]
    pub center: (f64, f64),
    /// Included angle of the countersink cone in degrees.
    #[pyo3(get)]
    pub tip_angle: f64,
    /// Chamfer diameter produced at the surface (mm).
    #[pyo3(get)]
    pub target_diameter: f64,
    #[pyo3(get)]
    pub z_surface: f64,
    #[pyo3(get)]
    pub rapid_z: f64,
    #[pyo3(get)]
    pub safe_z: f64,
    #[pyo3(get)]
    pub dwell_ms: f64,
}

impl PyCountersinkSpec {
    pub fn into_core(self) -> CountersinkSpec {
        CountersinkSpec {
            center: Point::new(self.center.0, self.center.1),
            tip_angle: self.tip_angle,
            target_diameter: self.target_diameter,
            z_surface: self.z_surface,
            rapid_z: self.rapid_z,
            safe_z: self.safe_z,
            dwell_ms: self.dwell_ms,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyCountersinkSpec {
    #[new]
    #[pyo3(signature = (
        center,
        tip_angle,
        target_diameter,
        z_surface = 0.0,
        rapid_z = 1.0,
        safe_z = 5.0,
        dwell_ms = 0.0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        center: (f64, f64),
        tip_angle: f64,
        target_diameter: f64,
        z_surface: f64,
        rapid_z: f64,
        safe_z: f64,
        dwell_ms: f64,
    ) -> Self {
        PyCountersinkSpec {
            center,
            tip_angle,
            target_diameter,
            z_surface,
            rapid_z,
            safe_z,
            dwell_ms,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    import raygeo

    def generate_drill(
        part: raygeo.ops.part.Part,
        center: tuple[float, float],
        diameter: float,
        target_z: float,
        tip_angle: float = 118.0,
        z_surface: float = 0.0,
        rapid_z: float = 1.0,
        safe_z: float = 5.0,
        peck: raygeo.ops.assembly.drill.PeckParams | None = None,
        dwell_ms: float = 0.0,
        spot: bool = False,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate one drilling cycle as explicit moves.

        Rapids to the safe plane and the rapid plane, plunges at the
        plunge feed (with dwell), retracts per the peck mode, and
        retracts to the safe plane. ``target_z`` is the full-diameter
        depth; the tip descends deeper by the tip compensation
        ``(diameter / 2) / tan(tip_angle / 2)``. The whole cycle is
        wrapped in a ``DRILL_CYCLE`` section so encoders can substitute
        a canned cycle.

        :param part: part owning the face the cycle is assembled on.
        :param center: ``(x, y)`` of the hole.
        :param diameter: drill (and hole) diameter (mm).
        :param target_z: full-diameter depth (mm).
        :param tip_angle: drill point angle in degrees (default 118).
        :param z_surface: surface Z the hole starts at (default 0).
        :param rapid_z: rapid plane above the surface (default 1).
        :param safe_z: safe rapid height (default 5).
        :param peck: peck parameters; None drills in a single plunge.
        :param dwell_ms: dwell at the bottom of every plunge (ms).
        :param spot: marks the cycle as a spot drill.
        :param state: optional machine state (feeds, spindle, coolant).
        :returns: An :class:`AssemblyResult` with the cycle's moves.
        """
    "#,
    module = "raygeo.ops.assembly.drill"
)]
#[pyfunction(name = "generate_drill")]
#[pyo3(signature = (
    part,
    center,
    diameter,
    target_z,
    tip_angle = 118.0,
    z_surface = 0.0,
    rapid_z = 1.0,
    safe_z = 5.0,
    peck = None,
    dwell_ms = 0.0,
    spot = false,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_drill_py(
    part: &mut crate::python::ops::part::part::PyPart,
    center: (f64, f64),
    diameter: f64,
    target_z: f64,
    tip_angle: f64,
    z_surface: f64,
    rapid_z: f64,
    safe_z: f64,
    peck: Option<Bound<'_, PyPeckParams>>,
    dwell_ms: f64,
    spot: bool,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let cut_state = match &state {
        Some(s) => s.borrow().0.clone(),
        None => State::default(),
    };
    let opts = DrillSpec {
        center: Point::new(center.0, center.1),
        diameter,
        tip_angle,
        z_surface,
        target_z,
        rapid_z,
        safe_z,
        peck: peck.map(|p| p.borrow().clone().into_core()),
        dwell_ms,
        spot,
        tip_compensate: true,
    };

    let mut trace = Tracelet::new();
    let meta = drill::generate_drill(
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

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_countersink(
        part: raygeo.ops.part.Part,
        center: tuple[float, float],
        tip_angle: float,
        target_diameter: float,
        z_surface: float = 0.0,
        rapid_z: float = 1.0,
        safe_z: float = 5.0,
        dwell_ms: float = 0.0,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Generate one countersink cycle as explicit moves.

        Plunges a conical countersink tool to the depth that produces
        ``target_diameter`` at the surface with the given cone angle.
        The cycle is wrapped in a ``DRILL_CYCLE`` section.

        :param part: part owning the face the cycle is assembled on.
        :param center: ``(x, y)`` of the hole.
        :param tip_angle: included angle of the countersink cone in
                          degrees (e.g. 90).
        :param target_diameter: chamfer diameter at the surface (mm).
        :param z_surface: surface Z (default 0).
        :param rapid_z: rapid plane above the surface (default 1).
        :param safe_z: safe rapid height (default 5).
        :param dwell_ms: dwell at the bottom (ms).
        :param state: optional machine state (feeds, spindle, coolant).
        :returns: An :class:`AssemblyResult` with the cycle's moves.
        """
    "#,
    module = "raygeo.ops.assembly.drill"
)]
#[pyfunction(name = "generate_countersink")]
#[pyo3(signature = (
    part,
    center,
    tip_angle,
    target_diameter,
    z_surface = 0.0,
    rapid_z = 1.0,
    safe_z = 5.0,
    dwell_ms = 0.0,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_countersink_py(
    part: &mut crate::python::ops::part::part::PyPart,
    center: (f64, f64),
    tip_angle: f64,
    target_diameter: f64,
    z_surface: f64,
    rapid_z: f64,
    safe_z: f64,
    dwell_ms: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let cut_state = match &state {
        Some(s) => s.borrow().0.clone(),
        None => State::default(),
    };
    let opts = CountersinkSpec {
        center: Point::new(center.0, center.1),
        tip_angle,
        target_diameter,
        z_surface,
        rapid_z,
        safe_z,
        dwell_ms,
    };

    let mut trace = Tracelet::new();
    let meta = drill::generate_countersink(
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
