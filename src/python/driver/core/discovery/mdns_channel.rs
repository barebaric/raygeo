//! The `raydriver.core.discovery.mdns_channel` submodule: network
//! (mDNS) discovery channel.
//!
//! Browses the network for every service type declared by a driver's
//! `MdnsRecognizer` plus the generic types its `fingerprint_services`
//! name, then maps each resolved service to a `DiscoveredDevice`.
//! Generic service types (e.g. `_http._tcp`, `_telnet._tcp`) that no
//! driver declared are offered to drivers that fingerprint them: each
//! candidate is fingerprinted to confirm its firmware before it is
//! claimed, and a declaration only probes the types it named.
//! Declared-service results always win over fingerprint claims on the
//! same host.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::core::discovery::mdns::normalize_service_type;
use crate::driver::core::discovery::tokens;
use crate::driver::core::discovery::types::{
    DeviceIdentity as CoreIdentity, DiscoveredDevice as CoreDevice, ParamValue,
};
use crate::python::driver::pyfuture::{await_py, spawn_future, PyAwait};

use super::helpers;
use super::mdns_scan::{MDNSService, ScanMdnsFn};
use super::spec::DiscoverySpec;
use super::types::DiscoveredDevice;

const LOGGER: &str = "raygeo.driver.core.discovery.mdns_channel";

/// Upper bound for the network part of a discovery scan; the browse
/// window itself is much shorter (see mdns_scan).
const NETWORK_SCAN_TIMEOUT: f64 = 10.0;

/// Fingerprint pass bounds: at most this many concurrent probes, and
/// at most this long per candidate (the probe itself is bounded too).
const FINGERPRINT_CONCURRENCY: usize = 8;
const FINGERPRINT_TIMEOUT: f64 = 2.0;

/// One driver's network declaration, owned for the async pass.
#[derive(Clone)]
struct Declaration {
    driver_name: String,
    label: String,
    services: Vec<String>,
    txt_map: HashMap<String, String>,
    fingerprint: Option<Py<PyAny>>,
    fingerprint_services: Vec<String>,
}

/// Pairs every driver whose ``DISCOVERY`` spec declares a valid mDNS
/// recognizer with it. Declared service types and fingerprint service
/// types are normalized to their canonical ``_name._transport`` form.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.mdns_channel")]
#[pyfunction]
#[gen_stub(override_return_type(
    type_repr = "builtins.list[tuple[builtins.type, \
                 spec.MdnsRecognizer]]",
    imports = ("builtins", "raygeo.driver.core.discovery.spec"),
))]
fn collect_mdns_recognizers<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Sequence[builtins.type]", imports = ("typing", "builtins")))]
    driver_classes: Vec<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyList>> {
    let out = PyList::empty(py);
    let spec_mod = py.import("raygeo.driver.core.discovery.spec")?;
    let recognizer_cls = spec_mod.getattr("MdnsRecognizer")?;
    for cls in driver_classes {
        let Ok(spec) = cls.getattr("DISCOVERY") else {
            continue;
        };
        if !spec.is_instance_of::<DiscoverySpec>() {
            continue;
        }
        let spec: PyRef<'_, DiscoverySpec> = spec.extract()?;
        let Some(recognizer) = spec.mdns(py) else {
            continue;
        };
        let recognizer = recognizer.bind(py);
        let services: Vec<String> =
            recognizer.getattr("services")?.extract()?;
        let normalized: Vec<String> =
            services.iter().map(|s| normalize_service_type(s)).collect();
        let fingerprint_services: Vec<String> = recognizer
            .getattr("fingerprint_services")?
            .extract::<Vec<String>>()?
            .iter()
            .map(|s| normalize_service_type(s))
            .collect();
        let kwargs = PyDict::new(py);
        kwargs.set_item("services", PyTuple::new(py, &normalized)?)?;
        kwargs.set_item("txt_map", recognizer.getattr("txt_map")?)?;
        kwargs.set_item("fingerprint", recognizer.getattr("fingerprint")?)?;
        kwargs.set_item(
            "fingerprint_services",
            PyTuple::new(py, &fingerprint_services)?,
        )?;
        let normalized_recognizer =
            recognizer_cls.call(PyTuple::empty(py), Some(&kwargs))?;
        out.append(PyTuple::new(py, [cls, normalized_recognizer])?)?;
    }
    Ok(out)
}

