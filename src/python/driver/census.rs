//! The protocol-generic device census surface.
//!
//! A session's ``probe_device`` resolves to the census pyclass of
//! its own protocol; the remote proxy resolves to whichever one the
//! hosted protocol produces, typed as their union.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::{ImportRef, PyStubType, TypeInfo};
use serde_json::Value;

use crate::python::driver::grbl::probe::GrblDeviceInfo;
use crate::python::driver::marlin::probe::MarlinDeviceInfo;
use crate::python::driver::octoprint::probe::OctoPrintDeviceInfo;
use crate::python::driver::smoothie::probe::SmoothieDeviceInfo;

/// Runs the native `probe_device()` method on *session* and returns
/// its awaitable. Rayforge imports module-level `probe_device()`
/// helpers; keeping the call in Rust avoids a Python shim module that
/// would shadow the native `probe` module.
pub(crate) fn call_session_probe(
    py: Python<'_>,
    session: &Bound<'_, PyAny>,
    boot_lines: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    let has_boot_lines = boot_lines.is_some();
    let kwargs = PyDict::new(py);
    if let Some(lines) = boot_lines {
        kwargs.set_item("boot_lines", lines)?;
    }
    let awaitable = if has_boot_lines {
        session.call_method("probe_device", (), Some(&kwargs))?
    } else {
        session.call_method0("probe_device")?
    };
    Ok(awaitable.unbind())
}

/// The census a session's ``probe_device`` returns: the concrete
/// type depends on the session's protocol.
pub(crate) struct DeviceCensus(Py<PyAny>);

impl<'py> IntoPyObject<'py> for DeviceCensus {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(self.0.into_bound(py))
    }
}

impl PyStubType for DeviceCensus {
    fn type_output() -> TypeInfo {
        // Qualified through each protocol's package, so the census
        // classes cannot collide on one import name.
        TypeInfo {
            name: "grbl.probe.GrblDeviceInfo \
                   | marlin.probe.MarlinDeviceInfo \
                   | smoothie.probe.SmoothieDeviceInfo \
                   | octoprint.probe.OctoPrintDeviceInfo"
                .to_string(),
            source_module: None,
            import: [
                "raygeo.driver.grbl",
                "raygeo.driver.marlin",
                "raygeo.driver.smoothie",
                "raygeo.driver.octoprint",
            ]
            .into_iter()
            .map(ImportRef::from)
            .collect(),
            type_refs: Default::default(),
        }
    }
}

/// Builds the census pyclass for *protocol* from its wire form.
pub(crate) fn census_from_json(
    py: Python<'_>,
    protocol: &str,
    value: &Value,
) -> PyResult<DeviceCensus> {
    let any: Py<PyAny> = match protocol {
        "GrblSerialDriver"
        | "GrblTelnetDriver"
        | "GrblBluetoothDriver"
        | "GrblNetworkDriver"
        | "GrblSerialSimpleDriver" => {
            let census = crate::driver::grbl::probe::GrblCensus::from_json(
                value.clone(),
            )
            .map_err(|err| {
                pyo3::exceptions::PyValueError::new_err(err.to_string())
            })?;
            Py::new(py, GrblDeviceInfo { inner: census })?.into_any()
        }
        "MarlinSerialDriver" => {
            let census = crate::driver::marlin::probe::MarlinCensus::from_json(
                value.clone(),
            )
            .map_err(|err| {
                pyo3::exceptions::PyValueError::new_err(err.to_string())
            })?;
            Py::new(py, MarlinDeviceInfo { inner: census })?.into_any()
        }
        "SmoothieDriver" => {
            let census =
                crate::driver::smoothie::probe::SmoothieCensus::from_json(
                    value.clone(),
                )
                .map_err(|err| {
                    pyo3::exceptions::PyValueError::new_err(err.to_string())
                })?;
            Py::new(py, SmoothieDeviceInfo { inner: census })?.into_any()
        }
        "OctoPrintDriver" => {
            let census =
                crate::driver::octoprint::probe::OctoPrintCensus::from_json(
                    value.clone(),
                )
                .map_err(|err| {
                    pyo3::exceptions::PyValueError::new_err(err.to_string())
                })?;
            Py::new(py, OctoPrintDeviceInfo { inner: census })?.into_any()
        }
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown driver {other:?}"
            )));
        }
    };
    Ok(DeviceCensus(any))
}
