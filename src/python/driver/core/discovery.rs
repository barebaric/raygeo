//! The `raydriver.core.discovery` Python submodule: transport-level
//! device discovery scanners (serial capture, mDNS browse, subnet
//! TCP scan), the discovery result types, and the driver-facing
//! discovery subsystem (declarations, channels, engine, session).

mod daemon;
mod engine;
mod fingerprints;
pub(crate) mod helpers;
mod mdns_channel;
mod mdns_scan;
mod ports;
mod raygeo_channel;
mod serial_channel;
mod serial_scan;
mod session;
mod spec;
mod types;

use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::core::discovery::types::{
    DeviceIdentity as CoreIdentity, DiscoveredDevice as CoreDevice,
};
use crate::driver::core::discovery::{mdns, serial, subnet};
use crate::python::driver::core::json::JsonDict;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};
use types::{DeviceIdentity, DiscoveredDevice};

/// Probe serial ports for device output.
///
/// Opens each port once, tries the baud rates in order and captures
/// everything a talking device sends. Returns one dict per talking
/// port with the keys ``port``, ``baud_rate`` and ``data`` (bytes).
/// ``timeout`` bounds the whole scan; observations collected so far
/// are still returned when it expires.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery")]
#[pyfunction]
#[pyo3(signature = (ports, baud_rates=None, timeout=None))]
fn scan_serial_ports(
    py: Python<'_>,
    ports: Vec<String>,
    baud_rates: Option<Vec<u32>>,
    timeout: Option<f64>,
) -> PyResult<PyAwait<'_, Vec<JsonDict>>> {
    let rates = baud_rates.unwrap_or_default();
    let deadline = timeout.map(Duration::from_secs_f64);
    spawn_future(py, async move {
        let observations =
            serial::scan_serial_ports(ports, rates, deadline).await;
        Python::attach(|py| {
            let mut out = Vec::with_capacity(observations.len());
            for observation in observations {
                let dict = PyDict::new(py);
                dict.set_item("port", observation.port)?;
                dict.set_item("baud_rate", observation.baud_rate)?;
                dict.set_item("data", PyBytes::new(py, &observation.data))?;
                out.push(JsonDict(dict.unbind()));
            }
            Ok(out)
        })
    })
}

/// Browse mDNS for the given service types.
///
/// Returns one dict per resolved service with the keys
/// ``service_type`` (normalized, e.g. ``_http._tcp``), ``name``,
/// ``host``, ``port``, ``server`` and ``txt`` (a str->str dict).
/// ``browse_window`` is the listen time in seconds.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery")]
#[pyfunction]
#[pyo3(signature = (service_types, browse_window=2.0))]
fn scan_mdns_services(
    py: Python<'_>,
    service_types: Vec<String>,
    browse_window: f64,
) -> PyResult<PyAwait<'_, Vec<JsonDict>>> {
    let window = Duration::from_secs_f64(browse_window);
    spawn_future(py, async move {
        let services = mdns::scan_mdns_services(service_types, window).await;
        Python::attach(|py| {
            let mut out = Vec::with_capacity(services.len());
            for service in services {
                let dict = PyDict::new(py);
                dict.set_item("service_type", service.service_type)?;
                dict.set_item("name", service.name)?;
                dict.set_item("host", service.host)?;
                dict.set_item("port", service.port)?;
                dict.set_item("server", service.server)?;
                dict.set_item("txt", service.txt)?;
                out.push(JsonDict(dict.unbind()));
            }
            Ok(out)
        })
    })
}

/// The local non-loopback IPv4 interfaces as (address, prefixlen)
/// pairs, for subnet scans.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery")]
#[pyfunction]
fn local_subnets() -> Vec<(String, u8)> {
    subnet::local_subnets()
        .into_iter()
        .map(|(addr, prefix)| (addr.to_string(), prefix))
        .collect()
}

