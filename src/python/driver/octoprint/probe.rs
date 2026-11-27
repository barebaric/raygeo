//! The `raydriver.octoprint.probe` submodule: the device census.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::python::driver::census::call_session_probe;
use crate::python::driver::core::json::{py_to_value, value_to_py};

/// The structured result of probing an OctoPrint server: server
/// identity and the work area the server reports.
#[gen_stub_pyclass]
#[pyclass(frozen, module = "raygeo.driver.octoprint.probe")]
pub struct OctoPrintDeviceInfo {
    pub(crate) inner: crate::driver::octoprint::probe::OctoPrintCensus,
}

#[gen_stub_pymethods]
#[pymethods]
impl OctoPrintDeviceInfo {
    #[getter]
    fn server_version(&self) -> Option<String> {
        self.inner.server_version.clone()
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn max_travel(&self) -> Option<(f64, f64)> {
        self.inner.max_travel
    }

    /// The mapping form of the census (the daemon wire form).
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        Ok(value_to_py(py, &value)?.into_bound(py).cast_into()?)
    }
}

/// Builds the census from the parsed OctoPrint API payloads.
///
/// Pure data transformation with no I/O; the caller is responsible
/// for the HTTP requests. *version_info* is the `GET /api/version`
/// response, *printer_info* the `GET /api/printer` response
/// (either may be `None` when the request failed).
#[gen_stub_pyfunction(module = "raygeo.driver.octoprint.probe")]
#[pyfunction]
#[pyo3(signature = (version_info=None, printer_info=None))]
fn describe(
    #[gen_stub(override_type(
        type_repr = "typing.Optional[dict[builtins.str, builtins.object]]",
        imports = ("typing", "builtins")
    ))]
    version_info: Option<&Bound<'_, PyAny>>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[dict[builtins.str, builtins.object]]",
        imports = ("typing", "builtins")
    ))]
    printer_info: Option<&Bound<'_, PyAny>>,
) -> PyResult<OctoPrintDeviceInfo> {
    let version = match version_info {
        Some(value) if !value.is_none() => py_to_value(value)?,
        _ => serde_json::Value::Null,
    };
    let printer = match printer_info {
        Some(value) if !value.is_none() => py_to_value(value)?,
        _ => serde_json::Value::Null,
    };
    Ok(OctoPrintDeviceInfo {
        inner: crate::driver::octoprint::probe::describe(&version, &printer),
    })
}

/// Runs the device census over *session* and resolves to
/// [`OctoPrintDeviceInfo`].
#[gen_stub_pyfunction(module = "raygeo.driver.octoprint.probe")]
#[gen_stub(override_return_type(
    type_repr = "collections.abc.Awaitable[OctoPrintDeviceInfo]",
    imports = ("collections.abc",)
))]
#[pyfunction]
fn probe_device(py: Python<'_>, session: Py<PyAny>) -> PyResult<Py<PyAny>> {
    call_session_probe(py, session.bind(py), None)
}

pub fn register(octoprint_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = octoprint_mod.py();
    let module = PyModule::new(py, "probe")?;
    module.setattr("__doc__", "Device census for OctoPrint servers.")?;
    module.add_class::<OctoPrintDeviceInfo>()?;
    module.add_function(wrap_pyfunction!(describe, &module)?)?;
    module.add_function(wrap_pyfunction!(probe_device, &module)?)?;
    crate::python::driver::core::discovery::helpers::attach_submodule(
        octoprint_mod,
        &module,
        "raygeo.driver.octoprint.probe",
    )?;
    Ok(())
}
