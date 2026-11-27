//! Python-visible value types mirroring Rayforge's
//! `machine.driver.driver` module.

// `from_py_object` generates a FromPyObject impl whose .clone() is
// flagged as clone_on_copy for Copy types; the lint cannot be
// targeted at the generated impl, only at this module.
#![allow(clippy::clone_on_copy)]

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods,
};

use crate::driver::core::types as core;

pub(crate) const MODULE_DOC: &str = "\
Shared device value types: DeviceState, DeviceStatus \
and DeviceError. These mirror the identically-named types in \
Rayforge so the Python shell can convert losslessly.
";

pyo3_stub_gen::module_doc!("raygeo.driver.core.types", "{}", MODULE_DOC);

/// Machine state as reported in Grbl status reports.
#[gen_stub_pyclass_enum]
#[pyclass(module = "raygeo.driver.core.types", eq, eq_int, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    #[pyo3(name = "UNKNOWN")]
    Unknown = 0,
    #[pyo3(name = "IDLE")]
    Idle = 1,
    #[pyo3(name = "RUN")]
    Run = 2,
    #[pyo3(name = "HOLD")]
    Hold = 3,
    #[pyo3(name = "JOG")]
    Jog = 4,
    #[pyo3(name = "ALARM")]
    Alarm = 5,
    #[pyo3(name = "DOOR")]
    Door = 6,
    #[pyo3(name = "CHECK")]
    Check = 7,
    #[pyo3(name = "HOME")]
    Home = 8,
    #[pyo3(name = "SLEEP")]
    Sleep = 9,
    #[pyo3(name = "TOOL")]
    Tool = 10,
    #[pyo3(name = "QUEUE")]
    Queue = 11,
    #[pyo3(name = "LOCK")]
    Lock = 12,
    #[pyo3(name = "UNLOCK")]
    Unlock = 13,
    #[pyo3(name = "CYCLE")]
    Cycle = 14,
    #[pyo3(name = "TEST")]
    Test = 15,
}

impl From<core::DeviceStatus> for DeviceStatus {
    fn from(status: core::DeviceStatus) -> Self {
        match status {
            core::DeviceStatus::Unknown => Self::Unknown,
            core::DeviceStatus::Idle => Self::Idle,
            core::DeviceStatus::Run => Self::Run,
            core::DeviceStatus::Hold => Self::Hold,
            core::DeviceStatus::Jog => Self::Jog,
            core::DeviceStatus::Alarm => Self::Alarm,
            core::DeviceStatus::Door => Self::Door,
            core::DeviceStatus::Check => Self::Check,
            core::DeviceStatus::Home => Self::Home,
            core::DeviceStatus::Sleep => Self::Sleep,
            core::DeviceStatus::Tool => Self::Tool,
            core::DeviceStatus::Queue => Self::Queue,
            core::DeviceStatus::Lock => Self::Lock,
            core::DeviceStatus::Unlock => Self::Unlock,
            core::DeviceStatus::Cycle => Self::Cycle,
            core::DeviceStatus::Test => Self::Test,
        }
    }
}

