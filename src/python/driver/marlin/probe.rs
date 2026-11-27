//! The `raygeo.driver.marlin.probe` submodule: the device census.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::python::driver::census::call_session_probe;
use crate::python::driver::core::json::value_to_py;

/// The structured result of probing a Marlin device: firmware
/// identity, software-endstop travel and motion limits.
#[gen_stub_pyclass]
#[pyclass(frozen, module = "raygeo.driver.marlin.probe")]
pub struct MarlinDeviceInfo {
    pub(crate) inner: crate::driver::marlin::probe::MarlinCensus,
}

#[gen_stub_pymethods]
#[pymethods]
impl MarlinDeviceInfo {
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn firmware_version(&self) -> Option<String> {
        self.inner.firmware_version.clone()
    }

    #[getter]
    fn machine_type(&self) -> Option<String> {
        self.inner.machine_type.clone()
    }

    #[getter]
    fn max_travel(&self) -> Option<(f64, f64)> {
        self.inner.max_travel
    }

    #[getter]
    fn max_speed(&self) -> Option<i64> {
        self.inner.max_speed
    }

    #[getter]
    fn acceleration(&self) -> Option<i64> {
        self.inner.acceleration
    }

    #[getter]
    fn units(&self) -> Option<String> {
        self.inner.units.clone()
    }

    /// The mapping form of the census (the daemon wire form).
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        Ok(value_to_py(py, &value)?.into_bound(py).cast_into()?)
    }
}

/// Builds the census from the raw Marlin census-command responses.
///
/// Pure data transformation with no I/O; the caller is responsible
/// for communicating with the device and passing the collected
/// response lines. `boot_lines` are startup messages observed
/// before the handshake; a version found there is more specific
/// than the `FIRMWARE_NAME` token M115 reports.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.probe")]
#[pyfunction]
#[pyo3(signature = (m115_lines, m211_lines, m503_lines, m149_lines,
                    boot_lines=None))]
fn describe(
    m115_lines: Vec<String>,
    m211_lines: Vec<String>,
    m503_lines: Vec<String>,
    m149_lines: Vec<String>,
    boot_lines: Option<Vec<String>>,
) -> MarlinDeviceInfo {
    MarlinDeviceInfo {
        inner: crate::driver::marlin::probe::describe(
            &m115_lines,
            &m211_lines,
            &m503_lines,
            &m149_lines,
            &boot_lines.unwrap_or_default(),
        ),
    }
}

/// Runs the device census over *session* and resolves to
/// [`MarlinDeviceInfo`]. *boot_lines* are the startup lines the
/// caller observed.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.probe")]
#[gen_stub(override_return_type(
    type_repr = "collections.abc.Awaitable[MarlinDeviceInfo]",
    imports = ("collections.abc",)
))]
#[pyfunction]
#[pyo3(signature = (session, boot_lines=None))]
fn probe_device(
    py: Python<'_>,
    session: Py<PyAny>,
    boot_lines: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    call_session_probe(py, session.bind(py), boot_lines)
}

pub fn register(marlin_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = marlin_mod.py();
    let module = PyModule::new(py, "probe")?;
    module.setattr("__doc__", "Device census for Marlin controllers.")?;
    module.add_class::<MarlinDeviceInfo>()?;
    module.add_function(wrap_pyfunction!(describe, &module)?)?;
    module.add_function(wrap_pyfunction!(probe_device, &module)?)?;
    crate::python::driver::core::discovery::helpers::attach_submodule(
        marlin_mod,
        &module,
        "raygeo.driver.marlin.probe",
    )?;
    Ok(())
}