fn read_declarations(
    collected: &Bound<'_, PyList>,
) -> PyResult<Vec<Declaration>> {
    let mut declarations = Vec::new();
    for item in collected.iter() {
        let cls = item.get_item(0)?;
        let recognizer = item.get_item(1)?;
        let driver_name: String = cls.getattr("__name__")?.extract()?;
        declarations.push(Declaration {
            driver_name,
            label: cls.getattr("label")?.extract()?,
            services: recognizer.getattr("services")?.extract()?,
            txt_map: recognizer.getattr("txt_map")?.extract()?,
            fingerprint: recognizer
                .getattr("fingerprint")?
                .extract::<Option<Py<PyAny>>>()?,
            fingerprint_services: recognizer
                .getattr("fingerprint_services")?
                .extract()?,
        });
    }
    Ok(declarations)
}

/// Every service type named by a fingerprinting declaration. Only
/// declarations that actually declare a fingerprint probe can offer
/// generic candidates, so their service types alone extend the browse
/// set.
fn fingerprint_service_types(declarations: &[Declaration]) -> BTreeSet<String> {
    declarations
        .iter()
        .filter(|d| d.fingerprint.is_some())
        .flat_map(|d| d.fingerprint_services.iter().cloned())
        .collect()
}

/// Browses the network for every declared service type plus the
/// generic types needed for fingerprinting, and returns one device
/// per confirmed match.
///
/// Like every channel, this never raises: failures are logged and
/// bounded by a timeout.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.mdns_channel")]
#[pyfunction]
fn find_network_devices<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Sequence[builtins.type]", imports = ("typing", "builtins")))]
    driver_classes: Vec<Bound<'_, PyAny>>,
) -> PyResult<PyAwait<'py, Vec<DiscoveredDevice>>> {
    let collected = collect_mdns_recognizers(py, driver_classes)?;
    if collected.is_empty() {
        return spawn_future(py, async move {
            let empty: Vec<DiscoveredDevice> = Vec::new();
            Ok(empty)
        });
    }
    let declarations = read_declarations(&collected)?;

    let mut service_types: BTreeSet<String> = declarations
        .iter()
        .flat_map(|d| d.services.iter().cloned())
        .collect();
    service_types.extend(fingerprint_service_types(&declarations));

    let timeout =
        helpers::timeout_attr(py, "mdns_channel", "NETWORK_SCAN_TIMEOUT")?;
    let scan = helpers::attr(py, "mdns_channel", "scan_mdns_services")?;
    let types_set = pyo3::types::PySet::new(py, service_types.iter())?;
    let scan_fut = helpers::wait_for(py, scan.call1((types_set,))?, timeout)?;
    let loop_py = py
        .import("asyncio")?
        .call_method0("get_running_loop")?
        .unbind();

    spawn_future(py, async move {
        let services = match scan_fut.await {
            Ok(value) => value,
            Err(e) => {
                return Python::attach(|py| {
                    if helpers::is_timeout(py, &e) {
                        helpers::log_warning(
                            py,
                            LOGGER,
                            "Network device discovery timed out",
                        );
                    } else {
                        helpers::log_exception(
                            py,
                            LOGGER,
                            "Network device discovery failed",
                            &e,
                        );
                    }
                    let empty: Vec<DiscoveredDevice> = Vec::new();
                    Ok(empty)
                });
            }
        };

        let (mut devices, claimed_hosts, candidates) =
            Python::attach(|py| -> PyResult<_> {
                let services: Vec<MDNSService> = services.bind(py).extract()?;
                split_services(py, &services, &declarations)
            })?;

        let fingerprinted =
            run_fingerprints(&declarations, candidates, claimed_hosts, loop_py)
                .await;
        Python::attach(|py| {
            for (declaration, service) in fingerprinted {
                devices.push(build_device(py, &declaration, &service)?);
            }
            Ok(devices)
        })
    })
}

