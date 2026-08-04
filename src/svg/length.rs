use crate::error::{RaygeoError, RaygeoResult};

// ── SVG Length parsing ────────────────────────────────────────────

/// A parsed SVG length value with its unit suffix.
///
/// Supports: `mm`, `cm`, `in`, `pt`, `pc`, `px` and unitless (treated as `px`).
#[derive(Debug, Clone, PartialEq)]
pub struct SvgLength {
    pub value: f64,
    pub unit: String,
}

impl SvgLength {
    /// Convert this length to millimetres using the given DPI for `px` / unitless values.
    pub fn to_mm(&self, dpi: f64) -> f64 {
        match self.unit.as_str() {
            "mm" => self.value,
            "cm" => self.value * 10.0,
            "dm" => self.value * 100.0,
            "m" => self.value * 1000.0,
            "in" | "inch" => self.value * 25.4,
            "pt" => self.value * 25.4 / 72.0,
            "pc" => self.value * 25.4 / 6.0,
            _ => self.value * 25.4 / dpi, // px, unitless, em, ex, %
        }
    }

    /// Convert this length to pixels using the given DPI.
    pub fn to_px(&self, dpi: f64) -> f64 {
        match self.unit.as_str() {
            "mm" => self.value * dpi / 25.4,
            "cm" => self.value * dpi / 2.54,
            "dm" => self.value * dpi / 0.254,
            "m" => self.value * dpi / 0.0254,
            "in" | "inch" => self.value * dpi,
            "pt" => self.value * dpi / 72.0,
            "pc" => self.value * dpi / 6.0,
            _ => self.value, // px, unitless
        }
    }
}

/// Parse an SVG length string (e.g. `"10mm"`, `"2.5in"`, `"100"`, `"3cm"`, `"12pt"`).
///
/// Returns the numeric value and unit string. Unitless or `px` lengths are
/// returned with `unit = "px"`.
pub fn parse_svg_length(length_str: &str) -> RaygeoResult<SvgLength> {
    let s = length_str.trim();
    if s.is_empty() {
        return Ok(SvgLength {
            value: 0.0,
            unit: "px".into(),
        });
    }
    let num_end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+')
        .unwrap_or(s.len());
    if num_end == 0 {
        return Err(RaygeoError::SvgParseError(format!(
            "invalid SVG length: {length_str}"
        )));
    }
    let value: f64 = s[..num_end].parse().map_err(|_| {
        RaygeoError::SvgParseError(format!(
            "invalid SVG length value: {length_str}"
        ))
    })?;
    let unit = s[num_end..].trim().to_string();
    let unit = if unit.is_empty() { "px".into() } else { unit };
    Ok(SvgLength { value, unit })
}
