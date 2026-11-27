//! Python adapter for hosted sessions.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pyo3::exceptions::{
    PyConnectionError, PyNotImplementedError, PyTimeoutError,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::Value;

use crate::driver::core::driver::DriverError;
use crate::driver::core::remote::protocol::WireEvent;
use crate::driver::core::remote::session::{
    HostedSession, RemoteEventSink, SessionFactory,
};
use crate::driver::core::types::DeviceState;
use crate::python::driver::core::json::value_to_py;
use crate::python::driver::core::types::DeviceState as PyDeviceState;
use crate::python::driver::remote::bridge::{
    call_method_on_loop, json_to_py_tuples, py_to_json,
};
use crate::python::driver::remote::events::{event_to_wire, extract_event};

type BoxFuture = Pin<Box<dyn Future<Output = PyResult<Py<PyAny>>> + Send>>;

fn classify(py: Python<'_>, err: &PyErr) -> DriverError {
    if err.is_instance_of::<PyTimeoutError>(py) {
        DriverError::Timeout(err.to_string())
    } else if err.is_instance_of::<PyNotImplementedError>(py)
        || err.is_instance_of::<pyo3::exceptions::PyAttributeError>(py)
    {
        DriverError::Unsupported(err.to_string())
    } else if err.is_instance_of::<PyConnectionError>(py) {
        DriverError::Connection(err.to_string())
    } else {
        DriverError::Other(err.to_string())
    }
}

#[pyclass(module = "raygeo.driver.remote")]
pub(crate) struct EventCallback {
    sink: Arc<Mutex<Option<Arc<dyn RemoteEventSink>>>>,
}

#[pymethods]
impl EventCallback {
    fn __call__(&self, py: Python<'_>, event: Py<PyAny>) {
        let Some(event) = extract_event(py, &event) else {
            return;
        };
        let sink = self.sink.lock().unwrap().clone();
        if let Some(sink) = sink {
            sink.emit(event_to_wire(&event));
        }
    }
}

#[pyclass(module = "raygeo.driver.remote")]
struct ProgressCallback {
    sink: Arc<dyn RemoteEventSink>,
}

#[pymethods]
impl ProgressCallback {
    fn __call__(&self, op: i64) {
        self.sink.emit(WireEvent::Progress { op });
    }
}

pub(crate) struct PySessionAdapter {
    protocol: String,
    session: Py<PyAny>,
    loop_: Py<PyAny>,
    sink: Arc<Mutex<Option<Arc<dyn RemoteEventSink>>>>,
}

impl PySessionAdapter {
    async fn call_async(
        &self,
        method: &str,
        args: Vec<Py<PyAny>>,
        kwargs: Option<Py<PyAny>>,
    ) -> Result<Py<PyAny>, DriverError> {
        let future: BoxFuture = Python::attach(|py| {
            let session = self.session.clone_ref(py);
            let loop_ = self.loop_.clone_ref(py);
            let future = call_method_on_loop(
                py,
                &loop_,
                session,
                method.to_string(),
                args,
                kwargs,
            )?;
            Ok::<_, PyErr>(Box::pin(future))
        })
        .map_err(|err| Python::attach(|py| classify(py, &err)))?;
        future
            .await
            .map_err(|err| Python::attach(|py| classify(py, &err)))
    }

    fn call_sync(
        &self,
        method: &str,
        kwargs: Option<Py<PyAny>>,
    ) -> Result<Value, DriverError> {
        let result: PyResult<Value> = Python::attach(|py| {
            let session = self.session.bind(py);
            let kwargs = kwargs.as_ref().map(|kwargs| kwargs.bind(py));
            let kwargs = match kwargs {
                Some(kwargs) => Some(
                    kwargs
                        .cast::<PyDict>()
                        .map_err(|_| {
                            pyo3::exceptions::PyTypeError::new_err(
                                "kwargs must be a dict",
                            )
                        })?
                        .clone(),
                ),
                None => None,
            };
            let result = session.call_method(method, (), kwargs.as_ref())?;
            py_to_json(&result)
        });
        result.map_err(|err| Python::attach(|py| classify(py, &err)))
    }

    fn config_kwargs(&self, args: &Value) -> Result<Py<PyAny>, DriverError> {
        Python::attach(|py| -> PyResult<Py<PyAny>> {
            let dict = PyDict::new(py);
            if let Some(entries) = args.as_object() {
                for (key, value) in entries {
                    dict.set_item(key, json_to_py_tuples(py, value)?)?;
                }
            }
            Ok(dict.unbind().into_any())
        })
        .map_err(|err| Python::attach(|py| classify(py, &err)))
    }
}

#[async_trait]
impl HostedSession for PySessionAdapter {
    fn protocol(&self) -> &str {
        &self.protocol
    }

    fn state(&self) -> DeviceState {
        Python::attach(|py| {
            let value = self.session.bind(py).getattr("state")?;
            let state = value.extract::<PyRef<PyDeviceState>>()?;
            Ok::<_, PyErr>(state.inner.clone())
        })
        .unwrap_or_default()
    }

    fn job_running(&self) -> bool {
        Python::attach(|py| {
            self.session
                .bind(py)
                .getattr("job_running")?
                .extract::<bool>()
        })
        .unwrap_or(false)
    }

