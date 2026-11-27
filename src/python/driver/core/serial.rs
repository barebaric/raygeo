//! Serial port enumeration for device pickers and the remote
//! daemon's device listing.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use serialport::SerialPortType;

use crate::driver::core::transport::serial;

/// Enumerate the serial ports this host can reach.
///
/// Returns one dict per port with the keys ``port`` (the OS device
/// path), ``vid``, ``pid``, ``serial_number``, ``manufacturer`` and
/// ``product``. The USB fields are ``None`` for non-USB ports.
#[gen_stub_pyfunction(module = "raygeo.driver.core")]
#[pyfunction]
pub(crate) fn list_serial_ports(
    py: Python<'_>,
) -> PyResult<Vec<Bound<'_, PyDict>>> {
    let ports = serial::list_serial_ports().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
    })?;
    let mut out = Vec::with_capacity(ports.len());
    for info in ports {
        let usb = match &info.port_type {
            SerialPortType::UsbPort(usb) => Some(usb),
            _ => None,
        };
        let dict = PyDict::new(py);
        dict.set_item("port", &info.port_name)?;
        dict.set_item("vid", usb.map(|u| u.vid))?;
        dict.set_item("pid", usb.map(|u| u.pid))?;
        dict.set_item(
            "serial_number",
            usb.and_then(|u| u.serial_number.clone()),
        )?;
        dict.set_item(
            "manufacturer",
            usb.and_then(|u| u.manufacturer.clone()),
        )?;
        dict.set_item("product", usb.and_then(|u| u.product.clone()))?;
        out.push(dict);
    }
    Ok(out)
}
