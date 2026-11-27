//! The `raygeo.driver.core.discovery.mdns_scan` submodule: mDNS network
//! service scanning.
//!
//! The browse runs in the Rust core; this submodule normalizes the
//! requested service types and shapes the results. Deciding which
//! driver could talk to a discovered service is the network
//! channel's concern.

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};
use pyo3_stub_gen::{PyStubType, TypeInfo};

use super::helpers;
use crate::driver::core::discovery::mdns;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

/// A replaceable mDNS browse callable: the network channels resolve
/// their scanner from their module namespace at call time (see
/// helpers), so the attribute is typed as an async callable
/// resolving to the resolved services.
pub(crate) struct ScanMdnsFn;

impl PyStubType for ScanMdnsFn {
    fn type_output() -> TypeInfo {
        let service = MDNSService::type_output();
        let mut import = service.import.clone();
        import.insert("collections.abc".into());
        import.insert("builtins".into());
        TypeInfo {
            name: format!(
                "collections.abc.Callable[..., \
                 collections.abc.Awaitable[builtins.list[{}]]]",
                service.name
            ),
            source_module: None,
            import,
            type_refs: service.type_refs,
        }
    }
}

/// One service resolved during an mDNS browse.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.mdns_scan")]
#[pyclass(
    module = "raygeo.driver.core.discovery.mdns_scan",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct MDNSService {
    pub(crate) service_type: String,
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) server: String,
    pub(crate) txt: HashMap<String, String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MDNSService {
    #[new]
    #[pyo3(signature = (service_type, name, host, port, server=None, txt=None))]
    fn new(
        service_type: String,
        name: String,
        host: String,
        port: u16,
        server: Option<String>,
        txt: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            service_type,
            name,
            host,
            port,
            server: server.unwrap_or_default(),
            txt: txt.unwrap_or_default(),
        }
    }

    /// Normalized service type, e.g. ``_octoprint._tcp``.
    #[getter]
    fn service_type(&self) -> String {
        self.service_type.clone()
    }

    /// Instance name, e.g. ``OctoPrint on octopi``.
    #[getter]
    fn name(&self) -> String {
        self.name.clone()
    }

    /// Address to connect to; IPv4 preferred over IPv6.
    #[getter]
    fn host(&self) -> String {
        self.host.clone()
    }

    #[getter]
    fn port(&self) -> u16 {
        self.port
    }

    /// The server's own hostname, e.g. ``octopi.local``.
    #[getter]
    fn server(&self) -> String {
        self.server.clone()
    }

    /// TXT record contents, values decoded to strings.
    #[getter]
    fn txt<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.txt {
            dict.set_item(key, value)?;
        }
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "MDNSService(service_type={:?}, name={:?}, host={:?}, port={})",
            self.service_type, self.name, self.host, self.port
        )
    }
}

/// Reduces a service type to its canonical ``_name._transport``
/// form: ``_octoprint._tcp.local.`` becomes ``_octoprint._tcp``.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.mdns_scan")]
#[pyfunction]
fn normalize_service_type(service_type: &str) -> String {
    mdns::normalize_service_type(service_type)
}

/// Browses *service_types* over mDNS for a short window and returns
/// every service that could be resolved to an address. Failures
/// (no network, no responder) are logged and yield an empty list
/// rather than raising.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.mdns_scan")]
#[pyfunction]
#[pyo3(signature = (service_types, browse_window=2.0))]
fn scan_mdns_services<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(type_repr = "typing.Iterable[builtins.str]", imports = ("typing", "builtins")))]
    service_types: &Bound<'_, PyAny>,
    browse_window: f64,
) -> PyResult<PyAwait<'py, Vec<MDNSService>>> {
    let service_types = helpers::string_iter(service_types)?;
    let normalized: Vec<String> = service_types
        .iter()
        .map(|t| mdns::normalize_service_type(t))
        .filter(|t| !t.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if normalized.is_empty() {
        return spawn_future(py, async move {
            let empty: Vec<MDNSService> = Vec::new();
            Ok(empty)
        });
    }
    let window = Duration::from_secs_f64(browse_window);
    spawn_future(py, async move {
        let services = mdns::scan_mdns_services(normalized, window).await;
        let out: Vec<MDNSService> = services
            .into_iter()
            .map(|s| MDNSService {
                service_type: s.service_type,
                name: s.name,
                host: s.host,
                port: s.port,
                server: s.server,
                txt: s.txt,
            })
            .collect();
        Ok(out)
    })
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "mdns_scan")?;
    module.setattr(
        "__doc__",
        "mDNS network service scanning: bounded browse plus result \
         shaping.",
    )?;
    module.add_class::<MDNSService>()?;
    module.add_function(wrap_pyfunction!(normalize_service_type, &module)?)?;
    module.add_function(wrap_pyfunction!(scan_mdns_services, &module)?)?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.mdns_scan",
    )?;
    Ok(())
}