impl From<DeviceStatus> for core::DeviceStatus {
    fn from(status: DeviceStatus) -> Self {
        match status {
            DeviceStatus::Unknown => Self::Unknown,
            DeviceStatus::Idle => Self::Idle,
            DeviceStatus::Run => Self::Run,
            DeviceStatus::Hold => Self::Hold,
            DeviceStatus::Jog => Self::Jog,
            DeviceStatus::Alarm => Self::Alarm,
            DeviceStatus::Door => Self::Door,
            DeviceStatus::Check => Self::Check,
            DeviceStatus::Home => Self::Home,
            DeviceStatus::Sleep => Self::Sleep,
            DeviceStatus::Tool => Self::Tool,
            DeviceStatus::Queue => Self::Queue,
            DeviceStatus::Lock => Self::Lock,
            DeviceStatus::Unlock => Self::Unlock,
            DeviceStatus::Cycle => Self::Cycle,
            DeviceStatus::Test => Self::Test,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl DeviceStatus {
    fn __repr__(&self) -> String {
        format!("DeviceStatus.{}", self.name())
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Idle => "IDLE",
            Self::Run => "RUN",
            Self::Hold => "HOLD",
            Self::Jog => "JOG",
            Self::Alarm => "ALARM",
            Self::Door => "DOOR",
            Self::Check => "CHECK",
            Self::Home => "HOME",
            Self::Sleep => "SLEEP",
            Self::Tool => "TOOL",
            Self::Queue => "QUEUE",
            Self::Lock => "LOCK",
            Self::Unlock => "UNLOCK",
            Self::Cycle => "CYCLE",
            Self::Test => "TEST",
        }
    }
}

/// An error or alarm with code, title and description.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.core.types", eq, from_py_object)]
#[derive(Clone, PartialEq)]
pub struct DeviceError {
    pub(crate) inner: core::DeviceError,
}

impl DeviceError {
    pub(crate) fn new(inner: core::DeviceError) -> Self {
        Self { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl DeviceError {
    #[new]
    fn new_py(code: i32, title: String, description: String) -> Self {
        Self {
            inner: core::DeviceError::new(code, &title, &description),
        }
    }

    #[getter]
    fn code(&self) -> i32 {
        self.inner.code
    }

    #[getter]
    fn title(&self) -> String {
        self.inner.title.clone()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "DeviceError(code={}, title={:?})",
            self.inner.code, self.inner.title
        )
    }
}

/// The complete state of a device at a moment in time.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.core.types", eq, from_py_object)]
#[derive(Clone, PartialEq)]
pub struct DeviceState {
    pub(crate) inner: core::DeviceState,
}

impl DeviceState {
    pub(crate) fn new(inner: core::DeviceState) -> Self {
        Self { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl DeviceState {
    #[new]
    fn new_py() -> Self {
        Self::new(core::DeviceState::default())
    }
    #[getter]
    fn status(&self) -> DeviceStatus {
        self.inner.status.into()
    }

    #[setter]
    fn set_status(&mut self, status: DeviceStatus) {
        self.inner.status = status.into();
    }

    #[getter]
    fn error(&self) -> Option<DeviceError> {
        self.inner.error.clone().map(DeviceError::new)
    }

    #[setter]
    fn set_error(&mut self, error: Option<&DeviceError>) {
        self.inner.error = error.map(|e| e.inner.clone());
    }

    /// Machine position in mm; entries may be `None`.
    #[getter]
    fn machine_pos(&self) -> Vec<Option<f64>> {
        self.inner.machine_pos.clone()
    }

    #[setter]
    fn set_machine_pos(&mut self, pos: Vec<Option<f64>>) {
        self.inner.machine_pos = pos;
    }

    /// Work position in mm; entries may be `None`.
    #[getter]
    fn work_pos(&self) -> Vec<Option<f64>> {
        self.inner.work_pos.clone()
    }

    #[setter]
    fn set_work_pos(&mut self, pos: Vec<Option<f64>>) {
        self.inner.work_pos = pos;
    }

    /// Work coordinate offset in mm; entries may be `None`.
    #[getter]
    fn wco(&self) -> Vec<Option<f64>> {
        self.inner.wco.clone()
    }

    #[setter]
    fn set_wco(&mut self, wco: Vec<Option<f64>>) {
        self.inner.wco = wco;
    }

    #[getter]
    fn feed_rate(&self) -> Option<i64> {
        self.inner.feed_rate
    }

    #[setter]
    fn set_feed_rate(&mut self, feed_rate: Option<i64>) {
        self.inner.feed_rate = feed_rate;
    }

    #[getter]
    fn spindle_speed(&self) -> Option<i64> {
        self.inner.spindle_speed
    }

    #[setter]
    fn set_spindle_speed(&mut self, spindle_speed: Option<i64>) {
        self.inner.spindle_speed = spindle_speed;
    }

    #[getter]
    fn buffer_available(&self) -> Option<i64> {
        self.inner.buffer_available
    }

    #[setter]
    fn set_buffer_available(&mut self, buffer_available: Option<i64>) {
        self.inner.buffer_available = buffer_available;
    }

    #[getter]
    fn buffer_rx_available(&self) -> Option<i64> {
        self.inner.buffer_rx_available
    }

    #[setter]
    fn set_buffer_rx_available(&mut self, buffer_rx_available: Option<i64>) {
        self.inner.buffer_rx_available = buffer_rx_available;
    }

    fn __repr__(&self) -> String {
        format!(
            "DeviceState(status={}, feed_rate={:?})",
            self.inner.status, self.inner.feed_rate
        )
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", MODULE_DOC)?;
    m.add(
        "__all__",
        vec!["DeviceState", "DeviceStatus", "DeviceError"],
    )?;
    m.add_class::<DeviceStatus>()?;
    m.add_class::<DeviceError>()?;
    m.add_class::<DeviceState>()?;
    Ok(())
}