    fn resource_uri(&self) -> Option<String> {
        Python::attach(|py| {
            self.session
                .bind(py)
                .getattr("resource_uri")?
                .extract::<Option<String>>()
        })
        .ok()
        .flatten()
    }

    fn buffer_count(&self) -> usize {
        Python::attach(|py| {
            self.session
                .bind(py)
                .getattr("buffer_count")?
                .extract::<usize>()
        })
        .unwrap_or(0)
    }

    fn rx_buffer_size(&self) -> usize {
        Python::attach(|py| {
            self.session
                .bind(py)
                .getattr("rx_buffer_size")?
                .extract::<usize>()
        })
        .unwrap_or(0)
    }

    fn pending_commands(&self) -> Vec<String> {
        Python::attach(|py| {
            self.session
                .bind(py)
                .getattr("pending_commands")?
                .extract::<Vec<String>>()
        })
        .unwrap_or_default()
    }

    fn set_event_sink(&self, sink: Arc<dyn RemoteEventSink>) {
        *self.sink.lock().unwrap() = Some(sink);
    }

    async fn connect(&self) -> Result<(), DriverError> {
        self.call_async("connect", Vec::new(), None)
            .await
            .map(|_| ())
    }

    async fn disconnect(&self) -> Result<(), DriverError> {
        self.call_async("disconnect", Vec::new(), None)
            .await
            .map(|_| ())
    }

    async fn execute(
        &self,
        method: &str,
        args: Value,
    ) -> Result<Value, DriverError> {
        let kwargs = self.config_kwargs(&args)?;
        if method == "update_dialect" {
            return self.call_sync(method, Some(kwargs));
        }
        let result = self.call_async(method, Vec::new(), Some(kwargs)).await?;
        Python::attach(|py| py_to_json(result.bind(py)))
            .map_err(|err| Python::attach(|py| classify(py, &err)))
    }

    async fn run(
        &self,
        gcode: String,
        op_map: Option<HashMap<usize, i64>>,
        op_estimates: Option<Vec<f64>>,
    ) -> Result<(), DriverError> {
        let sink = self.sink.lock().unwrap().clone().ok_or_else(|| {
            DriverError::Other("session has no event sink".to_string())
        })?;
        let kwargs = Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("gcode", gcode)?;
            if let Some(op_map) = &op_map {
                let map = PyDict::new(py);
                for (line, op) in op_map {
                    map.set_item(line, op)?;
                }
                dict.set_item("op_line_map", map)?;
            }
            if let Some(estimates) = &op_estimates {
                dict.set_item("op_estimates", estimates)?;
            }
            let callback = Py::new(py, ProgressCallback { sink })?;
            dict.set_item("progress_callback", callback)?;
            Ok::<_, PyErr>(dict.unbind().into_any())
        })
        .map_err(|err| Python::attach(|py| classify(py, &err)))?;
        self.call_async("run", Vec::new(), Some(kwargs))
            .await
            .map(|_| ())
    }
}

pub(crate) struct PySessionFactory {
    callback: Option<Py<PyAny>>,
    loop_: Py<PyAny>,
}

impl PySessionFactory {
    pub(crate) fn new(callback: Option<Py<PyAny>>, loop_: Py<PyAny>) -> Self {
        Self { callback, loop_ }
    }

    pub(crate) fn default_session(
        py: Python<'_>,
        protocol: &str,
        config: Py<PyAny>,
        dialect: Py<PyAny>,
        callback: Py<PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let driver_mod = py.import("raygeo.driver")?;
        let cls = driver_mod.call_method1("get_driver_cls", (protocol,))?;
        let kwargs = PyDict::new(py);
        kwargs.set_item("config", config)?;
        kwargs.set_item("dialect", dialect)?;
        kwargs.set_item("event_callback", callback)?;
        Ok(cls.call((), Some(&kwargs))?.unbind())
    }
}

impl SessionFactory for PySessionFactory {
    fn create(
        &self,
        protocol: &str,
        config: Value,
        dialect: Value,
        sink: Arc<dyn RemoteEventSink>,
    ) -> Result<Arc<dyn HostedSession>, DriverError> {
        Python::attach(|py| {
            let sink_slot = Arc::new(Mutex::new(Some(sink)));
            let callback = Py::new(
                py,
                EventCallback {
                    sink: sink_slot.clone(),
                },
            )?;
            let config = value_to_py(py, &config)?;
            let dialect = value_to_py(py, &dialect)?;
            let session = if let Some(factory) = &self.callback {
                factory
                    .bind(py)
                    .call1((protocol, config, dialect, callback))?
                    .unbind()
            } else {
                PySessionFactory::default_session(
                    py,
                    protocol,
                    config,
                    dialect,
                    callback.into_any(),
                )?
            };
            Ok::<_, PyErr>(Arc::new(PySessionAdapter {
                protocol: protocol.to_string(),
                session,
                loop_: self.loop_.clone_ref(py),
                sink: sink_slot,
            }) as Arc<dyn HostedSession>)
        })
        .map_err(|err| DriverError::Other(err.to_string()))
    }
}
