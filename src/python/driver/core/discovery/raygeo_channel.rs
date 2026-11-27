//! The `raygeo.driver.core.discovery.raygeo_channel` submodule:
//! discovery channel for raydriver daemons (`_raydriver._tcp`).
//!
//! A daemon is an endpoint, not a firmware: the channel browses the
//! service type and lists the daemon's devices via the Rust control
//! protocol probe. Every device becomes a `DiscoveredDevice` using
//! the single generic remote driver; the protocol is carried as info
//! (`_raydriver_protocol`), never used to pick a driver class.
//! Token-protected daemons cannot be enumerated without credentials;
//! they surface as a single daemon row that seeds the endpoint vars.

use std::collections::HashMap;
use std::time::Duration;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::core::discovery::daemon::{self, DaemonEntry, DaemonNote};
use crate::driver::core::discovery::types::{
    DeviceIdentity as CoreIdentity, DiscoveredDevice as CoreDevice, ParamValue,
};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

use super::helpers;
use super::mdns_scan::{MDNSService, ScanMdnsFn};
use super::types::DiscoveredDevice;

const LOGGER: &str = "raygeo.driver.core.discovery.raygeo_channel";

const SERVICE_TYPE: &str = "_raydriver._tcp.local.";
const SCAN_TIMEOUT: f64 = 10.0;
const QUERY_TIMEOUT: Duration = Duration::from_secs(3);

/// The one driver used for every raydriver-hosted device.
const REMOTE_DRIVER: &str = "RaydriverRemoteDriver";

/// Browse for raydriver daemons and list their sessions.
///
/// Like every channel, this never raises: failures are logged and
/// bounded by timeouts.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.raygeo_channel")]
#[pyfunction]
fn find_raygeo_devices(
    py: Python<'_>,
) -> PyResult<PyAwait<'_, Vec<DiscoveredDevice>>> {
    let scan = helpers::attr(py, "raygeo_channel", "scan_mdns_services")?;
    let scan_fut = helpers::wait_for(
        py,
        scan.call1((vec![SERVICE_TYPE.to_string()],))?,
        Duration::from_secs_f64(SCAN_TIMEOUT),
    )?;
    spawn_future(py, async move {
        let services = match scan_fut.await {
            Ok(value) => value,
            Err(e) => {
                return Python::attach(|py| {
                    if helpers::is_timeout(py, &e) {
                        helpers::log_warning(
                            py,
                            LOGGER,
                            "raydriver daemon discovery timed out",
                        );
                    } else {
                        helpers::log_exception(
                            py,
                            LOGGER,
                            "raydriver daemon discovery failed",
                            &e,
                        );
                    }
                    let empty: Vec<DiscoveredDevice> = Vec::new();
                    Ok(empty)
                });
            }
        };
        let services: Vec<MDNSService> =
            Python::attach(|py| services.bind(py).extract())?;
        let mut devices = Vec::new();
        for service in services {
            let report = daemon::query_daemon(
                &service.host,
                service.port,
                &service.name,
                QUERY_TIMEOUT,
            )
            .await;
            Python::attach(|py| -> PyResult<()> {
                for entry in &report.entries {
                    devices.push(build_entry(py, &report, entry)?);
                }
                Ok(())
            })?;
        }
        Ok(devices)
    })
}

fn build_entry(
    py: Python<'_>,
    report: &daemon::DaemonReport,
    entry: &DaemonEntry,
) -> PyResult<DiscoveredDevice> {
    let banner = if report.name.is_empty() {
        None
    } else {
        Some(report.name.clone())
    };
    let identity = CoreIdentity {
        banner,
        ..Default::default()
    };
    let mut params = HashMap::new();
    params.insert(
        "remote_host".to_string(),
        ParamValue::Str(report.host.clone()),
    );
    params.insert(
        "remote_port".to_string(),
        ParamValue::Int(report.port as i64),
    );
    match entry {
        DaemonEntry::Device {
            device,
            protocol,
            label,
        } => {
            params
                .insert("device".to_string(), ParamValue::Str(device.clone()));
            params.insert(
                "_raydriver_protocol".to_string(),
                ParamValue::Str(protocol.clone()),
            );
            let detail = helpers::tr(py, "{host}:{port} via raydriver daemon")?
                .replace("{host}", &report.host)
                .replace("{port}", &report.port.to_string());
            Ok(DiscoveredDevice {
                inner: CoreDevice {
                    driver_name: REMOTE_DRIVER.to_string(),
                    params,
                    label: label.clone().unwrap_or_else(|| device.clone()),
                    detail,
                    identity,
                },
                probe_profile: None,
            })
        }
        DaemonEntry::Row { note } => {
            let note = match note {
                DaemonNote::Unreachable => helpers::tr(py, "unreachable")?,
                DaemonNote::TokenRequired => helpers::tr(py, "token required")?,
                DaemonNote::NoDevices => helpers::tr(py, "no devices")?,
            };
            Ok(DiscoveredDevice {
                inner: CoreDevice {
                    driver_name: REMOTE_DRIVER.to_string(),
                    params,
                    label: helpers::tr(py, "raydriver daemon")?,
                    detail: format!(
                        "{}:{} ({})",
                        report.host, report.port, note
                    ),
                    identity,
                },
                probe_profile: None,
            })
        }
    }
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "raygeo_channel")?;
    module.setattr(
        "__doc__",
        "Discovery channel for raydriver daemons: browse, welcome, \
         device list.",
    )?;
    module.setattr("SERVICE_TYPE", SERVICE_TYPE)?;
    module.setattr("REMOTE_DRIVER", REMOTE_DRIVER)?;
    // The scanner backend, replaceable as a module attribute.
    let scan_mod = helpers::submodule(discovery_mod, "mdns_scan")?;
    module.setattr(
        "scan_mdns_services",
        scan_mod.getattr("scan_mdns_services")?,
    )?;
    module.add_function(wrap_pyfunction!(find_raygeo_devices, &module)?)?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.raygeo_channel",
    )?;
    Ok(())
}
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.raygeo_channel",
    "SERVICE_TYPE",
    String
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.raygeo_channel",
    "REMOTE_DRIVER",
    String
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.raygeo_channel",
    "scan_mdns_services",
    ScanMdnsFn
);
