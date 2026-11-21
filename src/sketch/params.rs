//! Named parameters and safe math-expression evaluation.
//!
//! Mirrors the sketcher's reference `ParameterContext`: expressions may
//! reference earlier or later parameters, parameters are resolved in
//! multiple passes, and unresolved expressions fall back to their
//! default value instead of aborting a solve.
//!
//! Expressions are evaluated with [`nanoexpr`], a small, wasm-friendly
//! math-expression parser. Both `^` and Python-style `**` are accepted as
//! power operators.

use std::collections::BTreeMap;

use nanoexpr::{Context, ErrorKind, Functions, Math};
use serde_json::{Map, Value};

/// Error produced by an invalid or unresolved expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionError {
    message: String,
}

impl ExpressionError {
    fn new(message: impl Into<String>) -> Self {
        ExpressionError {
            message: message.into(),
        }
    }

    /// Human-readable error text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ExpressionError {}

/// A resolved parameter value.
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterValue {
    /// Numeric value usable in expressions.
    Number(f64),
    /// Text value usable in templates but not in math expressions.
    Text(String),
}

impl ParameterValue {
    fn as_number(&self) -> Option<f64> {
        match self {
            ParameterValue::Number(value) => Some(*value),
            ParameterValue::Text(_) => None,
        }
    }
}

/// Manages named parameters and evaluates string expressions.
#[derive(Clone, Debug, Default)]
pub struct ParameterContext {
    expressions: BTreeMap<String, String>,
    cache: BTreeMap<String, ParameterValue>,
    unresolved: BTreeMap<String, String>,
    dirty: bool,
}

impl ParameterContext {
    /// Creates an empty context.
    pub fn new() -> Self {
        ParameterContext {
            expressions: BTreeMap::new(),
            cache: BTreeMap::new(),
            unresolved: BTreeMap::new(),
            dirty: false,
        }
    }

    /// Sets a parameter from a number.
    pub fn set_value(&mut self, name: &str, value: f64) {
        self.set_expression(name, &format_number(value));
    }

    /// Sets a parameter from an expression string.
    pub fn set_expression(&mut self, name: &str, value: &str) {
        self.expressions.insert(name.to_string(), value.to_string());
        self.dirty = true;
    }

    /// Returns the evaluated value of a parameter.
    pub fn get(&mut self, name: &str, default: f64) -> f64 {
        if self.dirty {
            self.evaluate_all(None);
        }
        self.cache
            .get(name)
            .and_then(ParameterValue::as_number)
            .unwrap_or(default)
    }

    /// Returns all evaluated values.
    pub fn get_all_values(&mut self) -> BTreeMap<String, ParameterValue> {
        if self.dirty {
            self.evaluate_all(None);
        }
        self.cache.clone()
    }

    /// Evaluates an arbitrary expression with the current context.
    pub fn evaluate(
        &mut self,
        expression: &str,
    ) -> Result<f64, ExpressionError> {
        if self.dirty {
            self.evaluate_all(None);
        }
        let expression = expression.trim();
        if expression.is_empty() {
            return Ok(0.0);
        }
        if let Some(value) = self
            .cache
            .get(expression)
            .and_then(ParameterValue::as_number)
        {
            return Ok(value);
        }
        evaluate_with_cache(expression, &self.cache)
    }

    /// Resolves every expression, optionally seeded with initial values.
    pub fn evaluate_all(
        &mut self,
        initial_values: Option<BTreeMap<String, ParameterValue>>,
    ) {
        let mut cache: BTreeMap<String, ParameterValue> =
            initial_values.unwrap_or_default();
        let mut errors: BTreeMap<String, String> = BTreeMap::new();
        let max_passes = self.expressions.len() + 1;

        for _ in 0..max_passes {
            let mut progress = false;
            for (name, expression) in self.expressions.iter() {
                if cache.contains_key(name) {
                    continue;
                }
                match evaluate_with_cache(expression, &cache) {
                    Ok(value) => {
                        cache.insert(
                            name.clone(),
                            ParameterValue::Number(value),
                        );
                        errors.remove(name);
                        progress = true;
                    }
                    Err(error) => {
                        errors
                            .insert(name.clone(), error.message().to_string());
                    }
                }
            }
            if !progress {
                break;
            }
        }

        self.unresolved = errors;
        self.cache = cache;
        self.dirty = false;
    }

    /// Expressions that remained unresolved during the last evaluation.
    pub fn unresolved_errors(&self) -> &BTreeMap<String, String> {
        &self.unresolved
    }

    /// Clears cached values so the next access re-evaluates everything.
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// Serializes the context to the reference dictionary shape.
    pub fn to_json(&self) -> Value {
        let expressions: Map<String, Value> = self
            .expressions
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect();
        serde_json::json!({ "expressions": expressions })
    }

    /// Deserializes a context from the reference dictionary shape.
    pub fn from_json(data: &Value) -> Option<Self> {
        let expressions = data.get("expressions")?.as_object()?;
        let mut context = ParameterContext::new();
        for (name, value) in expressions {
            let value = value.as_str()?;
            context.expressions.insert(name.clone(), value.to_string());
        }
        context.dirty = true;
        Some(context)
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e16 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

struct CachedValues<'a> {
    values: &'a BTreeMap<String, ParameterValue>,
}

impl Context<f64> for CachedValues<'_> {
    fn variable(&self, name: &str) -> Option<f64> {
        self.values.get(name).and_then(ParameterValue::as_number)
    }
}

fn evaluate_with_cache(
    expression: &str,
    cache: &BTreeMap<String, ParameterValue>,
) -> Result<f64, ExpressionError> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Ok(0.0);
    }

    let variables = CachedValues { values: cache };
    let context = (&variables, (Functions(extra_functions), Math));

    nanoexpr::evaluate_with(expression, &context)
        .map_err(|error| ExpressionError::new(error.to_string()))
}

fn extra_functions(
    name: &str,
    arguments: &[f64],
) -> Option<Result<f64, ErrorKind>> {
    if !matches!(
        name,
        "log"
            | "exp2"
            | "fract"
            | "fabs"
            | "degrees"
            | "radians"
            | "fmod"
            | "copysign"
            | "signum"
            | "isfinite"
            | "isinf"
            | "isnan"
    ) {
        return None;
    }

    let value = match (name, arguments) {
        ("log", [value]) => value.ln(),
        ("exp2", [value]) => value.exp2(),
        ("fract", [value]) => value.fract(),
        ("fabs", [value]) => value.abs(),
        ("degrees", [value]) => value.to_degrees(),
        ("radians", [value]) => value.to_radians(),
        ("fmod", [left, right]) => left % right,
        ("copysign", [left, right]) => left.copysign(*right),
        ("signum", [value]) => value.signum(),
        ("isfinite", [value]) => value.is_finite() as i32 as f64,
        ("isinf", [value]) => value.is_infinite() as i32 as f64,
        ("isnan", [value]) => value.is_nan() as i32 as f64,
        _ => return Some(Err(ErrorKind::ArgumentCount)),
    };
    Some(Ok(value))
}
