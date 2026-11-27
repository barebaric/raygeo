//! The `raygeo.driver.core.discovery.ports` submodule: serial port
//! enumeration for discovery and device pickers.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};
use pyo3_stub_gen::{PyStubType, TypeInfo};

use crate::driver::core::discovery::ports as core_ports;
use crate::driver::core::discovery::ports::SerialPortInfo as CorePortInfo;

use super::helpers;

/// A replaceable platform-enumeration callable: the listing
/// functions resolve it from the module namespace at call time (see
/// helpers), so the attribute is typed as a callable resolving to
/// one port dict per port.
pub(crate) struct ListSerialPortsFn;

impl PyStubType for ListSerialPortsFn {
    fn type_output() -> TypeInfo {
        TypeInfo {
            name: "collections.abc.Callable[[], \
                   builtins.list[dict[builtins.str, builtins.object]]]"
                .to_string(),
            source_module: None,
            import: ["collections.abc".into(), "builtins".into()]
                .into_iter()
                .collect(),
            type_refs: Default::default(),
        }
    }
}

/// A serial port path together with optional metadata used to
/// identify the underlying USB device.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.ports")]
#[pyclass(
    module = "raygeo.driver.core.discovery.ports",
    frozen,
    eq,
    hash,
    from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SerialPortInfo {
    pub(crate) inner: CorePortInfo,
}

#[gen_stub_pymethods]
#[pymethods]
impl SerialPortInfo {
    #[new]
    #[pyo3(signature = (device, description=None, manufacturer=None, vid=None, pid=None))]
    fn new(
        device: String,
        description: Option<String>,
        manufacturer: Option<String>,
        vid: Option<u16>,
        pid: Option<u16>,
    ) -> Self {
        Self {
            inner: CorePortInfo {
                device,
                description,
                manufacturer,
                vid,
                pid,
            },
        }
    }

    #[getter]
    fn device(&self) -> String {
        self.inner.device.clone()
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    #[getter]
    fn manufacturer(&self) -> Option<String> {
        self.inner.manufacturer.clone()
    }

    #[getter]
    fn vid(&self) -> Option<u16> {
        self.inner.vid
    }

    #[getter]
    fn pid(&self) -> Option<u16> {
        self.inner.pid
    }

    fn __repr__(&self) -> String {
        format!(
            "SerialPortInfo(device={:?}, description={:?}, vid={:?}, pid={:?})",
            self.inner.device,
            self.inner.description,
            self.inner.vid,
            self.inner.pid
        )
    }
}

/// Heuristically determines if a port path refers to a USB serial
/// device. On non-POSIX systems (e.g. Windows), every port is treated
/// as potentially USB.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn is_usb_serial_port(port: &str) -> bool {
    core_ports::is_usb_serial_port(port)
}

/// Splits a string into lowercase words and numbers so ttyUSB2 sorts
/// before ttyUSB10.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn natural_key<'py>(py: Python<'py>, s: &str) -> PyResult<Bound<'py, PyList>> {
    let parts = core_ports::natural_key(s);
    let list = PyList::empty(py);
    for part in parts {
        match part {
            core_ports::KeyPart::Num(n) => list.append(n)?,
            core_ports::KeyPart::Text(t) => list.append(t)?,
        }
    }
    Ok(list)
}

/// Sorts ports so USB serial adapters come first, followed by all
/// other ports. Each group is ordered naturally, e.g. ttyUSB2 before
/// ttyUSB10.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn sort_ports(ports: Vec<String>) -> Vec<String> {
    core_ports::sort_ports(ports)
}

/// Glob-style prefix listing of one directory; the enumeration
/// backend used by the sandbox-safe listing. Exposed as a module
/// attribute so tests can replace it.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn glob_ports(dir: &str, prefix: &str) -> Vec<String> {
    core_ports::glob_prefix(dir, prefix)
}

/// A non-crashing port listing for sandboxed Linux envs.
///
/// Direct enumeration of /dev/ttyS* is forbidden by the snap sandbox
/// and crashes pyserial-style scanners; only the common USB-to-serial
/// patterns permitted by the serial-port interface are globbed.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn safe_list_ports_linux(py: Python<'_>) -> PyResult<Vec<String>> {
    let glob = helpers::attr(py, "ports", "glob_ports")?;
    let mut ports = Vec::new();
    for (dir, prefix) in [
        ("/dev", "ttyUSB"),
        ("/dev", "ttyACM"),
        ("/dev", "rfcomm"),
        ("/dev/serial/by-id", ""),
        ("/dev/serial/by-path", ""),
    ] {
        let found: Vec<String> = glob.call1((dir, prefix))?.extract()?;
        ports.extend(found);
    }
    ports.sort();
    Ok(ports)
}

fn snap_paths(py: Python<'_>) -> PyResult<Vec<String>> {
    helpers::attr(py, "ports", "safe_list_ports_linux")?
        .call0()?
        .extract()
}

fn platform_dicts<'py>(py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
    helpers::attr(py, "ports", "list_serial_ports")?
        .call0()?
        .extract()
}

