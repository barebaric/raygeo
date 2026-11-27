//! Python bindings for the remote daemon.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use pyo3::exceptions::{PyOverflowError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};
use serde_json::Value;

use crate::driver::core::remote::daemon::{
    self as core, ClientQueue as CoreClientQueue, Daemon as CoreDaemon,
    DaemonConfig as CoreConfig, DaemonError, SEND_QUEUE_LIMIT,
};
use crate::python::driver::core::json::{value_to_dict, JsonDict};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};
use crate::python::driver::remote::bridge::{py_to_json, running_loop};
use crate::python::driver::remote::session::PySessionFactory;

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.daemon")]
pub struct DaemonConfig {
    pub(crate) inner: Arc<Mutex<CoreConfig>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl DaemonConfig {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        host = "127.0.0.1",
        port = core::DEFAULT_PORT,
        token = None,
        tls_cert = None,
        tls_key = None,
        keepalive_interval = 2.0,
        keepalive_misses = 3,
        lease_grace = 5.0,
        ring_size = 1000,
        cameras = None,
        sessions = None,
        mdns = true,
    ))]
    fn new(
        py: Python<'_>,
        host: &str,
        port: u16,
        token: Option<String>,
        tls_cert: Option<String>,
        tls_key: Option<String>,
        keepalive_interval: f64,
        keepalive_misses: u32,
        lease_grace: f64,
        ring_size: usize,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Sequence[dict[builtins.str, builtins.object]]]",
            imports = ("typing", "builtins")
        ))]
        cameras: Option<Py<PyAny>>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Sequence[dict[builtins.str, builtins.object]]]",
            imports = ("typing", "builtins")
        ))]
        sessions: Option<Py<PyAny>>,
        mdns: bool,
    ) -> PyResult<Self> {
        let cameras = match cameras {
            Some(cameras) => py_to_json(cameras.bind(py))?,
            None => Value::Array(Vec::new()),
        };
        let sessions = match sessions {
            Some(sessions) => py_to_json(sessions.bind(py))?,
            None => Value::Array(Vec::new()),
        };
        Ok(Self {
            inner: Arc::new(Mutex::new(CoreConfig {
                host: host.to_string(),
                port,
                token,
                tls_cert,
                tls_key,
                keepalive_interval,
                keepalive_misses,
                lease_grace,
                ring_size,
                cameras,
                sessions,
                mdns,
            })),
        })
    }

    #[getter]
    fn host(&self) -> String {
        self.inner.lock().unwrap().host.clone()
    }

    #[setter]
    fn set_host(&self, value: String) {
        self.inner.lock().unwrap().host = value;
    }

    #[getter]
    fn port(&self) -> u16 {
        self.inner.lock().unwrap().port
    }

    #[setter]
    fn set_port(&self, value: u16) {
        self.inner.lock().unwrap().port = value;
    }

    #[getter]
    fn token(&self) -> Option<String> {
        self.inner.lock().unwrap().token.clone()
    }

    #[setter]
    fn set_token(&self, value: Option<String>) {
        self.inner.lock().unwrap().token = value;
    }

    #[getter]
    fn tls_cert(&self) -> Option<String> {
        self.inner.lock().unwrap().tls_cert.clone()
    }

    #[setter]
    fn set_tls_cert(&self, value: Option<String>) {
        self.inner.lock().unwrap().tls_cert = value;
    }

    #[getter]
    fn tls_key(&self) -> Option<String> {
        self.inner.lock().unwrap().tls_key.clone()
    }

    #[setter]
    fn set_tls_key(&self, value: Option<String>) {
        self.inner.lock().unwrap().tls_key = value;
    }

    #[getter]
    fn keepalive_interval(&self) -> f64 {
        self.inner.lock().unwrap().keepalive_interval
    }

    #[setter]
    fn set_keepalive_interval(&self, value: f64) {
        self.inner.lock().unwrap().keepalive_interval = value;
    }

    #[getter]
    fn keepalive_misses(&self) -> u32 {
        self.inner.lock().unwrap().keepalive_misses
    }

    #[setter]
    fn set_keepalive_misses(&self, value: u32) {
        self.inner.lock().unwrap().keepalive_misses = value;
    }

    #[getter]
    fn lease_grace(&self) -> f64 {
        self.inner.lock().unwrap().lease_grace
    }

    #[setter]
    fn set_lease_grace(&self, value: f64) {
        self.inner.lock().unwrap().lease_grace = value;
    }

    #[getter]
    fn ring_size(&self) -> usize {
        self.inner.lock().unwrap().ring_size
    }

    #[setter]
    fn set_ring_size(&self, value: usize) {
        self.inner.lock().unwrap().ring_size = value;
    }

    #[getter]
    fn cameras(&self, py: Python<'_>) -> PyResult<Vec<JsonDict>> {
        let value = self.inner.lock().unwrap().cameras.clone();
        decode_dict_list(py, &value)
    }

    #[setter]
    fn set_cameras(
        &self,
        #[gen_stub(override_type(
            type_repr = "typing.Sequence[dict[builtins.str, builtins.object]]",
            imports = ("typing", "builtins")
        ))]
        value: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.inner.lock().unwrap().cameras = py_to_json(&value)?;
        Ok(())
    }

    #[getter]
    fn sessions(&self, py: Python<'_>) -> PyResult<Vec<JsonDict>> {
        let value = self.inner.lock().unwrap().sessions.clone();
        decode_dict_list(py, &value)
    }

    #[setter]
    fn set_sessions(
        &self,
        #[gen_stub(override_type(
            type_repr = "typing.Sequence[dict[builtins.str, builtins.object]]",
            imports = ("typing", "builtins")
        ))]
        value: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.inner.lock().unwrap().sessions = py_to_json(&value)?;
        Ok(())
    }

    #[getter]
    fn mdns(&self) -> bool {
        self.inner.lock().unwrap().mdns
    }

    #[setter]
    fn set_mdns(&self, value: bool) {
        self.inner.lock().unwrap().mdns = value;
    }
}

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.daemon")]
pub struct Daemon {
    inner: Arc<CoreDaemon>,
    config: Arc<Mutex<CoreConfig>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Daemon {
    #[new]
    #[pyo3(signature = (config = None, session_factory = None))]
    fn new(
        py: Python<'_>,
        config: Option<PyRef<'_, DaemonConfig>>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[..., builtins.object]]",
            imports = ("typing", "collections.abc")
        ))]
        session_factory: Option<Py<PyAny>>,
    ) -> PyResult<Self> {
        let config = config
            .map(|config| config.inner.clone())
            .unwrap_or_else(|| Arc::new(Mutex::new(CoreConfig::default())));
        let loop_ = running_loop(py)?;
        let factory = Arc::new(PySessionFactory::new(session_factory, loop_));
        let inner = Arc::new(CoreDaemon::new(
            config.clone(),
            factory,
            crate::driver::core::remote::registry::Registry::new(
                crate::driver::registry::DriverKind::ALL
                    .iter()
                    .map(|kind| kind.info())
                    .collect(),
            ),
        ));
        Ok(Self { inner, config })
    }

    #[getter]
    fn config(&self) -> DaemonConfig {
        DaemonConfig {
            inner: self.config.clone(),
        }
    }

    fn start<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, StartedServer>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let started = inner.start().await.map_err(core_error)?;
            Ok(StartedServer { addr: started.addr })
        })
    }

    fn stop<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.stop().await;
            Ok(())
        })
    }
}