/// Sorts resolved services into devices claimed by a declared
/// service type and unclaimed candidates of a fingerprinted service
/// type.
fn split_services(
    py: Python<'_>,
    services: &[MDNSService],
    declarations: &[Declaration],
) -> PyResult<(Vec<DiscoveredDevice>, HashSet<String>, Vec<MDNSService>)> {
    let fingerprint_types = fingerprint_service_types(declarations);
    let mut devices = Vec::new();
    let mut claimed_hosts = HashSet::new();
    let mut candidates = Vec::new();
    for service in services {
        let mut claimed = false;
        for declaration in declarations {
            if declaration.services.contains(&service.service_type) {
                devices.push(build_device(py, declaration, service)?);
                claimed_hosts.insert(service.host.clone());
                claimed = true;
                break;
            }
        }
        if !claimed && fingerprint_types.contains(&service.service_type) {
            candidates.push(service.clone());
        }
    }
    Ok((devices, claimed_hosts, candidates))
}

/// Fingerprints unclaimed generic-service candidates, bounded in
/// concurrency. A declaration probes only the candidates whose service
/// type it named in ``fingerprint_services``, so no cross-type probing
/// happens; the first match in declaration order claims a host, so a
/// host that several drivers recognize yields exactly one device.
async fn run_fingerprints(
    declarations: &[Declaration],
    candidates: Vec<MDNSService>,
    claimed_hosts: HashSet<String>,
    loop_py: Py<PyAny>,
) -> Vec<(Declaration, MDNSService)> {
    let probes = Python::attach(|py| {
        let mut probes = Vec::new();
        for declaration in declarations {
            let Some(fingerprint) = &declaration.fingerprint else {
                continue;
            };
            for service in &candidates {
                if claimed_hosts.contains(&service.host) {
                    continue;
                }
                if !declaration
                    .fingerprint_services
                    .contains(&service.service_type)
                {
                    continue;
                }
                probes.push((
                    declaration.clone(),
                    service.clone(),
                    fingerprint.clone_ref(py),
                    loop_py.clone_ref(py),
                ));
            }
        }
        probes
    });
    if probes.is_empty() {
        return Vec::new();
    }

    let semaphore =
        Arc::new(tokio::sync::Semaphore::new(FINGERPRINT_CONCURRENCY));
    let mut handles = Vec::new();
    for (declaration, service, fingerprint, loop_py) in probes {
        let semaphore = semaphore.clone();
        let host = service.host.clone();
        let port = service.port;
        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.ok()?;
            // The probe coroutine must be created on the event loop
            // thread: asyncio primitives need its running loop.
            let (tx, rx) = tokio::sync::oneshot::channel();
            let scheduled = Python::attach(|py| -> PyResult<()> {
                let loop_ = loop_py.bind(py);
                let tx = std::sync::Mutex::new(Some(tx));
                let create = pyo3::types::PyCFunction::new_closure(
                    py,
                    None,
                    None,
                    move |args: &Bound<'_, pyo3::types::PyTuple>,
                          _kwargs|
                          -> PyResult<Py<PyAny>> {
                        let py = args.py();
                        let probe = (|| -> PyResult<_> {
                            let coro =
                                fingerprint.call1(py, (host.clone(), port))?;
                            let bounded = py.import("asyncio")?.call_method1(
                                "wait_for",
                                (coro, FINGERPRINT_TIMEOUT),
                            )?;
                            await_py(py, bounded)
                        })();
                        let sender = tx.lock().unwrap().take();
                        if let Some(tx) = sender {
                            match probe {
                                Ok(fut) => {
                                    crate::python::driver::runtime::handle()
                                        .spawn(async move {
                                            let _ = tx.send(fut.await);
                                        });
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(e));
                                }
                            }
                        }
                        Ok(py.None())
                    },
                )?;
                loop_.call_method1("call_soon_threadsafe", (create,))?;
                Ok(())
            });
            let outcome = match scheduled {
                Ok(()) => rx.await.unwrap_or_else(|_| {
                    Err(pyo3::exceptions::PyRuntimeError::new_err(
                        "fingerprint probe dropped before completion",
                    ))
                }),
                Err(e) => Err(e),
            };
            Some((declaration, service, outcome))
        }));
    }

    let mut claimed = HashSet::new();
    let mut out = Vec::new();
    for handle in handles {
        let Ok(Some((declaration, service, outcome))) = handle.await else {
            continue;
        };
        let matched = Python::attach(|py| match outcome {
            // The claim is the probe's truthiness: the built-in
            // fingerprints resolve to a plain bool.
            Ok(value) => value.is_truthy(py).unwrap_or(false),
            Err(e) => {
                helpers::log_debug(
                    py,
                    LOGGER,
                    &format!(
                        "Fingerprint of {}:{} failed",
                        service.host, service.port
                    ),
                    &e,
                );
                false
            }
        });
        if !matched || claimed.contains(&service.host) {
            continue;
        }
        claimed.insert(service.host.clone());
        out.push((declaration, service));
    }
    out
}

