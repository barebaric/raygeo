//! The `raygeo.driver.core.discovery.session` submodule: GTK-free state
//! behind a discovery UI.
//!
//! A `DiscoverySession` owns everything about an ongoing discovery
//! that is not widget work: the known devices, the ports held by
//! found devices, scan-result reconciliation and probe-result
//! enrichment. A UI page drives it (feeds scans in) and maps its
//! results onto rows.

use std::collections::HashSet;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::helpers;
use super::types::DiscoveredDevice;

/// Accumulates discovered devices across repeated scans and reports
/// the difference between scans as plain data:
///
/// * `apply_scan` returns ``(added, removed_keys)`` — devices are
///   matched by their stable `DiscoveredDevice.key`.
/// * Serial ports of found devices (those without a ``host``
///   parameter) are remembered as *held*; held ports are meant to be
///   excluded from rescans (re-opening them would reset some boards)
///   and their devices stay listed even though a rescan cannot see
///   them. They leave only via `prune_absent_ports` when the port
///   vanishes from the system.
/// * `apply_probe` attaches a probed profile to a device and returns
///   the enriched device.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.session")]
#[pyclass(module = "raygeo.driver.core.discovery.session")]
#[derive(Default)]
pub struct DiscoverySession {
    /// Known devices in first-seen order, keyed by device key.
    devices: Vec<(String, Py<DiscoveredDevice>)>,
    held_ports: HashSet<String>,
}

impl DiscoverySession {
    fn find(&self, key: &str) -> Option<&Py<DiscoveredDevice>> {
        self.devices
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, device)| device)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl DiscoverySession {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Ports of known devices; exclude from rescans.
    #[getter]
    fn held_ports<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PySet>> {
        PySet::new(py, self.held_ports.iter())
    }

    #[getter]
    fn devices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let list = PyList::empty(py);
        for (_, device) in &self.devices {
            list.append(device.bind(py))?;
        }
        Ok(list)
    }

    fn get(&self, key: &str) -> Option<Py<DiscoveredDevice>> {
        self.find(key).cloned()
    }

    /// Forgets everything (e.g. when the UI re-enters discovery).
    fn reset(&mut self) {
        self.devices.clear();
        self.held_ports.clear();
    }

    /// Reconciles the known devices with a fresh scan result.
    /// Returns the newly seen devices and the keys of devices that
    /// disappeared (excluding those on held ports).
    fn apply_scan(
        &mut self,
        py: Python<'_>,
        devices: Vec<Py<DiscoveredDevice>>,
    ) -> PyResult<(Vec<Py<DiscoveredDevice>>, Vec<String>)> {
        let mut added = Vec::new();
        let mut seen = HashSet::new();
        for device in devices {
            let key: String = device.getattr(py, "key")?.extract(py)?;
            if !seen.insert(key.clone()) {
                continue;
            }
            let params = params_dict(py, &device)?;
            if let Some(port) = serial_port_of(&params)? {
                self.held_ports.insert(port);
            }
            if self.find(&key).is_none() {
                self.devices.push((key, device.clone_ref(py)));
                added.push(device);
            }
        }

        let mut removed = Vec::new();
        for (key, device) in &self.devices {
            if seen.contains(key) {
                continue;
            }
            let params = params_dict(py, device)?;
            let on_held = serial_port_of(&params)?
                .map(|port| self.held_ports.contains(&port))
                .unwrap_or(false);
            if !on_held {
                removed.push(key.clone());
            }
        }
        self.devices.retain(|(key, _)| !removed.contains(key));
        Ok((added, removed))
    }

    /// Attaches a probed profile to the device with *key* and
    /// returns the enriched device, or None when the key is unknown.
    fn apply_probe(
        &mut self,
        py: Python<'_>,
        key: &str,
        #[gen_stub(override_type(type_repr = "builtins.object", imports = ("builtins",)))]
        profile: Py<PyAny>,
    ) -> PyResult<Option<Py<DiscoveredDevice>>> {
        let Some(device) = self.find(key) else {
            return Ok(None);
        };
        device.setattr(py, "probe_profile", profile)?;
        Ok(Some(device.clone_ref(py)))
    }

    /// Drops devices whose serial port is no longer present on the
    /// system. Returns the removed keys.
    fn prune_absent_ports(
        &mut self,
        py: Python<'_>,
        #[gen_stub(override_type(type_repr = "typing.Iterable[builtins.str]", imports = ("typing", "builtins")))]
        present_ports: &Bound<'_, PyAny>,
    ) -> PyResult<Vec<String>> {
        let present: HashSet<String> = present_ports
            .try_iter()?
            .map(|item| item?.extract::<String>())
            .collect::<PyResult<_>>()?;
        let mut removed = Vec::new();
        for (key, device) in &self.devices {
            let params = params_dict(py, device)?;
            if let Some(port) = serial_port_of(&params)? {
                if !present.contains(&port) {
                    removed.push(key.clone());
                }
            }
        }
        for key in &removed {
            if let Some(index) = self.devices.iter().position(|(k, _)| k == key)
            {
                let (_, device) = self.devices.remove(index);
                let params = params_dict(py, &device)?;
                if let Some(port) = serial_port_of(&params)? {
                    self.held_ports.remove(&port);
                }
            }
        }
        Ok(removed)
    }
}

fn params_dict<'py>(
    py: Python<'py>,
    device: &Py<DiscoveredDevice>,
) -> PyResult<Bound<'py, PyDict>> {
    Ok(device.getattr(py, "params")?.into_bound(py).cast_into()?)
}

/// The serial port path carried by *params*, or ``None`` when the
/// device is not on a serial port. Whether a device is network (and
/// thus never held/pruned) is decided by its ``host`` parameter, not
/// by the type of its ``port``.
fn serial_port_of(params: &Bound<'_, PyDict>) -> PyResult<Option<String>> {
    if params.contains("host")? {
        return Ok(None);
    }
    match params.get_item("port")? {
        Some(value) => {
            let Ok(port) = value.extract::<String>() else {
                return Ok(None);
            };
            if port.is_empty() {
                Ok(None)
            } else {
                Ok(Some(port))
            }
        }
        None => Ok(None),
    }
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "session")?;
    module.setattr(
        "__doc__",
        "GTK-free discovery state: scan reconciliation, held ports \
         and probe enrichment.",
    )?;
    module.add_class::<DiscoverySession>()?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.session",
    )?;
    Ok(())
}
