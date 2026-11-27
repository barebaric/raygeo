//! The `raygeo.driver.xtool_d1` Python submodule: the xTool D-series
//! HTTP API session and the machine-type probe.

pub(crate) mod driver;

use std::time::Duration;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::xtool_d1::driver::probe_machine_type;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

pub(crate) const MODULE_DOC: &str = "\
xTool D-series driver implemented in Rust. XtoolNetworkDriver \
supports D-series G-code jobs over the HTTP API and the companion \
WebSocket notification channel.";

pyo3_stub_gen::module_doc!("raygeo.driver.xtool_d1", "{}", MODULE_DOC);

/// Queries the machine type an xTool device reports for itself.
///
/// GETs the machine-type endpoint at *host*:*port* and resolves to
/// the reported type when it names an xTool machine, and `None` on
/// any failure.
#[gen_stub_pyfunction(module = "raygeo.driver.xtool_d1")]
#[pyfunction(signature = (host, port, timeout=2.0))]
fn query_machine_type<'py>(
    py: Python<'py>,
    host: String,
    port: u16,
    timeout: f64,
) -> PyResult<PyAwait<'py, Option<String>>> {
    spawn_future(py, async move {
        let machine_type = probe_machine_type(
            &host,
            port,
            Duration::from_secs_f64(timeout.max(0.01)),
        )
        .await
        .ok()
        .filter(|machine_type| {
            machine_type.to_ascii_lowercase().contains("xtool")
        });
        Ok(machine_type)
    })
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let xtool_mod = PyModule::new(py, "xtool_d1")?;

    xtool_mod.setattr("__doc__", MODULE_DOC)?;
    xtool_mod
        .add("__all__", vec!["XtoolNetworkDriver", "query_machine_type"])?;
    xtool_mod.add_class::<driver::PyXtoolNetworkDriver>()?;
    xtool_mod
        .add_function(wrap_pyfunction!(query_machine_type, &xtool_mod)?)?;

    m.add_submodule(&xtool_mod)?;
    Ok(())
}