fn insecure_error(message: String) -> PyErr {
    Python::attach(|py| {
        let result: PyResult<PyErr> = (|| {
            let module = py.import("raygeo.driver.errors")?;
            let cls = module.getattr("InsecureListenerError")?;
            let value = cls.call1((message,))?;
            Ok(PyErr::from_value(value))
        })();
        result.unwrap_or_else(|_| {
            pyo3::exceptions::PyValueError::new_err("insecure listener")
        })
    })
}

fn core_error(err: DaemonError) -> PyErr {
    match err {
        DaemonError::Insecure(message) => insecure_error(message),
        other => pyo3::exceptions::PyRuntimeError::new_err(other.to_string()),
    }
}

/// Raise :class:`raydriver.errors.InsecureListenerError` unless
/// *host* is loopback or both *token* and *tls_cert* are configured.
#[gen_stub_pyfunction(module = "raygeo.driver.remote.daemon")]
#[pyfunction]
pub(crate) fn validate_bind_policy(
    host: &str,
    token: Option<&str>,
    tls_cert: Option<&str>,
) -> PyResult<()> {
    core::validate_bind_policy(host, token, tls_cert).map_err(core_error)
}

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.daemon")]
pub struct SocketInfo {
    addr: SocketAddr,
}

#[gen_stub_pymethods]
#[pymethods]
impl SocketInfo {
    fn getsockname(&self) -> (String, u16) {
        (self.addr.ip().to_string(), self.addr.port())
    }
}

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.daemon")]
pub struct StartedServer {
    addr: SocketAddr,
}

#[gen_stub_pymethods]
#[pymethods]
impl StartedServer {
    #[getter]
    #[gen_stub(override_return_type(
        type_repr = "tuple[SocketInfo, ...]",
        imports = ()
    ))]
    fn sockets(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let socket = Py::new(py, SocketInfo { addr: self.addr })?;
        Ok(PyTuple::new(py, [socket])?.unbind().into_any())
    }
}

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.daemon")]
pub struct ClientQueue {
    inner: Arc<CoreClientQueue>,
}

/// Decodes a stored config array (cameras/sessions) into JSON object
/// dicts.
fn decode_dict_list(py: Python<'_>, value: &Value) -> PyResult<Vec<JsonDict>> {
    let entries = value
        .as_array()
        .ok_or_else(|| PyValueError::new_err("expected a list of objects"))?;
    entries
        .iter()
        .map(|entry| value_to_dict(py, entry))
        .collect()
}

#[gen_stub_pymethods]
#[pymethods]
impl ClientQueue {
    #[new]
    #[pyo3(signature = (limit = SEND_QUEUE_LIMIT))]
    fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new(CoreClientQueue::new(limit)),
        }
    }

    fn put(
        &self,
        py: Python<'_>,
        #[gen_stub(override_type(
            type_repr = "dict[builtins.str, builtins.object]",
            imports = ("builtins",)
        ))]
        frame: Py<PyAny>,
    ) -> PyResult<()> {
        let frame = py_to_json(frame.bind(py))?;
        if !self.inner.push(frame) {
            return Err(PyOverflowError::new_err("client send queue overflow"));
        }
        Ok(())
    }

    fn get<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, JsonDict>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let frame = inner.get().await;
            Python::attach(|py| value_to_dict(py, &frame))
        })
    }
}

pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.daemon",
    "DEFAULT_PORT",
    u16
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.daemon",
    "DEFAULT_RING_SIZE",
    usize
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.daemon",
    "DEFAULT_LEASE_GRACE",
    f64
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.daemon",
    "SEND_QUEUE_LIMIT",
    usize
);
