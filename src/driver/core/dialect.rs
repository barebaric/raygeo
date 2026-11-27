//! Dialect command templates.
//!
//! Dialects remain data owned by Rayforge: callers pass in resolved
//! command templates (Python `str.format` strings) and this crate
//! formats them.  The formatting subset supports `{name}` and
//! `{name:.Nf}` — everything Rayforge's dialects use for interactive
//! commands.

use std::collections::HashMap;

/// The subset of dialect templates used for interactive commands.
#[derive(Debug, Clone)]
pub struct Dialect {
    pub home_all: String,
    pub home_axis: String,
    pub move_to: String,
    pub jog: String,
    pub clear_alarm: String,
    pub laser_on: String,
    pub laser_off: String,
    pub focus_laser_on: String,
    pub tool_change: String,
    pub set_wcs_offset: String,
    pub probe_cycle: String,
    /// Resolved safety-off commands (laser off, air assist off, …),
    /// sent after a cancel so no persistent output stays energized.
    pub safety_off_commands: Vec<String>,
    /// Optional additional emergency stop command, appended to the
    /// safety shutdown when `cancel(emergency=True)`.
    pub emergency_stop: Option<String>,
}

impl Default for Dialect {
    /// The default template set is GRBL-flavored (callers that use
    /// another firmware pass their own dialect data).
    fn default() -> Self {
        Self {
            home_all: "$H".to_string(),
            home_axis: "$H{axis_letter}".to_string(),
            move_to: "$J=G90 G21 F{speed} X{x} Y{y}".to_string(),
            jog: "$J=G91 G21 F{speed}".to_string(),
            clear_alarm: "$X".to_string(),
            laser_on: "M4 S{power:.0f}".to_string(),
            laser_off: "M5".to_string(),
            focus_laser_on: "M3 S{power:.0f}".to_string(),
            tool_change: "T{tool_number}".to_string(),
            set_wcs_offset: "G10 L2 P{p_num} X{x} Y{y} Z{z}".to_string(),
            probe_cycle: "G38.2 {axis_letter}{max_travel} F{feed_rate}"
                .to_string(),
            safety_off_commands: vec!["M5".to_string()],
            emergency_stop: None,
        }
    }
}

impl Dialect {
    /// Build from a template dict keyed by the field names.  Missing
    /// keys fall back to the default template.
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let mut dialect = Self::default();
        let get = |key: &str| map.get(key).cloned();
        if let Some(v) = get("home_all") {
            dialect.home_all = v;
        }
        if let Some(v) = get("home_axis") {
            dialect.home_axis = v;
        }
        if let Some(v) = get("move_to") {
            dialect.move_to = v;
        }
        if let Some(v) = get("jog") {
            dialect.jog = v;
        }
        if let Some(v) = get("clear_alarm") {
            dialect.clear_alarm = v;
        }
        if let Some(v) = get("laser_on") {
            dialect.laser_on = v;
        }
        if let Some(v) = get("laser_off") {
            dialect.laser_off = v;
        }
        if let Some(v) = get("focus_laser_on") {
            dialect.focus_laser_on = v;
        }
        if let Some(v) = get("tool_change") {
            dialect.tool_change = v;
        }
        if let Some(v) = get("set_wcs_offset") {
            dialect.set_wcs_offset = v;
        }
        if let Some(v) = get("probe_cycle") {
            dialect.probe_cycle = v;
        }
        if let Some(v) = get("emergency_stop") {
            dialect.emergency_stop = Some(v);
        }
        dialect
    }

    /// Format the `set_wcs_offset` template, dropping the Z word
    /// when `z` is `None` (machines without a Z axis).
    ///
    /// The templates are caller-supplied data; a malformed
    /// `set_wcs_offset` template (unknown placeholder or bad
    /// precision spec) is an error, not a panic.
    pub fn format_wcs_offset(
        &self,
        p_num: i64,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<String, String> {
        let mut template = self.set_wcs_offset.clone();
        if z.is_none() {
            template = template
                .replace(" Z{z}", "")
                .replace("Z{z}", " ")
                .trim()
                .to_string();
        }
        format_template(
            &template,
            &[
                ("p_num", Arg::Int(p_num)),
                ("x", Arg::Length(x)),
                ("y", Arg::Length(y)),
                ("z", Arg::Length(z.unwrap_or(0.0))),
            ],
        )
    }
}

/// A template formatting argument.
#[derive(Debug, Clone)]
pub enum Arg {
    /// Renders like a Python float: `10.5`, `10.0`.
    Length(f64),
    /// Renders like a Python value passed through
    /// `_to_machine_speed`: whole numbers render as integers
    /// (`1500`), fractional values as floats.
    Speed(f64),
    /// Renders like a Python int.
    Int(i64),
    /// Renders as-is.
    Str(String),
}

/// Format a Python-integral float the way Python's `str()` does
/// (`10.0` stays `10.0`).
pub(crate) fn python_float_repr(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e16 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

impl Arg {
    fn render(&self, spec: Option<&str>) -> String {
        let Some(spec) = spec else {
            return match self {
                Arg::Length(v) => python_float_repr(*v),
                Arg::Speed(v) => {
                    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e16 {
                        format!("{}", *v as i64)
                    } else {
                        format!("{v}")
                    }
                }
                Arg::Int(v) => format!("{v}"),
                Arg::Str(v) => v.clone(),
            };
        };
        let num = match self {
            Arg::Length(v) | Arg::Speed(v) => *v,
            Arg::Int(v) => *v as f64,
            Arg::Str(_) => {
                return String::new();
            }
        };
        if let Some(digits) =
            spec.strip_prefix('.').and_then(|s| s.strip_suffix('f'))
        {
            match digits.parse::<usize>() {
                Ok(n) => format!("{num:.n$}"),
                Err(_) => format!("{num}"),
            }
        } else {
            format!("{num}")
        }
    }
}

/// Format `template` with `{name}` / `{name:.Nf}` placeholders.
pub fn format_template(
    template: &str,
    args: &[(&str, Arg)],
) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            return Err(format!(
                "unterminated placeholder in template {template:?}"
            ));
        };
        let placeholder = &after[..end];
        let (name, spec) = match placeholder.split_once(':') {
            Some((n, s)) => (n, Some(s)),
            None => (placeholder, None),
        };
        let Some((_, arg)) = args.iter().find(|(n, _)| *n == name) else {
            return Err(format!("no value for placeholder {{{name}}}"));
        };
        out.push_str(&arg.render(spec));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
