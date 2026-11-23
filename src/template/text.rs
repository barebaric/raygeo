//! Per-placeholder text resolution and dependency stamps.
//!
//! A template is resolved one placeholder at a time. Each resolved
//! placeholder is stored with the values it read, so a later change to
//! an unrelated variable does not re-evaluate it. This keeps volatile
//! functions such as `uuid8()` stable when only a counter changes.
//!
//! The host owns the variables and functions through
//! [`TemplateContext`]; this module owns the template grammar, the
//! stamps and the reuse rules.

use std::collections::BTreeSet;

use crate::template::expr::{self, TemplateContext, Value};
use crate::template::placeholders::{names_in_expression, referenced_names};

/// A resolved placeholder and the values it was resolved with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextPart {
    /// The full placeholder text, including braces and format spec.
    pub placeholder: String,
    /// The rendered text.
    pub resolved: String,
    /// Pairs of variable name and stamp representation.
    pub stamp: Vec<(String, String)>,
}

/// A resolved text box and its dependency information.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextEntry {
    /// The raw template.
    pub source: String,
    /// The rendered text, or `None` for an empty template.
    pub resolved: Option<String>,
    /// The values the whole template read.
    pub stamp: Vec<(String, String)>,
    /// The resolved placeholders, by occurrence.
    pub parts: Vec<TextPart>,
}

/// The result of resolving one template.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextResolution {
    /// The rendered text, or `None` for an empty template.
    pub resolved: Option<String>,
    /// The cache entry to store for the next call.
    pub entry: TextEntry,
    /// Whether a newly evaluated placeholder used a host function.
    pub volatile: bool,
}

/// Resolves *template*, reusing unchanged placeholders from *previous*.
pub fn resolve_template(
    template: &str,
    context: &dyn TemplateContext,
    function_names: &BTreeSet<String>,
    previous: Option<&TextEntry>,
) -> TextResolution {
    if template.is_empty() {
        return TextResolution {
            resolved: None,
            entry: TextEntry::default(),
            volatile: false,
        };
    }

    let full_stamp = stamp_for_template(template, context, function_names);
    let mut output = String::new();
    let mut parts = Vec::new();
    let mut position = 0;
    let mut volatile = false;

    for (index, (start, end, inner)) in
        placeholders(template).into_iter().enumerate()
    {
        output.push_str(&template[position..start]);
        let placeholder = &template[start..=end];
        let (expression, format_spec) = match inner.split_once(':') {
            Some((expression, spec)) => (expression.trim(), spec),
            None => (inner.trim(), ""),
        };
        let stamp = stamp_for_expression(expression, context, function_names);
        let previous_part = previous.and_then(|entry| entry.parts.get(index));
        if let Some(previous_part) = previous_part {
            if previous_part.placeholder == placeholder
                && is_current(&previous_part.stamp, &stamp)
            {
                output.push_str(&previous_part.resolved);
                parts.push(previous_part.clone());
                position = end + 1;
                continue;
            }
        }

        let resolved_part = match expr::evaluate(expression, context) {
            Ok(value) => expr::format_value(&value, format_spec),
            Err(_) => placeholder.to_string(),
        };
        if names_in_expression(expression)
            .iter()
            .any(|name| function_names.contains(name))
        {
            volatile = true;
        }
        parts.push(TextPart {
            placeholder: placeholder.to_string(),
            resolved: resolved_part.clone(),
            stamp: stamp.clone(),
        });
        output.push_str(&resolved_part);
        position = end + 1;
    }
    output.push_str(&template[position..]);

    let resolved = Some(output);
    TextResolution {
        resolved: resolved.clone(),
        entry: TextEntry {
            source: template.to_string(),
            resolved,
            stamp: full_stamp,
            parts,
        },
        volatile,
    }
}

/// The values a template reads, as `(name, repr)` pairs.
pub fn stamp_for_template(
    template: &str,
    context: &dyn TemplateContext,
    function_names: &BTreeSet<String>,
) -> Vec<(String, String)> {
    stamp_for_names(&referenced_names(template), context, function_names)
}

fn stamp_for_expression(
    expression: &str,
    context: &dyn TemplateContext,
    function_names: &BTreeSet<String>,
) -> Vec<(String, String)> {
    stamp_for_names(&names_in_expression(expression), context, function_names)
}

fn stamp_for_names(
    names: &BTreeSet<String>,
    context: &dyn TemplateContext,
    function_names: &BTreeSet<String>,
) -> Vec<(String, String)> {
    names
        .iter()
        .filter(|name| !function_names.contains(*name))
        .filter_map(|name| {
            context
                .variable(name)
                .map(|value| (name.clone(), value.stamp_repr()))
        })
        .collect()
}

fn is_current(
    previous: &[(String, String)],
    current: &[(String, String)],
) -> bool {
    current.iter().all(|pair| previous.contains(pair))
}

/// Returns `(start, end, inner)` for every placeholder.
///
/// `start` is the `{`, `end` the `}`; nested braces are skipped the same
/// way the rest of the template code treats them.
fn placeholders(template: &str) -> Vec<(usize, usize, &str)> {
    let mut result = Vec::new();
    let mut position = 0;
    while let Some(open_relative) = template[position..].find('{') {
        let start = position + open_relative;
        let rest = &template[start + 1..];
        let Some(close_relative) = rest.find(['{', '}']) else {
            break;
        };
        if rest[close_relative..].starts_with('{') {
            position = start + 1;
            continue;
        }
        let end = start + 1 + close_relative;
        result.push((start, end, &rest[..close_relative]));
        position = end + 1;
    }
    result
}

/// Resolves an expression and returns the value for hosts that need it.
pub fn evaluate_expression(
    expression: &str,
    context: &dyn TemplateContext,
) -> Result<Value, expr::EvalError> {
    expr::evaluate(expression, context)
}
