//! Python bindings for template expressions and text resolution.

use std::collections::BTreeSet;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyTuple};
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::template::expr::{self, EvalError, TemplateContext, Value};
use crate::template::text::{self, TextEntry, TextPart};

struct PyTemplateContext<'py> {
    py: Python<'py>,
    values: Bound<'py, PyDict>,
    functions: Bound<'py, PyDict>,
}

impl TemplateContext for PyTemplateContext<'_> {
    fn variable(&self, name: &str) -> Option<Value> {
        let item = self.values.get_item(name).ok().flatten()?;
        Some(py_value_to_value(&item))
    }

    fn call(
        &self,
        name: &str,
        arguments: &[Value],
    ) -> Option<Result<Value, EvalError>> {
        let item = self.functions.get_item(name).ok().flatten()?;
        if !item.is_callable() {
            return None;
        }
        let args = match PyTuple::new(
            self.py,
            arguments
                .iter()
                .map(|value| value_to_py(self.py, value))
                .collect::<PyResult<Vec<_>>>()
                .ok()?,
        ) {
            Ok(args) => args,
            Err(error) => return Some(Err(EvalError::new(error.to_string()))),
        };
        match item.call1(args) {
            Ok(result) => Some(Ok(py_value_to_value(&result))),
            Err(error) => Some(Err(EvalError::new(error.to_string()))),
        }
    }
}

fn py_value_to_value(value: &Bound<'_, PyAny>) -> Value {
    if value.is_instance_of::<PyBool>() {
        if let Ok(boolean) = value.extract::<bool>() {
            return Value::Bool(boolean);
        }
    }
    if let Ok(integer) = value.extract::<i64>() {
        return Value::Int(integer);
    }
    if let Ok(float) = value.extract::<f64>() {
        return Value::Float(float);
    }
    if let Ok(text) = value.extract::<String>() {
        return Value::Text(text);
    }
    match value.str() {
        Ok(text) => Value::Text(text.to_string_lossy().into_owned()),
        Err(_) => Value::None,
    }
}

fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Int(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
        Value::Float(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
        Value::Text(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
        Value::Bool(value) => {
            Ok(value.into_pyobject(py)?.to_owned().into_any().unbind())
        }
        Value::None => Ok(py.None()),
    }
}

fn function_names(functions: &Bound<'_, PyDict>) -> BTreeSet<String> {
    functions
        .keys()
        .iter()
        .filter_map(|key| key.extract::<String>().ok())
        .collect()
}

fn stamp_to_py(
    py: Python<'_>,
    stamp: &[(String, String)],
) -> PyResult<Py<PyAny>> {
    let list = PyList::empty(py);
    for (name, value) in stamp {
        list.append((name, value))?;
    }
    Ok(list.into_any().unbind())
}

fn entry_to_py(py: Python<'_>, entry: &TextEntry) -> PyResult<Py<PyAny>> {
    let resolved = match &entry.resolved {
        Some(value) => value.into_pyobject(py)?.into_any().unbind(),
        None => py.None(),
    };
    let stamp = stamp_to_py(py, &entry.stamp)?;
    let parts = PyList::empty(py);
    for part in &entry.parts {
        parts.append((
            part.placeholder.as_str(),
            part.resolved.as_str(),
            stamp_to_py(py, &part.stamp)?,
        ))?;
    }
    let tuple = PyTuple::new(
        py,
        [
            entry.source.as_str().into_pyobject(py)?.into_any().unbind(),
            resolved,
            stamp,
            parts.into_any().unbind(),
        ],
    )?;
    Ok(tuple.into_any().unbind())
}

fn entry_from_py(entry: &Bound<'_, PyAny>) -> Option<TextEntry> {
    if entry.is_none() {
        return None;
    }
    let source = entry.get_item(0).ok()?.extract::<String>().ok()?;
    let resolved = match entry.get_item(1) {
        Ok(value) if !value.is_none() => Some(value.extract::<String>().ok()?),
        _ => None,
    };
    Some(TextEntry {
        source,
        resolved,
        stamp: stamp_from_py(entry.get_item(2).ok().as_ref())?,
        parts: parts_from_py(entry.get_item(3).ok().as_ref())
            .unwrap_or_default(),
    })
}

fn stamp_from_py(
    stamp: Option<&Bound<'_, PyAny>>,
) -> Option<Vec<(String, String)>> {
    let stamp = stamp?;
    if stamp.is_none() {
        return Some(Vec::new());
    }
    let mut pairs = Vec::new();
    for pair in stamp.try_iter().ok()? {
        let pair = pair.ok()?;
        let name = pair.get_item(0).ok()?.extract::<String>().ok()?;
        let value = pair.get_item(1).ok()?.extract::<String>().ok()?;
        pairs.push((name, value));
    }
    Some(pairs)
}

fn parts_from_py(parts: Option<&Bound<'_, PyAny>>) -> Option<Vec<TextPart>> {
    let parts = parts?;
    if parts.is_none() {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for part in parts.try_iter().ok()? {
        let part = part.ok()?;
        result.push(TextPart {
            placeholder: part.get_item(0).ok()?.extract().ok()?,
            resolved: part.get_item(1).ok()?.extract().ok()?,
            stamp: stamp_from_py(part.get_item(2).ok().as_ref())?,
        });
    }
    Some(result)
}

fn context<'py>(
    py: Python<'py>,
    values: &Bound<'py, PyDict>,
    functions: &Bound<'py, PyDict>,
) -> PyTemplateContext<'py> {
    PyTemplateContext {
        py,
        values: values.clone(),
        functions: functions.clone(),
    }
}

