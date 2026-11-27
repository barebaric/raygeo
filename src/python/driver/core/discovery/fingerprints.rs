//! The `raygeo.driver.core.discovery.fingerprints` submodule:
//! reusable fingerprint probes for generic mDNS service types.
//!
//! A fingerprint is an async callable handed to an `MdnsRecognizer`;
//! the network channel calls it with a candidate's address and port.
//! `TcpFingerprint` provides the common case: confirm a raw TCP
//! service by sending an optional command and matching a response
//! line prefix. `HttpFingerprint` confirms an HTTP endpoint by
//! matching its response body. Protocol-specific command strings
//! stay with the driver that declares them.

use std::time::Duration;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use regex::{Regex, RegexBuilder};

use crate::driver::core::discovery::probe::{
    probe_http, probe_tcp, HttpProbe, TcpProbe,
};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

use super::helpers;

/// Confirms a generic TCP service by matching a response line.
///
/// The probe waits *banner_wait* seconds for an unsolicited banner,
/// then (when *probe_command* is set) sends it and waits for the
/// remaining *timeout* seconds. It resolves to whether a line
/// starts with *match_prefix*: ``True`` claims the candidate,
/// ``False`` does not.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.fingerprints")]
#[pyclass(
    module = "raygeo.driver.core.discovery.fingerprints",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct TcpFingerprint {
    probe_command: Option<String>,
    match_prefix: String,
    banner_wait: f64,
    timeout: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl TcpFingerprint {
    #[new]
    #[pyo3(signature = (
        probe_command=None,
        match_prefix=None,
        banner_wait=1.0,
        timeout=1.9
    ))]
    fn new(
        probe_command: Option<String>,
        match_prefix: Option<String>,
        banner_wait: f64,
        timeout: f64,
    ) -> Self {
        Self {
            probe_command,
            match_prefix: match_prefix.unwrap_or_default(),
            banner_wait,
            timeout,
        }
    }

    #[getter]
    pub(crate) fn probe_command(&self) -> Option<String> {
        self.probe_command.clone()
    }

    #[getter]
    pub(crate) fn match_prefix(&self) -> String {
        self.match_prefix.clone()
    }

    #[getter]
    pub(crate) fn banner_wait(&self) -> f64 {
        self.banner_wait
    }

    #[getter]
    pub(crate) fn timeout(&self) -> f64 {
        self.timeout
    }

    /// The claim is boolean: ``True`` claims the candidate host for
    /// the declaring driver, ``False`` leaves it unclaimed.
    fn __call__<'py>(
        &self,
        py: Python<'py>,
        host: String,
        port: u16,
    ) -> PyResult<PyAwait<'py, bool>> {
        let config = self.clone();
        spawn_future(py, async move {
            let timeout = Duration::from_secs_f64(config.timeout.max(0.01));
            let banner_wait =
                Duration::from_secs_f64(config.banner_wait.max(0.0))
                    .min(timeout);
            let response_wait = timeout
                .saturating_sub(banner_wait)
                .max(Duration::from_millis(100));
            let matched = probe_tcp(
                &host,
                port,
                &TcpProbe {
                    connect_timeout: timeout,
                    banner_wait,
                    response_wait,
                    command: config.probe_command.as_deref(),
                    match_prefix: &config.match_prefix,
                },
            )
            .await
            .is_some();
            Ok(matched)
        })
    }
}

/// Confirms a generic HTTP service by matching the response body.
///
/// The probe GETs *path* and reports a match when the endpoint
/// answers `200` with a body matching *match_regex* (matched
/// case-insensitively), bounded by *timeout* seconds and *max_bytes*
/// of body. It resolves to ``True`` on a match, and ``False`` on
/// any failure.
#[gen_stub_pyclass(module = "raygeo.driver.core.discovery.fingerprints")]
#[pyclass(
    module = "raygeo.driver.core.discovery.fingerprints",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct HttpFingerprint {
    path: String,
    match_regex: String,
    regex: Regex,
    timeout: f64,
    max_bytes: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl HttpFingerprint {
    #[new]
    #[pyo3(signature = (path, match_regex, timeout=1.5, max_bytes=4096))]
    fn new(
        path: String,
        match_regex: String,
        timeout: f64,
        max_bytes: usize,
    ) -> PyResult<Self> {
        let regex = RegexBuilder::new(&match_regex)
            .case_insensitive(true)
            .build()
            .map_err(|err| {
                PyValueError::new_err(format!("invalid match_regex: {err}"))
            })?;
        Ok(Self {
            path,
            match_regex,
            regex,
            timeout,
            max_bytes,
        })
    }

    #[getter]
    pub(crate) fn path(&self) -> String {
        self.path.clone()
    }

    #[getter]
    pub(crate) fn match_regex(&self) -> String {
        self.match_regex.clone()
    }

    #[getter]
    pub(crate) fn timeout(&self) -> f64 {
        self.timeout
    }

    #[getter]
    pub(crate) fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// The claim is boolean: ``True`` claims the candidate host for
    /// the declaring driver, ``False`` leaves it unclaimed.
    fn __call__<'py>(
        &self,
        py: Python<'py>,
        host: String,
        port: u16,
    ) -> PyResult<PyAwait<'py, bool>> {
        let config = self.clone();
        spawn_future(py, async move {
            let matched = probe_http(
                &host,
                port,
                &HttpProbe {
                    path: &config.path,
                    match_regex: &config.regex,
                    timeout: Duration::from_secs_f64(config.timeout.max(0.01)),
                    max_bytes: config.max_bytes,
                },
            )
            .await
            .is_some();
            Ok(matched)
        })
    }
}

pub fn register(discovery_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = discovery_mod.py();
    let module = PyModule::new(py, "fingerprints")?;
    module.setattr(
        "__doc__",
        "Reusable fingerprint probes for generic mDNS service types.",
    )?;
    module.add_class::<TcpFingerprint>()?;
    module.add_class::<HttpFingerprint>()?;
    helpers::attach_submodule(
        discovery_mod,
        &module,
        "raygeo.driver.core.discovery.fingerprints",
    )?;
    Ok(())
}