fn dict_str(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<String>> {
    match dict.get_item(key)? {
        Some(value) if !value.is_none() => Ok(Some(value.extract()?)),
        _ => Ok(None),
    }
}

fn dict_u16(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<u16>> {
    match dict.get_item(key)? {
        Some(value) if !value.is_none() => Ok(Some(value.extract()?)),
        _ => Ok(None),
    }
}

fn sort_infos(infos: &mut [SerialPortInfo]) {
    infos.sort_by_key(|i| {
        (
            !core_ports::is_usb_serial_port(&i.inner.device),
            core_ports::natural_key(&i.inner.device),
        )
    });
}

/// Lists available serial ports, USB adapters first.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn list_ports(py: Python<'_>) -> PyResult<Vec<String>> {
    if core_ports::in_snap() {
        return Ok(core_ports::sort_ports(snap_paths(py)?));
    }
    let dicts = match platform_dicts(py) {
        Ok(dicts) => dicts,
        Err(e) => {
            helpers::log_exception(
                py,
                "raygeo.driver.core.discovery.ports",
                "Failed to list serial ports",
                &e,
            );
            return Ok(Vec::new());
        }
    };
    let mut ports = Vec::with_capacity(dicts.len());
    for dict in dicts {
        ports.push(dict_str(&dict, "port")?.unwrap_or_default());
    }
    Ok(core_ports::sort_ports(ports))
}

/// Lists available serial ports together with a human-readable
/// description when the platform provides one. USB adapters are
/// listed first.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn list_port_info(py: Python<'_>) -> PyResult<Vec<SerialPortInfo>> {
    if core_ports::in_snap() {
        let paths = snap_paths(py)?;
        let descriptions = core_ports::by_id_descriptions(&paths);
        let mut infos: Vec<SerialPortInfo> = paths
            .into_iter()
            .map(|path| SerialPortInfo {
                inner: CorePortInfo {
                    description: descriptions.get(&path).cloned(),
                    device: path,
                    ..Default::default()
                },
            })
            .collect();
        sort_infos(&mut infos);
        return Ok(infos);
    }

    let dicts = match platform_dicts(py) {
        Ok(dicts) => dicts,
        Err(e) => {
            helpers::log_exception(
                py,
                "raygeo.driver.core.discovery.ports",
                "Failed to list serial ports",
                &e,
            );
            return Ok(Vec::new());
        }
    };
    let mut infos = Vec::with_capacity(dicts.len());
    for dict in dicts {
        let description = dict_str(&dict, "product")?.filter(|d| d != "n/a");
        infos.push(SerialPortInfo {
            inner: CorePortInfo {
                device: dict_str(&dict, "port")?.unwrap_or_default(),
                description,
                manufacturer: dict_str(&dict, "manufacturer")?,
                vid: dict_u16(&dict, "vid")?,
                pid: dict_u16(&dict, "pid")?,
            },
        });
    }
    sort_infos(&mut infos);
    Ok(infos)
}

/// Like list_ports, but only returns USB serial ports.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn list_usb_ports(py: Python<'_>) -> PyResult<Vec<String>> {
    let all: Vec<String> = helpers::attr(py, "ports", "list_ports")?
        .call0()?
        .extract()?;
    if !cfg!(target_family = "unix") {
        // On non-POSIX systems, we can't reliably filter, so return all.
        return Ok(all);
    }
    Ok(all
        .into_iter()
        .filter(|p| core_ports::is_usb_serial_port(p))
        .collect())
}

/// The common serial baud rates offered to applications.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn list_baud_rates() -> Vec<u32> {
    core_ports::list_baud_rates()
}

/// Resolves a USB `vid:pid` pair (e.g. FTDI 0x0403/0x6001) to a
/// concrete device path by scanning the available ports; `None`
/// when no matching device is present.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn resolve_vidpid(
    py: Python<'_>,
    vid: u16,
    pid: u16,
) -> PyResult<Option<String>> {
    let infos = helpers::attr(py, "ports", "list_port_info")?.call0()?;
    let infos: Vec<SerialPortInfo> = infos.extract()?;
    let core_infos: Vec<CorePortInfo> =
        infos.iter().map(|info| info.inner.clone()).collect();
    Ok(core_ports::resolve_vidpid_in(&core_infos, vid, pid))
}

/// Diagnoses systemic serial-port access problems (missing
/// `dialout` group membership, snap confinement). Returns a
/// structured result; the application supplies the localized
/// message.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.ports")]
#[pyfunction]
fn check_permission_issue(py: Python<'_>) -> PyResult<String> {
    let ports: Vec<String> = helpers::attr(py, "ports", "list_usb_ports")?
        .call0()?
        .extract()?;
    Ok(core_ports::check_permission_issue_over(&ports)
        .as_str()
        .to_string())
}

pub fn register(
    discovery_mod: &Bound<'_, PyModule>,
    core_mod: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "ports")?;
    module.setattr(
        "__doc__",
        "Serial port enumeration for discovery and device pickers.",
    )?;
    // The platform enumeration backend, replaceable as a module
    // attribute (tests patch it with canned port lists).
    module
        .setattr("list_serial_ports", core_mod.getattr("list_serial_ports")?)?;
    module.add_class::<SerialPortInfo>()?;
    module.add_function(wrap_pyfunction!(is_usb_serial_port, &module)?)?;
    module.add_function(wrap_pyfunction!(natural_key, &module)?)?;
    module.add_function(wrap_pyfunction!(sort_ports, &module)?)?;
    module.add_function(wrap_pyfunction!(glob_ports, &module)?)?;
    module.add_function(wrap_pyfunction!(safe_list_ports_linux, &module)?)?;
    module.add_function(wrap_pyfunction!(list_ports, &module)?)?;
    module.add_function(wrap_pyfunction!(list_port_info, &module)?)?;
    module.add_function(wrap_pyfunction!(list_usb_ports, &module)?)?;
    module.add_function(wrap_pyfunction!(list_baud_rates, &module)?)?;
    module.add_function(wrap_pyfunction!(resolve_vidpid, &module)?)?;
    module.add_function(wrap_pyfunction!(check_permission_issue, &module)?)?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.ports",
    )?;
    Ok(())
}
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.ports",
    "list_serial_ports",
    ListSerialPortsFn
);
