//! The document-level job counter.
//!
//! A counter is a small immutable record: the value the next job starts
//! with, the value `reset` returns to, the increment per copy and per
//! job, and an optional end value after which the range wraps back to
//! `start`.
//!
//! The arithmetic mirrors Python's integer semantics, including floor
//! division for negative offsets, so a copy placed before the current
//! one wraps backwards correctly.

use serde_json::{json, Value};

/// An immutable job counter configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CounterSettings {
    /// The value the next job starts with.
    pub current: i64,
    /// The value a reset returns to and a wrapped range restarts at.
    pub start: i64,
    /// The increment per copy and per job.
    pub step: i64,
    /// Optional last value; values past it wrap back to `start`.
    pub end: Option<i64>,
    /// Whether a finished job advances the counter.
    pub auto_advance: bool,
    /// Whether exporting G-code advances the counter.
    pub advance_on_export: bool,
}

impl Default for CounterSettings {
    fn default() -> Self {
        CounterSettings {
            current: 1,
            start: 1,
            step: 1,
            end: None,
            auto_advance: true,
            advance_on_export: false,
        }
    }
}

impl CounterSettings {
    /// Creates a counter with the given configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        current: i64,
        start: i64,
        step: i64,
        end: Option<i64>,
        auto_advance: bool,
        advance_on_export: bool,
    ) -> Self {
        CounterSettings {
            current,
            start,
            step,
            end,
            auto_advance,
            advance_on_export,
        }
    }

    /// Number of values in `start..=end`, or `None` when the range is
    /// unbounded or empty.
    fn range_length(&self) -> Option<i64> {
        if self.step <= 0 {
            return None;
        }
        let end = self.end?;
        if end < self.start {
            return None;
        }
        Some((end - self.start) / self.step + 1)
    }

    /// The value shown by a copy that is `offset` copies further.
    ///
    /// Negative offsets wrap backwards using floor division, matching
    /// Python's `//` and `%` for positive steps.
    pub fn value_at(&self, offset: i64) -> i64 {
        let value = self
            .current
            .saturating_add(offset.saturating_mul(self.step));
        let Some(length) = self.range_length() else {
            return value;
        };
        let index = value.saturating_sub(self.start).div_euclid(self.step);
        self.start
            .saturating_add(index.rem_euclid(length).saturating_mul(self.step))
    }

    /// The values of `copies` consecutive copies.
    pub fn values_for(&self, copies: usize) -> Vec<i64> {
        (0..copies)
            .map(|index| self.value_at(index as i64))
            .collect()
    }

    /// A counter moved on (or back, if negative) by `copies`.
    pub fn advanced(&self, copies: i64) -> Self {
        CounterSettings {
            current: self.value_at(copies),
            ..self.clone()
        }
    }

    /// A counter whose current value is back at `start`.
    pub fn reset(&self) -> Self {
        CounterSettings {
            current: self.start,
            ..self.clone()
        }
    }

    /// Serializes the counter to the project dictionary shape.
    pub fn to_json(&self) -> Value {
        json!({
            "current": self.current,
            "start": self.start,
            "step": self.step,
            "end": self.end,
            "auto_advance": self.auto_advance,
            "advance_on_export": self.advance_on_export,
        })
    }

    /// Deserializes a counter, ignoring missing keys and dropping an
    /// end value below `start`.
    pub fn from_json(data: &Value) -> Self {
        let default = CounterSettings::default();
        let start = data
            .get("start")
            .and_then(Value::as_i64)
            .unwrap_or(default.start);
        let end = data
            .get("end")
            .and_then(Value::as_i64)
            .filter(|end| *end >= start);
        CounterSettings {
            current: data
                .get("current")
                .and_then(Value::as_i64)
                .unwrap_or(default.current),
            start,
            step: data
                .get("step")
                .and_then(Value::as_i64)
                .unwrap_or(default.step),
            end,
            auto_advance: data
                .get("auto_advance")
                .and_then(Value::as_bool)
                .unwrap_or(default.auto_advance),
            advance_on_export: data
                .get("advance_on_export")
                .and_then(Value::as_bool)
                .unwrap_or(default.advance_on_export),
        }
    }
}
