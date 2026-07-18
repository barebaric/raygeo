//! Generic template and string-formatting utilities.
//!
//! This module contains the template engine shared across
//! language-specific encoders.
//!
//! Supported features:
//!
//! * Named-key substitution: `{x}`, `{s_command}`, `{tool_number}`, …
//! * Path-style variable resolution: `{machine.name}`,
//!   `{job.extents[0]}`, `{wcs_offset[2]}`. Any placeholder whose
//!   name contains a `.` or `[` is looked up by exact full-path match
//!   in the caller-supplied `path_vars` map. Unresolved placeholders
//!   are left verbatim.
//! * Python-style format specifiers: `{name:.Nf}` — parsed as
//!   Rust's `{:.Nf}`-style format.

use std::collections::HashMap;

/// A small bag of named substitution values for a single template
/// line. Built per command by the encoder and passed to
/// [`render_named`].
#[derive(Default, Clone)]
pub struct NamedVars {
    pub pairs: Vec<(&'static str, String)>,
}

impl NamedVars {
    pub fn set(&mut self, key: &'static str, value: String) {
        if let Some(slot) = self.pairs.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.pairs.push((key, value));
        }
    }

    pub fn set_str(&mut self, key: &'static str, value: &str) {
        self.set(key, value.to_string());
    }

    pub fn set_num(&mut self, key: &'static str, value: f64) {
        self.set(key, value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find_map(|(k, v)| (*k == key).then_some(v.as_str()))
    }
}

/// Replace `{name}` placeholders in `template` using `vars` when the
/// name is one of the known keys. Unknown placeholders are left
/// verbatim so they can be resolved by `resolve_path_vars` if they are
/// path-style variables.
///
/// A placeholder is rendered as a named substitution only if the name
/// contains no `.` or `[`; otherwise it is left untouched and deferred
/// to the path resolver.
///
/// Format specifiers (`{name:spec}`) are supported: the looked-up
/// string value is parsed as `f64` and formatted with the spec using
/// Rust's `{:.Nf}`-style format. Currently `:spec` is parsed as `.Nf`
/// where N is an unsigned integer.
pub fn render_named(template: &str, vars: &NamedVars) -> String {
    if !template.contains('{') {
        return template.to_string();
    }

    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        rest = &rest[open..];

        let Some(close_rel) = rest[1..].find('}') else {
            out.push_str(rest);
            return out;
        };
        let inner = &rest[1..1 + close_rel];

        // Split format specifier first so the path-check only sees
        // the variable name (not the spec which may contain dots).
        let (name, fmt_spec) = match inner.split_once(':') {
            Some((n, s)) => (n, Some(s)),
            None => (inner, None),
        };

        let is_path = name.contains('.') || name.contains('[');
        if is_path {
            out.push('{');
            out.push_str(inner);
            out.push('}');
        } else if let Some(value) = vars.get(name) {
            if let Some(spec) = fmt_spec {
                out.push_str(&apply_format_spec(value, spec));
            } else {
                out.push_str(value);
            }
        } else {
            out.push('{');
            out.push_str(inner);
            out.push('}');
        }
        rest = &rest[2 + close_rel..];
    }

    out.push_str(rest);
    out
}

/// Apply a Python-style format spec (`.Nf`, `.Ns`, `.N`, `d`, etc.)
/// to a value that was looked up as a string. Currently supports the
/// most common specs used in G-code dialects: `.Nf` (float precision)
/// and `.N` (general precision). The value is parsed as f64 first.
pub fn apply_format_spec(value: &str, spec: &str) -> String {
    let parsed: f64 = match value.parse() {
        Ok(v) => v,
        Err(_) => return value.to_string(),
    };
    if let Some(rest) = spec.strip_prefix('.') {
        if let Some(precision_str) = rest.strip_suffix('f').or_else(|| {
            if rest.chars().all(|c| c.is_ascii_digit()) {
                Some(rest)
            } else {
                None
            }
        }) {
            if let Ok(prec) = precision_str.parse::<usize>() {
                return format!("{:.*}", prec, parsed);
            }
        }
    }
    if spec == "d" {
        return format!("{}", parsed as i64);
    }
    format!("{}", parsed)
}

/// Resolve path-style placeholders (`{machine.name}`,
/// `{job.extents[0]}`, etc.) using a flat dictionary provided by the
/// caller. Placeholders are looked up by exact full-path match only.
/// Unresolved placeholders are left verbatim.
pub fn resolve_path_vars(
    line: &str,
    path_vars: &HashMap<String, String>,
) -> String {
    if !line.contains('{') || path_vars.is_empty() {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len());
    let mut rest = line;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        rest = &rest[open..];

        let Some(close_rel) = rest[1..].find('}') else {
            out.push_str(rest);
            return out;
        };
        let inner = &rest[1..1 + close_rel];

        if inner.contains('.') || inner.contains('[') {
            if let Some(value) = path_vars.get(inner) {
                out.push_str(value);
            } else {
                out.push('{');
                out.push_str(inner);
                out.push('}');
            }
        } else {
            out.push('{');
            out.push_str(inner);
            out.push('}');
        }
        rest = &rest[2 + close_rel..];
    }

    out.push_str(rest);
    out
}

/// Match the regex `^\s*@include\((.*?)\)\s*$`. Strips surrounding
/// whitespace from both the directive and the macro name.
pub fn parse_include_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("@include(")?;
    let rest = rest.trim_end();
    let close = rest.strip_suffix(')')?;
    Some(close.trim().to_string())
}
