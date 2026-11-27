//! Python bindings for mDNS announce and browse.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::driver::core::remote::mdns as core;
use crate::python::driver::core::json::{value_to_dict, JsonDict};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};
use crate::python::driver::remote::daemon::DaemonConfig;

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.mdns")]
pub struct ServiceInfo {
    #[pyo3(get)]
    port: u16,
    address: String,
    properties: HashMap<String, String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ServiceInfo {
    fn parsed_addresses(&self) -> Vec<String> {
        vec![self.address.clone()]
    }

    #[getter]
    #[gen_stub(override_return_type(
        type_repr = "dict[builtins.bytes, builtins.bytes]",
        imports = ("builtins",)
    ))]
    fn properties(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.properties {
            dict.set_item(key.as_bytes(), value.as_bytes())?;
        }
        Ok(dict.unbind().into_any())
    }
}

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.mdns")]
pub struct Announcer {
    inner: Mutex<Option<core::MdnsAnnouncer>>,
    #[pyo3(get)]
    info: Py<ServiceInfo>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Announcer {
    fn close<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let inner = {
            let mut guard = self.inner.lock().unwrap();
            guard.take()
        };
        spawn_future(py, async move {
            if let Some(mut announcer) = inner {
                announcer.close();
            }
            Ok(())
        })
    }
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.mdns")]
#[pyfunction]
#[pyo3(signature = (timeout = 3.0))]
pub(crate) fn browse_daemons<'py>(
    py: Python<'py>,
    timeout: f64,
) -> PyResult<PyAwait<'py, Vec<JsonDict>>> {
    spawn_future(py, async move {
        let entries =
            core::browse_daemons(Duration::from_secs_f64(timeout)).await;
        Python::attach(|py| {
            entries
                .iter()
                .map(|entry| value_to_dict(py, entry))
                .collect()
        })
    })
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.mdns")]
#[pyfunction]
pub(crate) fn _announce_address(config: PyRef<'_, DaemonConfig>) -> String {
    let host = config.inner.lock().unwrap().host.clone();
    core::announce_address(&host)
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.mdns")]
#[pyfunction]
pub(crate) fn _service_name() -> String {
    format!("{}.{}", core::service_name(), core::SERVICE_TYPE)
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.mdns")]
#[pyfunction]
pub(crate) fn _service_name_for(host: &str) -> String {
    format!("{}.{}", core::service_name_for(host), core::SERVICE_TYPE)
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.mdns")]
#[pyfunction]
#[pyo3(signature = (config, port, name = None))]
pub(crate) fn announce<'py>(
    py: Python<'py>,
    config: PyRef<'_, DaemonConfig>,
    port: u16,
    name: Option<String>,
) -> PyResult<PyAwait<'py, Option<Announcer>>> {
    let cfg = config.inner.lock().unwrap().clone();
    let camera_count = cfg.cameras.as_array().map(Vec::len).unwrap_or(0);
    spawn_future(py, async move {
        let inner = core::announce(
            &cfg.host,
            port,
            cfg.token.is_some(),
            cfg.tls_cert.is_some(),
            camera_count,
            name.as_deref(),
        );
        let address = core::announce_address(&cfg.host);
        let mut properties = HashMap::new();
        properties.insert("protocol".to_string(), "1".to_string());
        properties.insert(
            "requires_auth".to_string(),
            cfg.token.is_some().to_string(),
        );
        properties
            .insert("tls".to_string(), cfg.tls_cert.is_some().to_string());
        properties.insert("cameras".to_string(), camera_count.to_string());
        Python::attach(|py| {
            let Some(inner) = inner else {
                return Ok(None);
            };
            let info = Py::new(
                py,
                ServiceInfo {
                    port,
                    address,
                    properties,
                },
            )?;
            Ok(Some(Announcer {
                inner: Mutex::new(Some(inner)),
                info,
            }))
        })
    })
}

pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.mdns",
    "SERVICE_TYPE",
    &'static str,
    core::SERVICE_TYPE
);
