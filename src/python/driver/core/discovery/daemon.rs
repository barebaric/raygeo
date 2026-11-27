//! The `raydriver.core.discovery.daemon` Python submodule: queries
//! against a running raydriver daemon.

use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use super::helpers;
use crate::driver::core::discovery::daemon;
use crate::python::driver::core::json::JsonDict;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

/// Lists the cameras a daemon advertises on the device host.
///
/// Each entry is a dict with ``name``, ``uri`` and ``type`` (e.g.
/// ``http_stream``). Video never crosses the control link: clients
/// open *uri* with their own network-camera source.
///
/// *token* authenticates against a protected daemon and is only sent
/// over TLS, except to loopback. TLS is enabled when *tls_ca* names a
/// PEM certificate; *tls_hostname* overrides the verified name.
#[gen_stub_pyfunction(module = "raygeo.driver.core.discovery.daemon")]
#[pyfunction]
#[pyo3(signature = (
    host,
    port,
    token=None,
    tls_ca=None,
    tls_hostname=None,
    timeout=5.0
))]
fn list_cameras(
    py: Python<'_>,
    host: String,
    port: u16,
    token: Option<String>,
    tls_ca: Option<String>,
    tls_hostname: Option<String>,
    timeout: f64,
) -> PyResult<PyAwait<'_, Vec<JsonDict>>> {
    let duration = Duration::from_secs_f64(timeout.max(0.0));
    spawn_future(py, async move {
        let cameras = daemon::query_cameras(
            &host,
            port,
            token.as_deref(),
            tls_ca.as_deref(),
            tls_hostname.as_deref(),
            duration,
        )
        .await
        .map_err(pyo3::exceptions::PyConnectionError::new_err)?;
        Python::attach(|py| {
            let mut out = Vec::with_capacity(cameras.len());
            for camera in cameras {
                let dict = PyDict::new(py);
                dict.set_item("name", camera.name)?;
                dict.set_item("uri", camera.uri)?;
                dict.set_item("type", camera.source_type)?;
                out.push(JsonDict(dict.unbind()));
            }
            Ok(out)
        })
    })
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "daemon")?;
    module.setattr("__doc__", "Queries against a running raydriver daemon.")?;
    module.add("__all__", vec!["list_cameras"])?;
    module.add_function(wrap_pyfunction!(list_cameras, &module)?)?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.daemon",
    )?;
    Ok(())
}
