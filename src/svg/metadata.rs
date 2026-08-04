use crate::error::{RaygeoError, RaygeoResult};
use crate::svg::length::parse_svg_length;

// ── SVG Metadata extraction ───────────────────────────────────────

/// Metadata extracted from the root `<svg>` element.
#[derive(Debug, Clone)]
pub struct SvgMetadata {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub width_unit: String,
    pub height_unit: String,
    pub viewbox: Option<(f64, f64, f64, f64)>,
}

/// Extract metadata (width, height, units, viewBox) from an SVG string.
pub fn extract_svg_metadata(svg_str: &str) -> RaygeoResult<SvgMetadata> {
    let doc = roxmltree::Document::parse(svg_str)
        .map_err(|e| RaygeoError::SvgParseError(format!("{e}")))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(RaygeoError::SvgParseError(
            "root element is not <svg>".into(),
        ));
    }

    let (width, width_unit) = if let Some(w) = root.attribute("width") {
        let pl = parse_svg_length(w)?;
        (Some(pl.value), pl.unit)
    } else {
        (None, "px".into())
    };

    let (height, height_unit) = if let Some(h) = root.attribute("height") {
        let pl = parse_svg_length(h)?;
        (Some(pl.value), pl.unit)
    } else {
        (None, "px".into())
    };

    let viewbox = root.attribute("viewBox").and_then(|vb| {
        let parts: Vec<f64> = vb
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() == 4 {
            Some((parts[0], parts[1], parts[2], parts[3]))
        } else {
            None
        }
    });

    Ok(SvgMetadata {
        width,
        height,
        width_unit,
        height_unit,
        viewbox,
    })
}
