use crate::ops::state::{CoolantMode, State};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

pyo3_stub_gen::module_doc!("raygeo.ops.state", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Machine state tracking for laser cutting and CNC milling.

Tracks the current or intended machine state at any point in a command
sequence, including power level (0.0–1.0), coolant mode, cut speed
and travel speed, active laser source UID, pulse frequency, and pulse
width. State objects are used by Ops to associate machine parameters
with moving commands and to detect rapid (non-power) state changes.
";

/// Register the State and CoolantMode classes with the Python module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<PyState>()?;
    m.add_class::<PyCoolantMode>()?;
    Ok(())
}

/// Coolant mode for CNC milling operations.
///
/// Controls the coolant state: ``Off``, ``Flood``, ``Mist``, or ``Air``.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    hash,
    skip_from_py_object,
    module = "raygeo.ops.state",
    name = "CoolantMode"
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyCoolantMode(pub CoolantMode);

#[gen_stub_pymethods]
#[pymethods]
impl PyCoolantMode {
    #[classattr]
    pub const OFF: PyCoolantMode = PyCoolantMode(CoolantMode::Off);
    #[classattr]
    pub const FLOOD: PyCoolantMode = PyCoolantMode(CoolantMode::Flood);
    #[classattr]
    pub const MIST: PyCoolantMode = PyCoolantMode(CoolantMode::Mist);
    #[classattr]
    pub const AIR: PyCoolantMode = PyCoolantMode(CoolantMode::Air);

    fn __repr__(&self) -> String {
        format!("CoolantMode.{}", self.name())
    }

    #[getter]
    fn value(&self) -> u8 {
        match self.0 {
            CoolantMode::Off => 0,
            CoolantMode::Flood => 1,
            CoolantMode::Mist => 2,
            CoolantMode::Air => 3,
        }
    }

    #[getter]
    fn name(&self) -> String {
        match self.0 {
            CoolantMode::Off => "OFF",
            CoolantMode::Flood => "FLOOD",
            CoolantMode::Mist => "MIST",
            CoolantMode::Air => "AIR",
        }
        .to_string()
    }
}

/// The current state of a laser cutting or CNC milling machine.
///
/// Tracks power level, coolant mode, cut/travel speeds,
/// active laser UID, frequency, pulse width, spindle speed,
/// and coolant mode.
#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.ops.state", name = "State")]
#[derive(Clone)]
pub struct PyState(pub State);

#[gen_stub_pymethods]
#[pymethods]
impl PyState {
    #[allow(clippy::too_many_arguments)]
    #[new]
    #[pyo3(signature = (power=0.0, cut_speed=None, travel_speed=None, active_laser_uid=None, frequency=None, pulse_width=None, dwell_ms=None, spindle_speed=None, coolant=None))]
    fn new(
        power: f64,
        cut_speed: Option<i32>,
        travel_speed: Option<i32>,
        active_laser_uid: Option<String>,
        frequency: Option<i32>,
        pulse_width: Option<f64>,
        dwell_ms: Option<f64>,
        spindle_speed: Option<u32>,
        coolant: Option<Bound<'_, PyCoolantMode>>,
    ) -> Self {
        PyState(State {
            power,
            cut_speed,
            travel_speed,
            active_laser_uid,
            frequency,
            pulse_width,
            dwell_ms,
            spindle_speed,
            coolant: coolant.map(|c| c.borrow().0),
        })
    }

    /// String representation like ``State(power=...)``.
    fn __repr__(&self) -> String {
        format!("State(power={})", self.0.power)
    }

    /// Check whether the machine can transition from the current
    /// state to the *target* state without a ``SetPower`` command.
    ///
    /// :param target: The target state to compare against.
    /// :returns: True if the change is a rapid (non-power) change.
    fn allow_rapid_change(&self, target: &PyState) -> bool {
        self.0.allow_rapid_change(&target.0)
    }

    /// Laser power level (0.0 – 1.0 typically).
    #[getter]
    fn power(&self) -> f64 {
        self.0.power
    }

    #[setter]
    fn set_power(&mut self, value: f64) {
        self.0.power = value;
    }

    /// Cutting speed in mm/s (if set).
    #[getter]
    fn cut_speed(&self) -> Option<i32> {
        self.0.cut_speed
    }

    #[setter]
    fn set_cut_speed(&mut self, value: Option<i32>) {
        self.0.cut_speed = value;
    }

    /// Travel (rapid) speed in mm/s (if set).
    #[getter]
    fn travel_speed(&self) -> Option<i32> {
        self.0.travel_speed
    }

    #[setter]
    fn set_travel_speed(&mut self, value: Option<i32>) {
        self.0.travel_speed = value;
    }

    /// UID of the active laser source (if set).
    #[getter]
    fn active_laser_uid(&self) -> Option<&str> {
        self.0.active_laser_uid.as_deref()
    }

    #[setter]
    fn set_active_laser_uid(&mut self, value: Option<String>) {
        self.0.active_laser_uid = value;
    }

    /// Laser pulse frequency in Hz (if set).
    #[getter]
    fn frequency(&self) -> Option<i32> {
        self.0.frequency
    }

    #[setter]
    fn set_frequency(&mut self, value: Option<i32>) {
        self.0.frequency = value;
    }

    /// Laser pulse width in microseconds (if set).
    #[getter]
    fn pulse_width(&self) -> Option<f64> {
        self.0.pulse_width
    }

    #[setter]
    fn set_pulse_width(&mut self, value: Option<f64>) {
        self.0.pulse_width = value;
    }

    /// Dwell time in milliseconds (if set).
    #[getter]
    fn dwell_ms(&self) -> Option<f64> {
        self.0.dwell_ms
    }

    #[setter]
    fn set_dwell_ms(&mut self, value: Option<f64>) {
        self.0.dwell_ms = value;
    }

    /// Spindle speed in RPM (if set).
    #[getter]
    fn spindle_speed(&self) -> Option<u32> {
        self.0.spindle_speed
    }

    #[setter]
    fn set_spindle_speed(&mut self, value: Option<u32>) {
        self.0.spindle_speed = value;
    }

    /// Coolant mode (if set).
    #[getter]
    fn coolant(&self) -> Option<PyCoolantMode> {
        self.0.coolant.map(PyCoolantMode)
    }

    #[setter]
    fn set_coolant(&mut self, value: Option<Bound<'_, PyCoolantMode>>) {
        self.0.coolant = value.map(|c| c.borrow().0);
    }
}
