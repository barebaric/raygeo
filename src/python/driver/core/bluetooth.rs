//! The `raygeo.driver.core.bluetooth` submodule: Bluetooth SPP
//! enumeration and address helpers for device pickers.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::driver::core::transport::bluetooth as core_bluetooth;
use crate::driver::core::transport::bluetooth::BluetoothDeviceInfo as CoreDeviceInfo;

use super::discovery::helpers;

/// A paired Bluetooth device offered for a Serial Port Profile
/// connection.
#[gen_stub_pyclass(module = "raygeo.driver.core.bluetooth")]
#[pyclass(
    module = "raygeo.driver.core.bluetooth",
    frozen,
    eq,
    hash,
    from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BluetoothDeviceInfo {
    pub(crate) inner: CoreDeviceInfo,
}

#[gen_stub_pymethods]
#[pymethods]
impl BluetoothDeviceInfo {
    #[new]
    #[pyo3(signature = (address, name=None, channel=None))]
    fn new(address: String, name: Option<String>, channel: Option<u8>) -> Self {
        Self {
            inner: CoreDeviceInfo {
                address,
                name,
                channel,
            },
        }
    }

    #[getter]
    fn address(&self) -> String {
        self.inner.address.clone()
    }

    #[getter]
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[getter]
    fn channel(&self) -> Option<u8> {
        self.inner.channel
    }

    fn __repr__(&self) -> String {
        format!(
            "BluetoothDeviceInfo(address={:?}, name={:?}, channel={:?})",
            self.inner.address, self.inner.name, self.inner.channel
        )
    }
}

/// Enumerate paired Bluetooth devices that can carry SPP/RFCOMM.
///
/// Backends that cannot query service records return every paired
/// device. Enumeration failures (no adapter, sandbox denial) yield an
/// empty list.
#[gen_stub_pyfunction(module = "raygeo.driver.core.bluetooth")]
#[pyfunction]
fn list_paired_devices() -> Vec<BluetoothDeviceInfo> {
    core_bluetooth::list_paired_devices()
        .into_iter()
        .map(|inner| BluetoothDeviceInfo { inner })
        .collect()
}

/// Whether this build has a Bluetooth SPP backend.
#[gen_stub_pyfunction(module = "raygeo.driver.core.bluetooth")]
#[pyfunction]
fn is_supported() -> bool {
    core_bluetooth::is_supported()
}

/// Normalizes a Bluetooth address to uppercase colon-separated form.
#[gen_stub_pyfunction(module = "raygeo.driver.core.bluetooth")]
#[pyfunction]
fn normalize_address(address: &str) -> PyResult<String> {
    core_bluetooth::normalize_address(address).map_err(PyValueError::new_err)
}

pub fn register(core_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = core_mod.py();
    let module = PyModule::new(py, "bluetooth")?;
    module
        .setattr("__doc__", "Bluetooth SPP enumeration and address helpers.")?;
    module.add(
        "__all__",
        vec![
            "BluetoothDeviceInfo",
            "is_supported",
            "list_paired_devices",
            "normalize_address",
        ],
    )?;
    module.add_class::<BluetoothDeviceInfo>()?;
    module.add_function(wrap_pyfunction!(list_paired_devices, &module)?)?;
    module.add_function(wrap_pyfunction!(is_supported, &module)?)?;
    module.add_function(wrap_pyfunction!(normalize_address, &module)?)?;
    helpers::attach_submodule(
        core_mod,
        &module,
        "raygeo.driver.core.bluetooth",
    )?;
    Ok(())
}
