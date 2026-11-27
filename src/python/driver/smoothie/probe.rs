//! The `raygeo.driver.smoothie.probe` submodule: the device census.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::python::driver::census::call_session_probe;
use crate::python::driver::core::json::value_to_py;

/// The structured result of probing a Smoothieware device: firmware
/// identity and the motion limits applications need.
#[gen_stub_pyclass]
#[pyclass(frozen, module = "raygeo.driver.smoothie.probe")]
pub struct SmoothieDeviceInfo {
    pub(crate) inner: crate::driver::smoothie::probe::SmoothieCensus,
}

#[gen_stub_pymethods]
#[pymethods]
impl SmoothieDeviceInfo {
    #[getter]
    fn firmware_version(&self) -> Option<String> {
        self.inner.firmware_version.clone()
    }

    #[getter]
    fn alpha_max(&self) -> Option<f64> {
        self.inner.alpha_max
    }

    #[getter]
    fn beta_max(&self) -> Option<f64> {
        self.inner.beta_max
    }

    #[getter]
    fn alpha_rate(&self) -> Option<f64> {
        self.inner.alpha_rate
    }

    #[getter]
    fn beta_rate(&self) -> Option<f64> {
        self.inner.beta_rate
    }

    #[getter]
    fn acceleration(&self) -> Option<f64> {
        self.inner.acceleration
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
    fn acceleration_steps(&self) -> Option<i64> {
        self.inner.acceleration_steps
    }

    /// The mapping form of the census (the daemon wire form).
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        Ok(value_to_py(py, &value)?.into_bound(py).cast_into()?)
    }
}

/// Builds the census from raw Smoothieware census-command responses.
///
/// Pure data transformation with no I/O; the caller is responsible
/// for communicating with the device and passing the collected
/// response lines. ``ok`` acknowledgements and ``<...>`` status
/// reports are dropped, and non-numeric ``config-get`` replies
/// yield ``None`` fields.
#[gen_stub_pyfunction(module = "raygeo.driver.smoothie.probe")]
#[pyfunction]
#[pyo3(signature = (version_lines, alpha_max_lines, beta_max_lines,
                    alpha_max_rate_lines, beta_max_rate_lines,
                    acceleration_lines))]
fn describe(
    version_lines: Vec<String>,
    alpha_max_lines: Vec<String>,
    beta_max_lines: Vec<String>,
    alpha_max_rate_lines: Vec<String>,
    beta_max_rate_lines: Vec<String>,
    acceleration_lines: Vec<String>,
) -> SmoothieDeviceInfo {
    SmoothieDeviceInfo {
        inner: crate::driver::smoothie::probe::describe(
            &version_lines,
            &alpha_max_lines,
            &beta_max_lines,
            &alpha_max_rate_lines,
            &beta_max_rate_lines,
            &acceleration_lines,
        ),
    }
}

/// Runs the device census over *session* and resolves to
/// [`SmoothieDeviceInfo`].
#[gen_stub_pyfunction(module = "raygeo.driver.smoothie.probe")]
#[gen_stub(override_return_type(
    type_repr = "collections.abc.Awaitable[SmoothieDeviceInfo]",
    imports = ("collections.abc",)
))]
#[pyfunction]
fn probe_device(py: Python<'_>, session: Py<PyAny>) -> PyResult<Py<PyAny>> {
    call_session_probe(py, session.bind(py), None)
}

pub fn register(smoothie_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = smoothie_mod.py();
    let module = PyModule::new(py, "probe")?;
    module.setattr("__doc__", "Device census for Smoothieware controllers.")?;
    module.add_class::<SmoothieDeviceInfo>()?;
    module.add_function(wrap_pyfunction!(describe, &module)?)?;
    module.add_function(wrap_pyfunction!(probe_device, &module)?)?;
    crate::python::driver::core::discovery::helpers::attach_submodule(
        smoothie_mod,
        &module,
        "raygeo.driver.smoothie.probe",
    )?;
    Ok(())
}