/// Resolves a text template, reusing unchanged placeholders.
#[gen_stub_pyfunction(
    python = r#"
    def resolve_text(
        template: str,
        values: dict,
        functions: dict,
        previous: tuple | None = None,
    ) -> tuple[str | None, tuple, bool]:
        """Resolve a text template.

        ``values`` are the variables, ``functions`` the host's zero-argument
        template functions, and ``previous`` the entry returned by an earlier
        call. Returns ``(resolved, entry, volatile)``.
        """
"#,
    module = "raygeo.template"
)]
#[pyfunction]
#[pyo3(signature = (template, values, functions, previous = None))]
fn resolve_text<'py>(
    py: Python<'py>,
    template: &str,
    values: &Bound<'py, PyDict>,
    functions: &Bound<'py, PyDict>,
    previous: Option<&Bound<'py, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let context = context(py, values, functions);
    let names = function_names(functions);
    let previous = previous.and_then(entry_from_py);
    let result =
        text::resolve_template(template, &context, &names, previous.as_ref());
    let resolved = match result.resolved {
        Some(value) => value.into_pyobject(py)?.into_any().unbind(),
        None => py.None(),
    };
    let tuple = PyTuple::new(
        py,
        [
            resolved,
            entry_to_py(py, &result.entry)?,
            result
                .volatile
                .into_pyobject(py)?
                .to_owned()
                .into_any()
                .unbind(),
        ],
    )?;
    Ok(tuple.into_any().unbind())
}

/// Evaluates an expression and returns its Python value.
#[gen_stub_pyfunction(
    python = r#"
    def evaluate(
        expression: str,
        values: dict,
        functions: dict | None = None,
    ) -> object:
        """Evaluate an expression with host variables and functions."""
"#,
    module = "raygeo.template"
)]
#[pyfunction]
#[pyo3(signature = (expression, values, functions = None))]
fn evaluate<'py>(
    py: Python<'py>,
    expression: &str,
    values: &Bound<'py, PyDict>,
    functions: Option<&Bound<'py, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let empty = PyDict::new(py);
    let functions = functions.unwrap_or(&empty);
    let context = context(py, values, functions);
    let value = expr::evaluate(expression, &context)
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    value_to_py(py, &value)
}

/// Evaluates an expression as a floating-point number.
#[gen_stub_pyfunction(
    python = r#"
    def evaluate_number(
        expression: str,
        values: dict,
        functions: dict | None = None,
    ) -> float:
        """Evaluate an expression and return a float."""
"#,
    module = "raygeo.template"
)]
#[pyfunction]
#[pyo3(signature = (expression, values, functions = None))]
fn evaluate_number(
    py: Python<'_>,
    expression: &str,
    values: &Bound<'_, PyDict>,
    functions: Option<&Bound<'_, PyDict>>,
) -> PyResult<f64> {
    let empty = PyDict::new(py);
    let functions = functions.unwrap_or(&empty);
    let context = context(py, values, functions);
    expr::evaluate_number(expression, &context)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Formats a value with a Python-style format spec.
#[gen_stub_pyfunction(
    python = r#"
    def format_value(value: object, spec: str) -> str:
        """Format a value with a Python-style format spec."""
"#,
    module = "raygeo.template"
)]
#[pyfunction]
fn format_value(value: &Bound<'_, PyAny>, spec: &str) -> String {
    expr::format_value(&py_value_to_value(value), spec)
}

pub(crate) fn add_to(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(resolve_text, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_number, m)?)?;
    m.add_function(wrap_pyfunction!(format_value, m)?)?;
    Ok(())
}
