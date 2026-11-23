//! Python bindings for the document job counter.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::template::counter::CounterSettings;

/// An immutable job counter.
///
/// The counter is the number the next job starts with, the value a
/// reset returns to, the increment per copy and per job, and an
/// optional end value after which the range wraps back to ``start``.
#[gen_stub_pyclass(module = "raygeo.template")]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.template",
    name = "CounterSettings"
)]
#[derive(Clone, Debug)]
pub struct PyCounterSettings {
    pub inner: CounterSettings,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCounterSettings {
    #[new]
    #[pyo3(signature = (
        current = 1,
        start = 1,
        step = 1,
        end = None,
        auto_advance = true,
        advance_on_export = false
    ))]
    fn new(
        current: i64,
        start: i64,
        step: i64,
        end: Option<i64>,
        auto_advance: bool,
        advance_on_export: bool,
    ) -> Self {
        PyCounterSettings {
            inner: CounterSettings::new(
                current,
                start,
                step,
                end,
                auto_advance,
                advance_on_export,
            ),
        }
    }

    /// The value the next job starts with.
    #[getter]
    fn current(&self) -> i64 {
        self.inner.current
    }

    /// The value a reset returns to and a wrapped range restarts at.
    #[getter]
    fn start(&self) -> i64 {
        self.inner.start
    }

    /// The increment per copy and per job.
    #[getter]
    fn step(&self) -> i64 {
        self.inner.step
    }

    /// The optional last value of the range.
    #[getter]
    fn end(&self) -> Option<i64> {
        self.inner.end
    }

    /// Whether a finished job advances the counter.
    #[getter]
    fn auto_advance(&self) -> bool {
        self.inner.auto_advance
    }

    /// Whether exporting G-code advances the counter.
    #[getter]
    fn advance_on_export(&self) -> bool {
        self.inner.advance_on_export
    }

    /// The value shown by a copy that is ``offset`` copies further.
    fn value_at(&self, offset: i64) -> i64 {
        self.inner.value_at(offset)
    }

    /// The values of ``copies`` consecutive copies.
    fn values_for(&self, copies: usize) -> Vec<i64> {
        self.inner.values_for(copies)
    }

    /// A counter moved on (or back, if negative) by ``copies``.
    #[pyo3(signature = (copies = 1))]
    fn advanced(&self, copies: i64) -> Self {
        PyCounterSettings {
            inner: self.inner.advanced(copies),
        }
    }

    /// A counter whose current value is back at ``start``.
    fn reset(&self) -> Self {
        PyCounterSettings {
            inner: self.inner.reset(),
        }
    }

    /// Serializes the counter to the project dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("current", self.inner.current)?;
        dict.set_item("start", self.inner.start)?;
        dict.set_item("step", self.inner.step)?;
        dict.set_item("end", self.inner.end)?;
        dict.set_item("auto_advance", self.inner.auto_advance)?;
        dict.set_item("advance_on_export", self.inner.advance_on_export)?;
        Ok(dict)
    }

    /// Deserializes a counter from the project dictionary shape.
    #[staticmethod]
    fn from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        let default = CounterSettings::default();
        let start = read_i64(dict, "start")?.unwrap_or(default.start);
        let end = read_i64(dict, "end")?.filter(|end| *end >= start);
        Ok(PyCounterSettings {
            inner: CounterSettings::new(
                read_i64(dict, "current")?.unwrap_or(default.current),
                start,
                read_i64(dict, "step")?.unwrap_or(default.step),
                end,
                read_bool(dict, "auto_advance")?
                    .unwrap_or(default.auto_advance),
                read_bool(dict, "advance_on_export")?
                    .unwrap_or(default.advance_on_export),
            ),
        })
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        other
            .extract::<PyRef<Self>>()
            .is_ok_and(|other| other.inner == self.inner)
    }

    fn __repr__(&self) -> String {
        let end = self
            .inner
            .end
            .map(|value| value.to_string())
            .unwrap_or_else(|| "None".to_string());
        let auto_advance = if self.inner.auto_advance {
            "True"
        } else {
            "False"
        };
        let advance_on_export = if self.inner.advance_on_export {
            "True"
        } else {
            "False"
        };
        format!(
            "CounterSettings(current={}, start={}, step={}, end={end}, \
             auto_advance={auto_advance}, advance_on_export={advance_on_export})",
            self.inner.current,
            self.inner.start,
            self.inner.step,
        )
    }
}

fn read_i64(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<i64>> {
    match dict.get_item(key)? {
        Some(value) if !value.is_none() => Ok(Some(value.extract()?)),
        _ => Ok(None),
    }
}

fn read_bool(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<bool>> {
    match dict.get_item(key)? {
        Some(value) if !value.is_none() => Ok(Some(value.extract()?)),
        _ => Ok(None),
    }
}

pub(crate) fn add_to(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCounterSettings>()?;
    Ok(())
}
