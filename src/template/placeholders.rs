//! Placeholder and variable-name scanning for text templates.
//!
//! Templates embed expressions inside braces; a format spec may follow
//! the first colon, for example `{counter:04d}`. The scanner collects
//! the names a template reads without evaluating it, so callers can
//! tell whether a cached rendering is still current.
//!
//! This is deliberately a conservative approximation of the host
//! expression grammar: it skips string literals, keywords, numeric
//! literals and attribute names, and may report a function name as a
//! variable. An extra name can only cause an extra refresh, never a
//! stale result.

use std::collections::BTreeSet;

/// Python keywords that are never variable names in an expression.
const KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "False", "finally", "for", "from",
    "global", "if", "import", "in", "is", "lambda", "None", "nonlocal", "not",
    "or", "pass", "raise", "return", "True", "try", "while", "with", "yield",
];

/// The expressions inside the `{...}` placeholders of *template*.
///
/// A format spec after the first colon is not part of the expression.
pub fn placeholder_expressions(template: &str) -> Vec<&str> {
    let mut expressions = Vec::new();
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find(['{', '}']) else {
            break;
        };
        if rest[close..].starts_with('{') {
            rest = &rest[close..];
            continue;
        }
        let inner = &rest[..close];
        let expression = inner.split_once(':').map_or(inner, |(expr, _)| expr);
        expressions.push(expression);
        rest = &rest[close + 1..];
    }

    expressions
}

/// The placeholder names of *template*, including dotted path names
/// such as `workpiece.counter`. A format spec after the first colon is
/// not part of the name.
pub fn placeholder_names(template: &str) -> BTreeSet<String> {
    placeholder_expressions(template)
        .into_iter()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

/// The variable names an expression reads.
///
/// String literals, keywords, numeric literals and attribute names are
/// ignored.
pub fn names_in_expression(expression: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut chars = expression.chars().peekable();
    let mut previous_was_dot = false;

    while let Some(ch) = chars.next() {
        if ch == '\'' || ch == '"' {
            skip_string(&mut chars, ch);
            previous_was_dot = false;
            continue;
        }
        if ch == '.' {
            previous_was_dot = true;
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let mut name = String::new();
            name.push(ch);
            while let Some(next) = chars.peek() {
                if next.is_alphanumeric() || *next == '_' {
                    name.push(*next);
                    chars.next();
                } else {
                    break;
                }
            }
            if !previous_was_dot && !KEYWORDS.contains(&name.as_str()) {
                names.insert(name);
            }
            previous_was_dot = false;
            continue;
        }
        previous_was_dot = false;
    }

    names
}

/// The variable names used inside the `{...}` placeholders of
/// *template*.
pub fn referenced_names(template: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for expression in placeholder_expressions(template) {
        names.extend(names_in_expression(expression));
    }
    names
}

/// Consumes a quoted string literal, stopping after the closing quote.
fn skip_string(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    quote: char,
) {
    let mut escaped = false;
    for ch in chars.by_ref() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return;
        }
    }
}
