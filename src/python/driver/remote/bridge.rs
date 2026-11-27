//! Bridges Python asyncio coroutines into Rust futures.
//!
//! The daemon and client cores run on tokio worker threads, while
//! Python sessions live on the asyncio loop. Every call that awaits
//! a Python coroutine is scheduled onto that loop with
//! `call_soon_threadsafe`; the result comes back through a oneshot
//! channel.

use std::future::Future;
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3::types::{PyCFunction, PyDict, PyList, PyTuple};
use serde_json::{Map, Value};

/// The running asyncio loop. Must be called on the loop thread.
pub(crate) fn running_loop(py: Python<'_>) -> PyResult<Py<PyAny>> {
    Ok(py
        .import("asyncio")?
        .call_method0("get_running_loop")?
        .unbind())
}

/// Runs *make* on the asyncio loop, awaits its coroutine and returns
/// the result. Safe to call from any thread.
pub(crate) fn call_on_loop<F>(
    py: Python<'_>,
    loop_: &Py<PyAny>,
    make: F,
) -> PyResult<impl Future<Output = PyResult<Py<PyAny>>> + Send>
where
    F: FnOnce(Python<'_>) -> PyResult<Py<PyAny>> + Send + 'static,
{
    let make = Arc::new(Mutex::new(Some(make)));
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let callback = PyCFunction::new_closure(
        py,
        None,
        None,
        move |_args: &Bound<'_, PyTuple>, _kwargs| -> PyResult<Py<PyAny>> {
            let py = _args.py();
            let Some(make) = make.lock().unwrap().take() else {
                return Ok(py.None());
            };
            // Complete the waiting Rust future with the error instead
            // of raising it into the asyncio exception handler: the
            // caller must see a failed request, not a silent hang.
            let fail = |err: PyErr| -> PyResult<Py<PyAny>> {
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(Err(err));
                }
                Ok(py.None())
            };
            let awaitable = match make(py) {
                Ok(awaitable) => awaitable,
                Err(err) => return fail(err),
            };
            let asyncio = match py.import("asyncio") {
                Ok(asyncio) => asyncio,
                Err(err) => return fail(err),
            };
            let task = match asyncio.call_method1("ensure_future", (awaitable,))
            {
                Ok(task) => task,
                Err(err) => return fail(err),
            };
            let tx = tx.clone();
            let done = PyCFunction::new_closure(
                py,
                None,
                None,
                move |args: &Bound<'_, PyTuple>,
                      _kwargs|
                      -> PyResult<Py<PyAny>> {
                    let py = args.py();
                    let task = args.get_item(0)?;
                    let outcome: PyResult<Py<PyAny>> = (|| {
                        let exc = task.call_method0("exception")?;
                        if !exc.is_none() {
                            return Err(PyErr::from_value(exc));
                        }
                        Ok(task.call_method0("result")?.unbind())
                    })();
                    if let Some(tx) = tx.lock().unwrap().take() {
                        let _ = tx.send(outcome);
                    }
                    Ok(py.None())
                },
            )?;
            if let Err(err) = task.call_method1("add_done_callback", (done,)) {
                return fail(err);
            }
            Ok(py.None())
        },
    )?;
    loop_
        .bind(py)
        .call_method1("call_soon_threadsafe", (callback,))?;
    Ok(async move {
        rx.await.map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "asyncio task dropped before completion",
            )
        })?
    })
}

/// Calls `target.method(*args, **kwargs)` on the loop and awaits it.
pub(crate) fn call_method_on_loop(
    py: Python<'_>,
    loop_: &Py<PyAny>,
    target: Py<PyAny>,
    method: String,
    args: Vec<Py<PyAny>>,
    kwargs: Option<Py<PyAny>>,
) -> PyResult<impl Future<Output = PyResult<Py<PyAny>>> + Send> {
    call_on_loop(py, loop_, move |py| {
        let target = target.bind(py);
        let tuple = PyTuple::new(
            py,
            args.iter().map(|arg| arg.bind(py)).collect::<Vec<_>>(),
        )?;
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
        Ok(target
            .call_method(&method, tuple, kwargs.as_ref())?
            .unbind())
    })
}

/// Converts a Python value to JSON.
pub(crate) fn py_to_json(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if value.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(dict) = value.call_method0("to_dict") {
        return py_to_json(&dict);
    }
    if let Ok(flag) = value.extract::<bool>() {
        if value.is_instance_of::<pyo3::types::PyBool>() {
            return Ok(Value::Bool(flag));
        }
    }
    if let Ok(int) = value.extract::<i64>() {
        return Ok(Value::Number(int.into()));
    }
    if let Ok(float) = value.extract::<f64>() {
        if let Some(number) = serde_json::Number::from_f64(float) {
            return Ok(Value::Number(number));
        }
    }
    if let Ok(text) = value.extract::<String>() {
        return Ok(Value::String(text));
    }
    if let Ok(list) = value.cast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(py_to_json(&item)?);
        }
        return Ok(Value::Array(items));
    }
    if let Ok(tuple) = value.cast::<PyTuple>() {
        let mut items = Vec::with_capacity(tuple.len());
        for item in tuple.iter() {
            items.push(py_to_json(&item)?);
        }
        return Ok(Value::Array(items));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let mut map = Map::new();
        for (key, item) in dict.iter() {
            map.insert(key.extract::<String>()?, py_to_json(&item)?);
        }
        return Ok(Value::Object(map));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "value is not JSON serializable",
    ))
}

/// Converts JSON to Python, restoring arrays as tuples for session
/// arguments whose Rust methods extract tuple sequences.
pub(crate) fn json_to_py_tuples(
    py: Python<'_>,
    value: &Value,
) -> PyResult<Py<PyAny>> {
    match value {
        Value::Array(items) => {
            let converted: PyResult<Vec<Py<PyAny>>> = items
                .iter()
                .map(|item| json_to_py_tuples(py, item))
                .collect();
            Ok(PyTuple::new(py, converted?)?.unbind().into_any())
        }
        Value::Object(entries) => {
            let dict = PyDict::new(py);
            for (key, item) in entries {
                dict.set_item(key, json_to_py_tuples(py, item)?)?;
            }
            Ok(dict.unbind().into_any())
        }
        _ => crate::python::driver::core::json::value_to_py(py, value),
    }
}