/// The host addresses to probe on the subnet containing *address*.
///
/// Networks wider than /24 are narrowed to the /24 around the
/// address; network and broadcast addresses are excluded.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery")]
#[pyfunction]
fn subnet_hosts(address: &str, prefixlen: u8) -> PyResult<Vec<String>> {
    let addr: std::net::Ipv4Addr = address.parse().map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "not an IPv4 address: {address}"
        ))
    })?;
    Ok(subnet::subnet_hosts(addr, prefixlen))
}

/// TCP-connect to *port* on every host and return the hosts that
/// accept a connection, in input order.
///
/// At most *concurrency* connects run in flight, each bounded by
/// *connect_timeout* seconds. Intended for explicit, user-requested
/// scans only.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery")]
#[pyfunction]
#[pyo3(signature = (hosts, port, connect_timeout=0.5, concurrency=32))]
fn scan_subnet(
    py: Python<'_>,
    hosts: Vec<String>,
    port: u16,
    connect_timeout: f64,
    concurrency: usize,
) -> PyResult<PyAwait<'_, Vec<String>>> {
    let timeout = Duration::from_secs_f64(connect_timeout);
    spawn_future(py, async move {
        let reachable =
            subnet::scan_subnet(hosts, port, timeout, concurrency).await;
        Ok(reachable)
    })
}

/// Scan the local subnets for declared TCP ports and confirm each
/// reachable host with its spec's probe.
///
/// *specs* is a list of dicts with the keys ``id`` (opaque caller
/// identifier), ``port``, and optionally ``probe_command`` (sent to
/// confirm the firmware) and ``match_prefix`` (response-line prefix
/// identifying it), plus the declaring driver's display fields
/// (``driver_name``, ``label``, ``firmware``). Without a probe command
/// any listener on the port is claimed. Returns one
/// :class:`DiscoveredDevice` per claimed host; the first matching
/// spec claims a host.
///
/// *subnets* overrides the interface enumeration as (address,
/// prefixlen) pairs; None scans the local interfaces.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery")]
#[pyfunction]
#[pyo3(signature = (specs, subnets=None))]
fn find_subnet_devices<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Sequence[dict[builtins.str, builtins.object]]", imports = ("typing", "builtins")))]
    specs: Vec<Bound<'py, PyDict>>,
    subnets: Option<Vec<(String, u8)>>,
) -> PyResult<PyAwait<'py, Vec<DiscoveredDevice>>> {
    let mut parsed = Vec::with_capacity(specs.len());
    let mut display = Vec::with_capacity(specs.len());
    for spec in &specs {
        parsed.push(subnet::SubnetSpec {
            id: required_str(spec, "id")?,
            port: required_u16(spec, "port")?,
            probe_command: optional_str(spec, "probe_command")?,
            match_prefix: optional_str(spec, "match_prefix")?,
        });
        display.push((
            required_str(spec, "id")?,
            required_str(spec, "driver_name")?,
            required_str(spec, "label")?,
            optional_str(spec, "firmware")?,
        ));
    }
    let subnets = subnets
        .map(|pairs| {
            pairs
                .into_iter()
                .map(|(addr, prefix)| {
                    addr.parse::<std::net::Ipv4Addr>()
                        .map(|addr| (addr, prefix))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "invalid subnet address: {e}"
            ))
        })?;
    spawn_future(py, async move {
        let hits = subnet::find_subnet_devices(parsed, subnets).await;
        Python::attach(|py| {
            let mut out = Vec::with_capacity(hits.len());
            for hit in hits {
                let Some((_, driver_name, label, firmware)) =
                    display.iter().find(|(id, ..)| *id == hit.id)
                else {
                    continue;
                };
                let params = PyDict::new(py);
                params.set_item("host", &hit.host)?;
                params.set_item("port", hit.port)?;
                out.push(DiscoveredDevice {
                    inner: CoreDevice {
                        driver_name: driver_name.clone(),
                        params: types::params_from_py(&params)?,
                        label: label.clone(),
                        detail: format!("{}:{}", hit.host, hit.port),
                        identity: CoreIdentity {
                            firmware: firmware.clone(),
                            banner: hit.banner.clone(),
                            ..Default::default()
                        },
                    },
                    probe_profile: None,
                });
            }
            Ok(out)
        })
    })
}