/// A firmware identifier derived from the driver class name
/// (e.g. ``OctoPrintDriver`` → ``octoprint``).
fn driver_firmware_id(driver_name: &str) -> String {
    driver_name
        .strip_suffix("Driver")
        .unwrap_or(driver_name)
        .to_lowercase()
}

/// Assembles a `DiscoveredDevice` from a resolved mDNS service.
///
/// TXT record fields the recognizer declared in ``txt_map`` are
/// forwarded into ``params``. Identity-strengthening TXT fields
/// (``vendor``, ``model``) feed the matching token set; ``version``
/// becomes the identity banner when the service announced no
/// instance name of its own.
fn build_device(
    py: Python<'_>,
    declaration: &Declaration,
    service: &MDNSService,
) -> PyResult<DiscoveredDevice> {
    let mut detail = helpers::tr(py, "{host}:{port}")?
        .replace("{host}", &service.host)
        .replace("{port}", &service.port.to_string());
    if !service.server.is_empty() {
        detail = format!("{detail} ({})", service.server);
    }

    let mut params = HashMap::new();
    params.insert("host".to_string(), ParamValue::Str(service.host.clone()));
    params.insert("port".to_string(), ParamValue::Int(service.port as i64));
    for (txt_key, arg_key) in &declaration.txt_map {
        if let Some(value) = service.txt.get(txt_key) {
            if !value.is_empty() {
                params.insert(arg_key.clone(), ParamValue::Str(value.clone()));
            }
        }
    }

    let version = service.txt.get("version").cloned().unwrap_or_default();
    let banner = if !service.name.is_empty() {
        Some(service.name.clone())
    } else if !version.is_empty() {
        Some(version)
    } else {
        None
    };

    let tokens = tokens::normalize_tokens(&[
        Some(service.name.as_str()),
        Some(service.server.as_str()),
        service.txt.get("vendor").map(String::as_str),
        service.txt.get("model").map(String::as_str),
    ]);
    Ok(DiscoveredDevice {
        inner: CoreDevice {
            driver_name: declaration.driver_name.clone(),
            params,
            label: declaration.label.clone(),
            detail,
            identity: CoreIdentity {
                firmware: Some(driver_firmware_id(&declaration.driver_name)),
                banner,
                tokens,
                ..Default::default()
            },
        },
        probe_profile: None,
    })
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "mdns_channel")?;
    module.setattr(
        "__doc__",
        "Network (mDNS) discovery channel with fingerprint \
         confirmation for generic service types.",
    )?;
    module.setattr("NETWORK_SCAN_TIMEOUT", NETWORK_SCAN_TIMEOUT)?;
    // The scanner backend, replaceable as a module attribute.
    let scan_mod = helpers::submodule(discovery_mod, "mdns_scan")?;
    module.setattr(
        "scan_mdns_services",
        scan_mod.getattr("scan_mdns_services")?,
    )?;
    module
        .add_function(wrap_pyfunction!(collect_mdns_recognizers, &module)?)?;
    module.add_function(wrap_pyfunction!(find_network_devices, &module)?)?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.mdns_channel",
    )?;
    Ok(())
}
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.mdns_channel",
    "NETWORK_SCAN_TIMEOUT",
    f64
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.core.discovery.mdns_channel",
    "scan_mdns_services",
    ScanMdnsFn
);
