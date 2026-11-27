//! The mock transport exposed to Python for tests.

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::transport::mock::MockInner;
use crate::driver::core::transport::Transport;

/// An in-memory byte transport for tests.
///
/// Feed bytes to the session with `push()` (as if the device had
/// sent them), inspect everything the session sent with `sent()`,
/// and simulate link state with `set_connected()` /
/// `set_write_error()`.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.core.transport")]
pub struct MockTransport {
    pub(crate) inner: Arc<MockInner>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MockTransport {
    #[new]
    fn new() -> Self {
        Self {
            inner: MockInner::new(),
        }
    }

    /// Deliver bytes to the session as if the device sent them.
    fn push(&self, data: Vec<u8>) {
        self.inner.push(data);
    }

    /// All bytes written to the device so far.
    fn sent<'py>(&self, py: Python<'py>) -> Vec<Bound<'py, PyBytes>> {
        self.inner
            .sent()
            .into_iter()
            .map(|data| PyBytes::new(py, &data))
            .collect()
    }

    /// Writes recorded since the previous call (or the last
    /// `clear_sent`): a monotonic cursor, so clearing the log never
    /// skips or replays chunks.  This is what device emulators
    /// should consume.
    fn take_new_sent<'py>(&self, py: Python<'py>) -> Vec<Bound<'py, PyBytes>> {
        self.inner
            .take_new_sent()
            .into_iter()
            .map(|data| PyBytes::new(py, &data))
            .collect()
    }

    /// Clear the recorded sent bytes.
    fn clear_sent(&self) {
        self.inner.clear_sent();
    }

    /// Simulate connect/disconnect of the physical link.
    fn set_connected(&self, connected: bool) {
        self.inner.set_open(connected);
    }

    /// True while the transport link is open (i.e. a session
    /// connected through this mock).
    #[getter]
    fn is_connected(&self) -> bool {
        self.inner.is_open()
    }

    /// Fail subsequent writes with this message (and mark the link
    /// down), simulating an unplugged cable.
    #[pyo3(signature = (message=None))]
    fn set_write_error(&self, message: Option<String>) {
        self.inner.set_write_error(message);
    }

    fn __repr__(&self) -> &'static str {
        "MockTransport()"
    }
}
