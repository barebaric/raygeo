use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

/// Resolve named substitution variables in a template string.
///
/// Replaces ``{name}`` placeholders using the provided dict. Unknown
/// placeholders (including path-style ``{machine.name}``) are left
/// verbatim for a subsequent ``resolve_path_vars`` call.
#[gen_stub_pyfunction(
    python = r#"
    def render_named(
        template: str,
        vars: dict[str, str],
    ) -> str:
        """Resolve named substitution variables in a template string.

        Replaces ``{name}`` placeholders using the provided dict.
        Unknown placeholders (including path-style ``{machine.name}``)
        are left verbatim for a subsequent ``resolve_path_vars`` call.
        """
"#,
    module = "raygeo.fstring"
)]
#[pyfunction(name = "render_named")]
fn py_render_named(template: &str, vars: HashMap<String, String>) -> String {
    let mut nv = crate::fstring::NamedVars::default();
    for (k, v) in &vars {
        let key: &'static str = Box::leak(k.clone().into_boxed_str());
        nv.set_str(key, v);
    }
    crate::fstring::render_named(template, &nv)
}

/// Resolve path-style placeholders (``{machine.name}``,
/// ``{job.extents[0]}``) using a flat dict. Unresolved placeholders
/// are left verbatim.
#[gen_stub_pyfunction(
    python = r#"
    def resolve_path_vars(
        template: str,
        path_vars: dict[str, str],
    ) -> str:
        """Resolve path-style placeholders using a flat dict.

        Replaces ``{machine.name}``, ``{job.extents[0]}`` etc. using
        the provided dict. Unresolved placeholders are left verbatim.
        """
"#,
    module = "raygeo.fstring"
)]
#[pyfunction(name = "resolve_path_vars")]
fn py_resolve_path_vars(
    template: &str,
    path_vars: HashMap<String, String>,
) -> String {
    crate::fstring::resolve_path_vars(template, &path_vars)
}

/// Parse an ``@include(MacroName)`` directive, returning the macro
/// name (stripped of whitespace) or ``None`` if the line is not an
/// include directive.
#[gen_stub_pyfunction(
    python = r#"
    def parse_include_directive(
        line: str,
    ) -> str | None:
        """Parse an ``@include(MacroName)`` directive.

        Returns the macro name (stripped of whitespace) or ``None`` if
        the line is not an include directive.
        """
"#,
    module = "raygeo.fstring"
)]
#[pyfunction(name = "parse_include_directive")]
fn py_parse_include_directive(line: &str) -> Option<String> {
    crate::fstring::parse_include_directive(line)
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let mod_ = PyModule::new(parent.py(), "fstring")?;

    mod_.add(
        "__all__",
        vec![
            "parse_include_directive",
            "render_named",
            "resolve_path_vars",
        ],
    )?;

    mod_.add_function(wrap_pyfunction!(py_render_named, &mod_)?)?;
    mod_.add_function(wrap_pyfunction!(py_resolve_path_vars, &mod_)?)?;
    mod_.add_function(wrap_pyfunction!(py_parse_include_directive, &mod_)?)?;

    parent.add_submodule(&mod_)?;

    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.fstring", &mod_)?;

    Ok(())
}
