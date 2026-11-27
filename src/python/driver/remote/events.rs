//! Conversion between Python session events and wire events.

use pyo3::prelude::*;

use crate::driver::core::remote::protocol::WireEvent;
use crate::python::driver::core::types::DeviceState as PyDeviceState;
use crate::python::driver::events::DriverEvent;

pub(crate) fn event_to_wire(event: &DriverEvent) -> WireEvent {
    match event {
        DriverEvent::StateChanged { state } => {
            WireEvent::State(state.inner.clone())
        }
        DriverEvent::ConnectionStatusChanged { status, message } => {
            WireEvent::ConnectionStatus {
                status: status.clone(),
                message: message.clone(),
            }
        }
        DriverEvent::CommandStatusChanged { status, message } => {
            WireEvent::CommandStatus {
                status: status.clone(),
                message: message.clone(),
            }
        }
        DriverEvent::JobFinished {} => WireEvent::JobFinished,
        DriverEvent::JobError { message } => WireEvent::JobError {
            message: message.clone(),
        },
        DriverEvent::ProbeStatusChanged { message } => WireEvent::ProbeStatus {
            message: message.clone(),
        },
        DriverEvent::WcsUpdated { offsets } => WireEvent::WcsUpdated {
            offsets: offsets.clone(),
        },
        DriverEvent::ConfigChanged { key, value } => WireEvent::ConfigChanged {
            key: key.clone(),
            value: *value,
        },
        DriverEvent::RawRx { line } => WireEvent::RawRx { line: line.clone() },
        DriverEvent::RawTx { line } => WireEvent::RawTx { line: line.clone() },
        DriverEvent::RawRxBytes { data } => {
            WireEvent::RawRxBytes { data: data.clone() }
        }
        DriverEvent::RawTxBytes { data } => {
            WireEvent::RawTxBytes { data: data.clone() }
        }
    }
}

/// Extract a Python `DriverEvent` from a callback argument.
pub(crate) fn extract_event(
    py: Python<'_>,
    value: &Py<PyAny>,
) -> Option<DriverEvent> {
    value
        .bind(py)
        .extract::<PyRef<DriverEvent>>()
        .ok()
        .map(|event| match &*event {
            DriverEvent::StateChanged { state } => DriverEvent::StateChanged {
                state: state.clone(),
            },
            DriverEvent::ConnectionStatusChanged { status, message } => {
                DriverEvent::ConnectionStatusChanged {
                    status: status.clone(),
                    message: message.clone(),
                }
            }
            DriverEvent::CommandStatusChanged { status, message } => {
                DriverEvent::CommandStatusChanged {
                    status: status.clone(),
                    message: message.clone(),
                }
            }
            DriverEvent::JobFinished {} => DriverEvent::JobFinished {},
            DriverEvent::JobError { message } => DriverEvent::JobError {
                message: message.clone(),
            },
            DriverEvent::ProbeStatusChanged { message } => {
                DriverEvent::ProbeStatusChanged {
                    message: message.clone(),
                }
            }
            DriverEvent::WcsUpdated { offsets } => DriverEvent::WcsUpdated {
                offsets: offsets.clone(),
            },
            DriverEvent::ConfigChanged { key, value } => {
                DriverEvent::ConfigChanged {
                    key: key.clone(),
                    value: *value,
                }
            }
            DriverEvent::RawRx { line } => {
                DriverEvent::RawRx { line: line.clone() }
            }
            DriverEvent::RawTx { line } => {
                DriverEvent::RawTx { line: line.clone() }
            }
            DriverEvent::RawRxBytes { data } => {
                DriverEvent::RawRxBytes { data: data.clone() }
            }
            DriverEvent::RawTxBytes { data } => {
                DriverEvent::RawTxBytes { data: data.clone() }
            }
        })
}

/// Builds the Python `DriverEvent` variant object for *event*.
pub(crate) fn wire_to_py_event(
    py: Python<'_>,
    event: &WireEvent,
) -> PyResult<Option<Py<PyAny>>> {
    let base = py.import("raygeo.driver.core")?.getattr("DriverEvent")?;
    match event {
        WireEvent::State(state) => {
            let cls = base.getattr("StateChanged")?;
            let state = Py::new(py, PyDeviceState::new(state.clone()))?;
            Ok(Some(cls.call1((state,))?.unbind()))
        }
        WireEvent::ConnectionStatus { status, message } => {
            let cls = base.getattr("ConnectionStatusChanged")?;
            Ok(Some(cls.call1((status.clone(), message.clone()))?.unbind()))
        }
        WireEvent::CommandStatus { status, message } => {
            let cls = base.getattr("CommandStatusChanged")?;
            Ok(Some(cls.call1((status.clone(), message.clone()))?.unbind()))
        }
        WireEvent::JobFinished => {
            let cls = base.getattr("JobFinished")?;
            Ok(Some(cls.call0()?.unbind()))
        }
        WireEvent::JobError { message } => {
            let cls = base.getattr("JobError")?;
            Ok(Some(cls.call1((message.clone(),))?.unbind()))
        }
        WireEvent::ProbeStatus { message } => {
            let cls = base.getattr("ProbeStatusChanged")?;
            Ok(Some(cls.call1((message.clone(),))?.unbind()))
        }
        WireEvent::WcsUpdated { offsets } => {
            let cls = base.getattr("WcsUpdated")?;
            let dict = pyo3::types::PyDict::new(py);
            for (key, (x, y, z)) in offsets {
                dict.set_item(key, (x, y, z))?;
            }
            Ok(Some(cls.call1((dict,))?.unbind()))
        }
        WireEvent::ConfigChanged { key, value } => {
            let cls = base.getattr("ConfigChanged")?;
            Ok(Some(cls.call1((key.clone(), *value))?.unbind()))
        }
        WireEvent::RawRx { line } => {
            let cls = base.getattr("RawRx")?;
            Ok(Some(cls.call1((line.clone(),))?.unbind()))
        }
        WireEvent::RawTx { line } => {
            let cls = base.getattr("RawTx")?;
            Ok(Some(cls.call1((line.clone(),))?.unbind()))
        }
        WireEvent::RawRxBytes { data } => {
            let cls = base.getattr("RawRxBytes")?;
            let data = pyo3::types::PyBytes::new(py, data);
            Ok(Some(cls.call1((data,))?.unbind()))
        }
        WireEvent::RawTxBytes { data } => {
            let cls = base.getattr("RawTxBytes")?;
            let data = pyo3::types::PyBytes::new(py, data);
            Ok(Some(cls.call1((data,))?.unbind()))
        }
        _ => Ok(None),
    }
}
