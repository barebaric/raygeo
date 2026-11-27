//! Python bindings for read-only daemon inspection.

use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use serde_json::Value;

use crate::driver::core::remote::inspect as core;
use crate::python::driver::core::json::{value_to_dict, JsonDict};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};
use crate::python::driver::remote::bridge::py_to_json;

/// Converts a JSON object payload, mapping null to ``None`` (a
/// daemon that sends a welcome without a payload).
fn optional_dict(py: Python<'_>, value: &Value) -> PyResult<Option<JsonDict>> {
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(value_to_dict(py, value)?))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.inspect")]
#[pyfunction]
pub(crate) fn daemon_rpc<'py>(
    py: Python<'py>,
    host: String,
    port: u16,
    token: Option<String>,
    tls_ca: Option<String>,
    #[gen_stub(override_type(
        type_repr = "typing.Sequence[tuple[builtins.str, dict[builtins.str, builtins.object]]]",
        imports = ("typing", "builtins")
    ))]
    requests: Py<PyAny>,
) -> PyResult<PyAwait<'py, (Option<JsonDict>, Vec<JsonDict>)>> {
    let requests: Vec<(String, Value)> = requests
        .bind(py)
        .try_iter()?
        .map(|entry| {
            let entry = entry?;
            let pair = entry.cast::<PyTuple>()?;
            let type_: String = pair.get_item(0)?.extract()?;
            let payload = py_to_json(&pair.get_item(1)?)?;
            Ok((type_, payload))
        })
        .collect::<PyResult<_>>()?;
    spawn_future(py, async move {
        let borrowed: Vec<(&str, Value)> = requests
            .iter()
            .map(|(type_, payload)| (type_.as_str(), payload.clone()))
            .collect();
        let (welcome, replies) = core::daemon_rpc(
            &host,
            port,
            token.as_deref(),
            tls_ca.as_deref(),
            None,
            &borrowed,
        )
        .await
        .map_err(pyo3::exceptions::PyConnectionError::new_err)?;
        Python::attach(|py| {
            let welcome = optional_dict(py, &welcome)?;
            let replies = replies
                .iter()
                .map(|reply| value_to_dict(py, reply))
                .collect::<PyResult<Vec<JsonDict>>>()?;
            Ok((welcome, replies))
        })
    })
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.inspect")]
#[pyfunction]
pub(crate) fn welcome_probe<'py>(
    py: Python<'py>,
    host: String,
    port: u16,
) -> PyResult<PyAwait<'py, Option<JsonDict>>> {
    spawn_future(py, async move {
        let welcome = core::welcome_probe(&host, port)
            .await
            .map_err(pyo3::exceptions::PyConnectionError::new_err)?;
        Python::attach(|py| optional_dict(py, &welcome))
    })
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.inspect")]
#[pyfunction]
#[pyo3(signature = (host, port, token = None, tls_ca = None))]
pub(crate) fn snapshot<'py>(
    py: Python<'py>,
    host: String,
    port: u16,
    token: Option<String>,
    tls_ca: Option<String>,
) -> PyResult<PyAwait<'py, JsonDict>> {
    spawn_future(py, async move {
        let snapshot =
            core::snapshot(&host, port, token.as_deref(), tls_ca.as_deref())
                .await
                .map_err(pyo3::exceptions::PyConnectionError::new_err)?;
        Python::attach(|py| value_to_dict(py, &snapshot))
    })
}
