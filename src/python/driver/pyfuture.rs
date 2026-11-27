//! Bridge for completing asyncio futures from the tokio runtime.
//!
//! Python async methods hand back an `asyncio.Future` that a Rust
//! future completes via `loop.call_soon_threadsafe` — mirroring the
//! pattern Rayforge's Python drivers use to wake waiters from
//! non-loop threads.  Nothing here ever blocks the event loop.

use std::future::Future;
use std::marker::PhantomData;

use pyo3::prelude::*;
use pyo3::types::{PyCFunction, PyTuple};
use pyo3::BoundObject;
use pyo3_stub_gen::TypeInfo;

use super::runtime;

/// The awaitable a Rust async session method returns to Python: the
/// asyncio future object, typed with the future's resolution type
/// for the generated stubs (``collections.abc.Awaitable[T]``).
pub(crate) struct PyAwait<'py, T> {
    future: Bound<'py, PyAny>,
    _marker: PhantomData<fn() -> T>,
}

impl<'py, T> PyAwait<'py, T> {
    pub(crate) fn new(future: Bound<'py, PyAny>) -> Self {
        Self {
            future,
            _marker: PhantomData,
        }
    }
}

impl<'py, T> IntoPyObject<'py> for PyAwait<'py, T> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, _py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(self.future)
    }
}

impl<T: pyo3_stub_gen::PyStubType> pyo3_stub_gen::PyStubType
    for PyAwait<'_, T>
{
    fn type_output() -> TypeInfo {
        let inner = T::type_output();
        let mut import = inner.import.clone();
        import.insert("collections.abc".into());
        TypeInfo {
            name: format!("collections.abc.Awaitable[{}]", inner.name),
            source_module: None,
            import,
            type_refs: inner.type_refs,
        }
    }
}

/// Spawn *fut* on the shared tokio runtime and return an asyncio
/// future that resolves with its result.
///
/// The future must be created while running on the event loop (i.e.
/// from an async Python call stack), which is the only supported
/// calling context.
pub(crate) fn spawn_future<'py, F, T>(
    py: Python<'py>,
    fut: F,
) -> PyResult<PyAwait<'py, T>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'any> IntoPyObject<'any> + Send + 'static,
{
    spawn_future_inner(py, fut, None)
}

/// Like [`spawn_future`], but runs *on_cancel* when the awaiting
/// Python task is cancelled. The Rust future is aborted either way.
pub(crate) fn spawn_future_with_cancel<'py, F, T, C>(
    py: Python<'py>,
    fut: F,
    on_cancel: C,
) -> PyResult<PyAwait<'py, T>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'any> IntoPyObject<'any> + Send + 'static,
    C: FnOnce() + Send + 'static,
{
    spawn_future_inner(py, fut, Some(Box::new(on_cancel)))
}

fn spawn_future_inner<'py, F, T>(
    py: Python<'py>,
    fut: F,
    on_cancel: Option<Box<dyn FnOnce() + Send>>,
) -> PyResult<PyAwait<'py, T>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'any> IntoPyObject<'any> + Send + 'static,
{
    let asyncio = py.import("asyncio")?;
    let loop_ = asyncio.call_method0("get_running_loop")?;
    let future = loop_.call_method1("create_future", ())?;
    let loop_py: Py<PyAny> = loop_.unbind();
    let future_py: Py<PyAny> = future.clone().unbind();

    let handle = runtime::handle().spawn(async move {
        let outcome = match fut.await {
            Ok(value) => Python::attach(|py| {
                value
                    .into_pyobject(py)
                    .map(|obj| obj.into_any().unbind())
                    .map_err(Into::into)
            }),
            Err(err) => Err(err),
        };
        let schedule: PyResult<()> = Python::attach(|py| {
            let loop_bound = loop_py.bind(py);
            // The awaiting loop may be gone or tearing down (e.g. the
            // test ended); scheduling onto it crashes on Windows.
            if !loop_accepts_callbacks(loop_bound)? {
                return Ok(());
            }
            // The awaiting task may have been cancelled while the
            // Rust side was still running (e.g. app shutdown);
            // completing a done future raises InvalidStateError in
            // the loop callback, so guard on done() at call time.
            let complete = py.eval(
                c"lambda f, v, e: None if f.done() else \
                  (f.set_exception(e) if e is not None else f.set_result(v))",
                None,
                None,
            )?;
            match outcome {
                Ok(value) => {
                    loop_bound.call_method1(
                        "call_soon_threadsafe",
                        (complete, future_py, value, py.None()),
                    )?;
                }
                Err(err) => {
                    let exc = err.value(py).to_owned();
                    loop_bound.call_method1(
                        "call_soon_threadsafe",
                        (complete, future_py, py.None(), exc),
                    )?;
                }
            }
            Ok(())
        });
        if let Err(err) = schedule {
            Python::attach(|py| err.print(py));
        }
    });
    let abort = handle.abort_handle();
    let on_cancel = std::sync::Mutex::new(on_cancel);
    let callback = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>, _kwargs| -> PyResult<Py<PyAny>> {
            let py = args.py();
            let future = args.get_item(0)?;
            if future.call_method0("cancelled")?.is_truthy()? {
                abort.abort();
                if let Some(on_cancel) = on_cancel.lock().unwrap().take() {
                    on_cancel();
                }
            }
            Ok(py.None())
        },
    )?;
    future.call_method1("add_done_callback", (callback,))?;
    Ok(PyAwait::new(future))
}

/// Whether *loop* can still receive threadsafe wake-ups: not closed
/// and running. ``close()`` only runs on a non-running loop and frees
/// the self-pipe while a worker's wake-up write may be in flight (the
/// socket send releases the GIL), which crashes on Windows — so only
/// schedule onto a live, running loop.
pub(crate) fn loop_accepts_callbacks(
    loop_bound: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    if loop_bound.call_method0("is_closed")?.is_truthy()? {
        return Ok(false);
    }
    loop_bound.call_method0("is_running")?.is_truthy()
}

/// Turns a Python awaitable into a Rust future, so Rust async code
/// can await driver-supplied coroutines (e.g. fingerprint probes).
///
/// The awaitable is wrapped in an asyncio task whose done callback
/// hands the result (or exception) to a oneshot channel; the Rust
/// side never blocks the event loop. Must be called from the loop
/// thread.
pub(crate) fn await_py(
    py: Python<'_>,
    awaitable: Bound<'_, PyAny>,
) -> PyResult<impl Future<Output = PyResult<Py<PyAny>>> + Send> {
    let asyncio = py.import("asyncio")?;
    let task = asyncio.call_method1("ensure_future", (awaitable,))?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    let callback = pyo3::types::PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, pyo3::types::PyTuple>,
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
    task.call_method1("add_done_callback", (callback,))?;
    Ok(async move {
        rx.await.map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "discovery task dropped before completion",
            )
        })?
    })
}