/// Stable identity for a device from its driver and connection
/// parameters: serial ports by path, network devices by host and
/// TCP port, remote devices by daemon endpoint plus device resource.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery")]
#[pyfunction]
fn device_key(
    driver_name: &str,
    #[gen_stub(override_type(type_repr = "dict[builtins.str, builtins.object]", imports = ("builtins",)))]
    params: &Bound<'_, PyDict>,
) -> PyResult<String> {
    Ok(crate::driver::core::discovery::types::device_key(
        driver_name,
        &types::params_from_py(params)?,
    ))
}

fn required_str(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    dict.get_item(key)?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key.to_string()))?
        .extract()
}

fn required_u16(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<u16> {
    dict.get_item(key)?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key.to_string()))?
        .extract()
}

fn optional_str(
    dict: &Bound<'_, PyDict>,
    key: &str,
) -> PyResult<Option<String>> {
    match dict.get_item(key)? {
        Some(value) => Ok(Some(value.extract()?)),
        None => Ok(None),
    }
}

pub fn register(core_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = core_mod.py();
    let discovery_mod = PyModule::new(py, "discovery")?;
    discovery_mod.setattr(
        "__doc__",
        "Device discovery: transport-level scanners (serial capture, \
         mDNS browse, subnet TCP scan), result types, and the \
         driver-facing subsystem (declarations, channels, engine, \
         session).",
    )?;
    discovery_mod.add(
        "__all__",
        vec![
            "scan_serial_ports",
            "scan_mdns_services",
            "local_subnets",
            "subnet_hosts",
            "scan_subnet",
            "find_subnet_devices",
            "DeviceIdentity",
            "DiscoveredDevice",
            "device_key",
            "ports",
            "spec",
            "fingerprints",
            "serial_scan",
            "mdns_scan",
            "serial_channel",
            "mdns_channel",
            "raygeo_channel",
            "daemon",
            "engine",
            "session",
        ],
    )?;
    discovery_mod
        .add_function(wrap_pyfunction!(scan_serial_ports, &discovery_mod)?)?;
    discovery_mod
        .add_function(wrap_pyfunction!(scan_mdns_services, &discovery_mod)?)?;
    discovery_mod
        .add_function(wrap_pyfunction!(local_subnets, &discovery_mod)?)?;
    discovery_mod
        .add_function(wrap_pyfunction!(subnet_hosts, &discovery_mod)?)?;
    discovery_mod
        .add_function(wrap_pyfunction!(scan_subnet, &discovery_mod)?)?;
    discovery_mod
        .add_function(wrap_pyfunction!(find_subnet_devices, &discovery_mod)?)?;
    discovery_mod
        .add_function(wrap_pyfunction!(device_key, &discovery_mod)?)?;
    discovery_mod.add_class::<DeviceIdentity>()?;
    discovery_mod.add_class::<DiscoveredDevice>()?;
    // Register under the public name before the submodules, whose
    // registration resolves collaborators through it.
    helpers::attach_submodule(
        core_mod,
        &discovery_mod,
        "raygeo.driver.core.discovery",
    )?;
    ports::register(&discovery_mod, core_mod)?;
    spec::register(&discovery_mod)?;
    fingerprints::register(&discovery_mod)?;
    serial_scan::register(&discovery_mod)?;
    mdns_scan::register(&discovery_mod)?;
    serial_channel::register(&discovery_mod)?;
    mdns_channel::register(&discovery_mod)?;
    raygeo_channel::register(&discovery_mod)?;
    daemon::register(&discovery_mod)?;
    engine::register(&discovery_mod)?;
    session::register(&discovery_mod)?;
    Ok(())
}
