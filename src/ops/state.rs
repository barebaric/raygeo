use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use raygeo_core::ops::state::{MachineState, State};

/// Register the State and MachineState classes with the Python module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyState>()?;
    m.add_class::<PyMachineState>()?;
    Ok(())
}

/// The current state of a laser cutting job.
///
/// Tracks power level, air assist, cut/travel speeds,
/// active laser UID, frequency, and pulse width.
#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.ops", name = "State")]
#[derive(Clone)]
pub struct PyState(pub State);

#[gen_stub_pymethods]
#[pymethods]
impl PyState {
    #[new]
    #[pyo3(signature = (power=0.0, air_assist=false, cut_speed=None, travel_speed=None, active_laser_uid=None, frequency=None, pulse_width=None))]
    fn new(
        power: f64,
        air_assist: bool,
        cut_speed: Option<i32>,
        travel_speed: Option<i32>,
        active_laser_uid: Option<String>,
        frequency: Option<i32>,
        pulse_width: Option<f64>,
    ) -> Self {
        PyState(State {
            power,
            air_assist,
            cut_speed,
            travel_speed,
            active_laser_uid,
            frequency,
            pulse_width,
        })
    }

    /// String representation like ``State(power=..., air_assist=...)``.
    fn __repr__(&self) -> String {
        format!("State(power={}, air_assist={})", self.0.power, self.0.air_assist)
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

    /// Whether air assist is enabled.
    #[getter]
    fn air_assist(&self) -> bool {
        self.0.air_assist
    }

    #[setter]
    fn set_air_assist(&mut self, value: bool) {
        self.0.air_assist = value;
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
}

/// Immutable snapshot of the machine state at a point in time.
///
/// Unlike :class:`State`, this represents the actual machine state
/// at a specific command index and is typically read-only.
#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.ops", name = "MachineState")]
#[derive(Clone)]
pub struct PyMachineState(pub MachineState);

#[gen_stub_pymethods]
#[pymethods]
impl PyMachineState {
    #[new]
    #[pyo3(signature = (power=0.0, air_assist=false, cut_speed=None, travel_speed=None, active_laser_uid=None, frequency=None, pulse_width=None))]
    fn new(
        power: f64,
        air_assist: bool,
        cut_speed: Option<i32>,
        travel_speed: Option<i32>,
        active_laser_uid: Option<String>,
        frequency: Option<i32>,
        pulse_width: Option<f64>,
    ) -> Self {
        PyMachineState(MachineState {
            power,
            air_assist,
            cut_speed,
            travel_speed,
            active_laser_uid,
            frequency,
            pulse_width,
        })
    }

    /// String representation like ``MachineState(power=..., air_assist=...)``.
    fn __repr__(&self) -> String {
        format!("MachineState(power={}, air_assist={})", self.0.power, self.0.air_assist)
    }

    /// Laser power level at this state.
    #[getter]
    fn power(&self) -> f64 {
        self.0.power
    }

    #[setter]
    fn set_power(&mut self, value: f64) {
        self.0.power = value;
    }

    /// Whether air assist was enabled at this state.
    #[getter]
    fn air_assist(&self) -> bool {
        self.0.air_assist
    }

    #[setter]
    fn set_air_assist(&mut self, value: bool) {
        self.0.air_assist = value;
    }

    /// Cutting speed at this state (mm/s).
    #[getter]
    fn cut_speed(&self) -> Option<i32> {
        self.0.cut_speed
    }

    #[setter]
    fn set_cut_speed(&mut self, value: Option<i32>) {
        self.0.cut_speed = value;
    }

    /// Travel speed at this state (mm/s).
    #[getter]
    fn travel_speed(&self) -> Option<i32> {
        self.0.travel_speed
    }

    #[setter]
    fn set_travel_speed(&mut self, value: Option<i32>) {
        self.0.travel_speed = value;
    }

    /// Active laser UID at this state.
    #[getter]
    fn active_laser_uid(&self) -> Option<&str> {
        self.0.active_laser_uid.as_deref()
    }

    #[setter]
    fn set_active_laser_uid(&mut self, value: Option<String>) {
        self.0.active_laser_uid = value;
    }

    /// Laser pulse frequency at this state (Hz).
    #[getter]
    fn frequency(&self) -> Option<i32> {
        self.0.frequency
    }

    #[setter]
    fn set_frequency(&mut self, value: Option<i32>) {
        self.0.frequency = value;
    }

    /// Laser pulse width at this state (microseconds).
    #[getter]
    fn pulse_width(&self) -> Option<f64> {
        self.0.pulse_width
    }

    #[setter]
    fn set_pulse_width(&mut self, value: Option<f64>) {
        self.0.pulse_width = value;
    }
}
