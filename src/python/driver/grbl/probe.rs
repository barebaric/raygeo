//! The `raygeo.driver.grbl.probe` submodule: the device census.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pyfunction,
    gen_stub_pymethods,
};

use crate::python::driver::census::call_session_probe;
use crate::python::driver::core::json::value_to_py;

/// The firmware family a device identifies itself as.
#[gen_stub_pyclass_enum]
#[pyclass(eq, skip_from_py_object, module = "raygeo.driver.grbl.probe")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrblFlavor {
    #[pyo3(name = "STOCK")]
    Stock,
    #[pyo3(name = "GRBLHAL")]
    Grblhal,
    #[pyo3(name = "FLUIDNC")]
    Fluidnc,
    #[pyo3(name = "UNKNOWN")]
    Unknown,
}

impl GrblFlavor {
    fn from_name(name: &str) -> Self {
        match name {
            "stock" => Self::Stock,
            "grblhal" => Self::Grblhal,
            "fluidnc" => Self::Fluidnc,
            _ => Self::Unknown,
        }
    }
}

/// The structured result of probing a GRBL device: firmware
/// identity, compile flags, flavor classification and the semantic
/// values applications need.
#[gen_stub_pyclass]
#[pyclass(frozen, module = "raygeo.driver.grbl.probe")]
pub struct GrblDeviceInfo {
    pub(crate) inner: crate::driver::grbl::probe::GrblCensus,
}

#[gen_stub_pymethods]
#[pymethods]
impl GrblDeviceInfo {
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn firmware_version(&self) -> Option<String> {
        self.inner.firmware_version.clone()
    }

    #[getter]
    fn rx_buffer_size(&self) -> Option<i64> {
        self.inner.rx_buffer_size
    }

    #[getter]
    fn compile_flags(&self) -> String {
        self.inner.compile_flags.clone()
    }

    #[getter]
    fn flavor(&self) -> GrblFlavor {
        GrblFlavor::from_name(&self.inner.flavor)
    }

    #[getter]
    fn settings(&self) -> std::collections::HashMap<String, f64> {
        self.inner.settings.clone()
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
    fn max_spindle(&self) -> Option<i64> {
        self.inner.max_spindle
    }

    #[getter]
    fn laser_mode(&self) -> Option<bool> {
        self.inner.laser_mode
    }

    #[getter]
    fn homing_enabled(&self) -> Option<bool> {
        self.inner.homing_enabled
    }

    #[getter]
    fn report_inches(&self) -> Option<bool> {
        self.inner.report_inches
    }

    #[getter]
    fn arc_tolerance(&self) -> Option<f64> {
        self.inner.arc_tolerance
    }

    #[getter]
    fn single_axis_homing(&self) -> bool {
        self.inner.single_axis_homing
    }

    /// The mapping form of the census (the daemon wire form).
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        Ok(value_to_py(py, &value)?.into_bound(py).cast_into()?)
    }
}

/// Builds the census from the raw Grbl `$I` and `$$` response
/// lines.
///
/// Pure data transformation with no I/O; the caller is responsible
/// for communicating with the device and passing the collected
/// response lines.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.probe")]
#[pyfunction]
fn describe(
    build_info: Vec<String>,
    settings_lines: Vec<String>,
) -> GrblDeviceInfo {
    GrblDeviceInfo {
        inner: crate::driver::grbl::probe::describe(
            &build_info,
            &settings_lines,
        ),
    }
}

/// Runs the device census over *session* and resolves to
/// [`GrblDeviceInfo`].
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.probe")]
#[gen_stub(override_return_type(
    type_repr = "collections.abc.Awaitable[GrblDeviceInfo]",
    imports = ("collections.abc",)
))]
#[pyfunction]
fn probe_device(py: Python<'_>, session: Py<PyAny>) -> PyResult<Py<PyAny>> {
    call_session_probe(py, session.bind(py), None)
}

pub fn register(grbl_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = grbl_mod.py();
    let module = PyModule::new(py, "probe")?;
    module.setattr("__doc__", "Device census for GRBL controllers.")?;
    module.add_class::<GrblFlavor>()?;
    module.add_class::<GrblDeviceInfo>()?;
    module.add_function(wrap_pyfunction!(describe, &module)?)?;
    module.add_function(wrap_pyfunction!(probe_device, &module)?)?;
    crate::python::driver::core::discovery::helpers::attach_submodule(
        grbl_mod,
        &module,
        "raygeo.driver.grbl.probe",
    )?;
    Ok(())
}
