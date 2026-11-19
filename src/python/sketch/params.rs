//! Python bindings for the sketch parameter context.

use std::collections::BTreeMap;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::{value_from_dict, value_to_py};
use crate::sketch::params::ParameterContext;

#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.params",
    name = "ParameterContext",
    skip_from_py_object
)]
#[derive(Default)]
pub struct PyParameterContext {
    inner: ParameterContext,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyParameterContext {
    /// Creates an empty parameter context.
    #[new]
    fn new() -> Self {
        PyParameterContext::default()
    }

    /// Sets a parameter from a number or an expression string.
    fn set(&mut self, name: String, value: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(number) = value.extract::<f64>() {
            self.inner.set_value(&name, number);
            return Ok(());
        }
        if let Ok(expression) = value.extract::<String>() {
            self.inner.set_expression(&name, &expression);
            return Ok(());
        }
        Err(PyTypeError::new_err(
            "parameter value must be a number or a string",
        ))
    }

    /// Returns the evaluated value of a parameter.
    #[pyo3(signature = (name, default = 0.0))]
    fn get(&mut self, name: String, default: f64) -> f64 {
        self.inner.get(&name, default)
    }

    /// Returns a copy of all evaluated parameter values.
    fn get_all_values(&mut self) -> BTreeMap<String, f64> {
        self.inner.get_all_values()
    }

    /// Evaluates an expression using the current parameters.
    fn evaluate(&mut self, expression: &Bound<'_, PyAny>) -> PyResult<f64> {
        let expression = if let Ok(number) = expression.extract::<f64>() {
            return Ok(number);
        } else if let Ok(text) = expression.extract::<String>() {
            text
        } else {
            return Err(PyTypeError::new_err(
                "expression must be a number or a string",
            ));
        };
        self.inner
            .evaluate(&expression)
            .map_err(|error| PyValueError::new_err(error.to_string()))
    }

    /// Resolves every expression, optionally seeded with initial values.
    #[pyo3(signature = (initial_values = None))]
    fn evaluate_all(&mut self, initial_values: Option<BTreeMap<String, f64>>) {
        self.inner.evaluate_all(initial_values);
    }

    /// Expressions that remained unresolved during the last evaluation.
    fn unresolved_errors(&self) -> BTreeMap<String, String> {
        self.inner.unresolved_errors().clone()
    }

    /// Clears cached values so the next access re-evaluates everything.
    fn invalidate(&mut self) {
        self.inner.invalidate();
    }

    /// Serializes the context to the reference dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        let dict = PyDict::new(py);
        if let serde_json::Value::Object(map) = value {
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, &item)?)?;
            }
        }
        Ok(dict)
    }

    /// Deserializes a context from the reference dictionary shape.
    #[staticmethod]
    fn from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        let value = value_from_dict(dict)?;
        ParameterContext::from_json(&value)
            .map(|inner| PyParameterContext { inner })
            .ok_or_else(|| PyValueError::new_err("invalid parameter context"))
    }

    fn __repr__(&self) -> String {
        format!("ParameterContext({})", self.inner.to_json())
    }
}

pyo3_stub_gen::module_doc!("raygeo.sketch.params", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Named sketch parameters and expressions.

A parameter may be a number or a math expression. Expressions are
resolved in multiple passes so they can reference parameters defined
later, and unresolved expressions are reported through
``unresolved_errors()``. The context serializes to the reference
``{'expressions': {...}}`` dictionary.
";

/// Registers the parameter context with the given module.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let params_mod = PyModule::new(py, "params")?;

    params_mod.setattr("__doc__", MODULE_DOC)?;
    params_mod.add_class::<PyParameterContext>()?;

    m.add_submodule(&params_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.params", &params_mod)?;

    Ok(())
}
