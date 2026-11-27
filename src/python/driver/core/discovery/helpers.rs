//! Shared plumbing for the discovery submodules.
//!
//! Channels resolve their collaborators (scanners, timeouts) from
//! their own module namespace at call time, so tests and embedding
//! applications can replace a collaborator by assigning the module
//! attribute — the same seam the Python implementation exposed.

use std::future::Future;
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::PyDict;

/// A sibling discovery submodule, resolved from the parent module
/// handle (registration runs before the package is importable).
pub(crate) fn submodule<'py>(
    discovery_mod: &Bound<'py, PyModule>,
    name: &str,
) -> PyResult<Bound<'py, PyModule>> {
    Ok(discovery_mod.getattr(name)?.cast_into::<PyModule>()?)
}

/// Attaches *module* to *parent* and registers it in ``sys.modules``
/// under its public dotted name, so ``import`` statements resolve to
/// the Rust module without a Python package on disk.
pub(crate) fn attach_submodule(
    parent: &Bound<'_, PyModule>,
    module: &Bound<'_, PyModule>,
    public_name: &str,
) -> PyResult<()> {
    let py = parent.py();
    module.setattr("__name__", public_name)?;
    let short_name = public_name.rsplit('.').next().unwrap_or(public_name);
    parent.setattr(short_name, module)?;
    // Import machinery metadata, so tooling (pytest, importlib)
    // treats the Rust module like a regular importable module.
    let package = public_name.rsplit_once('.').map(|(p, _)| p).unwrap_or("");
    module.setattr("__package__", package)?;
    let machinery = py.import("importlib.machinery")?;
    let spec =
        machinery.call_method1("ModuleSpec", (public_name, py.None()))?;
    module.setattr("__spec__", spec)?;
    py.import("sys")?
        .getattr("modules")?
        .set_item(public_name, module)?;
    Ok(())
}

/// Extracts an iterable of strings (list, set, tuple, ...).
pub(crate) fn string_iter(value: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    let mut out = Vec::new();
    for item in value.try_iter()? {
        out.push(item?.extract::<String>()?);
    }
    Ok(out)
}

/// Extracts an optional iterable of strings.
pub(crate) fn opt_strings(
    value: Option<Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<String>>> {
    match value {
        Some(v) => Ok(Some(string_iter(&v)?)),
        None => Ok(None),
    }
}

/// Extracts an optional iterable of integers.
pub(crate) fn opt_u32(
    value: Option<Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<u32>>> {
    let Some(v) = value else {
        return Ok(None);
    };
    let mut out = Vec::new();
    for item in v.try_iter()? {
        out.push(item?.extract::<u32>()?);
    }
    Ok(Some(out))
}

/// A collaborator attribute of a discovery submodule, resolved at
/// call time so it can be monkeypatched.
pub(crate) fn attr<'py>(
    py: Python<'py>,
    module: &str,
    name: &str,
) -> PyResult<Bound<'py, PyAny>> {
    py.import("raygeo.driver.core.discovery")?
        .getattr(module)?
        .getattr(name)
}

/// A timeout attribute (seconds) of a discovery submodule.
pub(crate) fn timeout_attr(
    py: Python<'_>,
    module: &str,
    name: &str,
) -> PyResult<Duration> {
    let seconds: f64 = attr(py, module, name)?.extract()?;
    Ok(Duration::from_secs_f64(seconds))
}

/// Translates a user-visible string through the embedding
/// application's installed gettext catalog.
pub(crate) fn tr(py: Python<'_>, message: &str) -> PyResult<String> {
    py.import("gettext")?
        .call_method1("gettext", (message,))?
        .extract()
}

/// Bounds an awaitable by *timeout*, mirroring ``asyncio.wait_for``:
/// the awaitable is cancelled when the deadline expires. Must be
/// called from the loop thread; poll the returned future anywhere.
pub(crate) fn wait_for(
    py: Python<'_>,
    awaitable: Bound<'_, PyAny>,
    timeout: Duration,
) -> PyResult<impl Future<Output = PyResult<Py<PyAny>>> + Send> {
    let asyncio = py.import("asyncio")?;
    let wait = asyncio.getattr("wait_for")?;
    crate::python::driver::pyfuture::await_py(
        py,
        wait.call1((awaitable, timeout.as_secs_f64()))?,
    )
}

/// Whether an error is an ``asyncio.TimeoutError`` (or the builtin
/// ``TimeoutError`` it aliases on modern Python).
pub(crate) fn is_timeout(py: Python<'_>, err: &PyErr) -> bool {
    let asyncio_timeout =
        py.import("asyncio").and_then(|m| m.getattr("TimeoutError"));
    match asyncio_timeout {
        Ok(cls) => err.matches(py, &cls).unwrap_or(false),
        Err(_) => false,
    }
}

/// Logs an exception through the Python ``logging`` module, mirroring
/// ``logger.exception``.
pub(crate) fn log_exception(
    py: Python<'_>,
    logger: &str,
    message: &str,
    err: &PyErr,
) {
    let _ = (|| -> PyResult<()> {
        let logging = py.import("logging")?;
        let log = logging.call_method1("getLogger", (logger,))?;
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "exc_info",
            (err.get_type(py), err.value(py), err.traceback(py)),
        )?;
        log.call_method("exception", (message,), Some(&kwargs))?;
        Ok(())
    })();
}

pub(crate) fn log_warning(py: Python<'_>, logger: &str, message: &str) {
    let _ = (|| -> PyResult<()> {
        let logging = py.import("logging")?;
        let log = logging.call_method1("getLogger", (logger,))?;
        log.call_method1("warning", (message,))?;
        Ok(())
    })();
}

/// Logs an exception at debug level, mirroring
/// ``logger.debug(..., exc_info=True)``.
pub(crate) fn log_debug(
    py: Python<'_>,
    logger: &str,
    message: &str,
    err: &PyErr,
) {
    let _ = (|| -> PyResult<()> {
        let logging = py.import("logging")?;
        let log = logging.call_method1("getLogger", (logger,))?;
        let kwargs = PyDict::new(py);
        kwargs.set_item(
            "exc_info",
            (err.get_type(py), err.value(py), err.traceback(py)),
        )?;
        log.call_method("debug", (message,), Some(&kwargs))?;
        Ok(())
    })();
}
