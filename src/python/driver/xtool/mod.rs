//! The `raydriver.xtool` Python submodule: sessions for xTool's
//! D-series HTTP API and M2 WS-V2 API.

pub(crate) mod driver;
pub(crate) mod m2;

use std::time::Duration;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::xtool::driver::probe_machine_type;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

pub(crate) const MODULE_DOC: &str = "\
xTool device drivers implemented in Rust.
XtoolNetworkDriver supports D-series G-code jobs over the HTTP API. XtoolM2Driver \
is available at raydriver.xtool.m2 for WS-V2 monitoring and control; \
it does not upload G-code jobs. Its untrusted-certificate option is \
explicit and disabled by default.
";

pyo3_stub_gen::module_doc!("raygeo.driver.xtool", "{}", MODULE_DOC);

/// Queries the machine type an xTool device reports for itself.
///
/// GETs the machine-type endpoint at *host*:*port* and resolves to
/// the reported type when it names an xTool machine, and `None` on
/// any failure.
#[gen_stub_pyfunction(module = "raygeo.driver.xtool")]
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

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let xtool_mod = PyModule::new(py, "xtool")?;

    xtool_mod.setattr("__doc__", MODULE_DOC)?;
    xtool_mod.add(
        "__all__",
        vec!["XtoolNetworkDriver", "query_machine_type", "m2"],
    )?;
    xtool_mod.add_class::<driver::PyXtoolNetworkDriver>()?;
    xtool_mod
        .add_function(wrap_pyfunction!(query_machine_type, &xtool_mod)?)?;
    m2::register(&xtool_mod)?;

    m.add_submodule(&xtool_mod)?;
    // Note: raydriver.xtool is intentionally NOT registered in
    // sys.modules — the Python __init__.py at
    // python/raydriver/xtool/__init__.py serves as the package and
    // delegates to this Rust module.

    Ok(())
}
