//! Python bindings for template placeholder scanning.

use std::collections::BTreeSet;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::template::placeholders;

/// The variable names used in the `{...}` placeholders of a template.
#[gen_stub_pyfunction(
    python = r#"
    def referenced_names(template: str) -> set[str]:
        """Return the variable names a template reads.

        A format spec after the first colon is ignored. String literals,
        keywords and attribute names are not reported.
        """
"#,
    module = "raygeo.template"
)]
#[pyfunction(name = "referenced_names")]
fn py_referenced_names(template: &str) -> BTreeSet<String> {
    placeholders::referenced_names(template)
}

/// The variable names a single expression reads.
#[gen_stub_pyfunction(
    python = r#"
    def names_in_expression(expression: str) -> set[str]:
        """Return the variable names an expression reads.

        String literals, keywords, numeric literals and attribute names
        are ignored.
        """
"#,
    module = "raygeo.template"
)]
#[pyfunction(name = "names_in_expression")]
fn py_names_in_expression(expression: &str) -> BTreeSet<String> {
    placeholders::names_in_expression(expression)
}

/// The placeholder names of a template, including dotted path names.
#[gen_stub_pyfunction(
    python = r#"
    def placeholder_names(template: str) -> set[str]:
        """Return the placeholder names of a template.

        Dotted path names such as ``workpiece.counter`` are returned
        verbatim; a format spec after the first colon is not part of
        the name.
        """
"#,
    module = "raygeo.template"
)]
#[pyfunction(name = "placeholder_names")]
fn py_placeholder_names(template: &str) -> BTreeSet<String> {
    placeholders::placeholder_names(template)
}

pub(crate) fn add_to(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_referenced_names, m)?)?;
    m.add_function(wrap_pyfunction!(py_names_in_expression, m)?)?;
    m.add_function(wrap_pyfunction!(py_placeholder_names, m)?)?;
    Ok(())
}
