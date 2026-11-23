//! Turns an [`Encoded`] module matrix into millimetre-space geometry.
//!
//! Dark modules and bars are merged into closed outlines with holes, so
//! the result works both as a fill and as a contour. 1D symbols can carry
//! human-readable text below the bars; inverted symbols turn the quiet
//! zone into a dark border.

use crate::geo::algo::topology::normalize_winding_orders;
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::get_polygons_union;
use crate::geo::shape::text::{text_to_geometry, FontConfig};
use crate::geo::types::{Point, Polygon};

use super::{min_quiet_zone, BarcodeError, Encoded, Symbology};

const TEXT_HEIGHT_MODULES: f64 = 8.0;
const TEXT_GAP_MODULES: f64 = 1.0;
const TEXT_TOLERANCE_MM: f64 = 0.01;
const EAN_FIRST_DIGIT_X: f64 = -4.0;
const EAN_LEFT_HALF_CENTRE: f64 = 24.0;
const EAN_RIGHT_HALF_CENTRE: f64 = 71.0;

/// How a symbol is drawn.
///
/// `quiet_zone` is in modules and only drawn when `invert` is set; when
/// it is `None`, the symbology's minimum is used. `bar_height_mm` and
/// `show_text` apply to 1D codes only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarcodeStyle {
    /// Width of the narrowest bar, or one QR module, in millimetres.
    pub module_mm: f64,
    /// Height of the bars, in millimetres.
    pub bar_height_mm: f64,
    /// Quiet zone in modules; `None` uses the symbology minimum.
    pub quiet_zone: Option<u32>,
    /// Draw the human-readable text under a 1D code.
    pub show_text: bool,
    /// Draw a light code on a dark quiet-zone border.
    pub invert: bool,
}

impl Default for BarcodeStyle {
    fn default() -> Self {
        BarcodeStyle {
            module_mm: 0.5,
            bar_height_mm: 15.0,
            quiet_zone: None,
            show_text: false,
            invert: false,
        }
    }
}

/// Builds the dark areas of a symbol as closed contours, in millimetres.
pub fn to_geometry(
    encoded: &Encoded,
    style: &BarcodeStyle,
) -> Result<Geometry, BarcodeError> {
    if !style.module_mm.is_finite() || style.module_mm <= 0.0 {
        return Err(BarcodeError::InvalidModuleSize);
    }
    if !style.bar_height_mm.is_finite() || style.bar_height_mm < 0.0 {
        return Err(BarcodeError::InvalidBarHeight);
    }
    if style.quiet_zone.is_some_and(|zone| zone > 1000) {
        return Err(BarcodeError::InvalidQuietZone);
    }

    let (mut polygons, symbol_height) = dark_polygons(encoded, style);
    if style.show_text && encoded.symbology != Symbology::Qr {
        polygons.extend(human_readable(encoded, style));
    }

    if style.invert {
        let zone = style
            .quiet_zone
            .unwrap_or_else(|| min_quiet_zone(encoded.symbology));
        let border = f64::from(zone) * style.module_mm;
        let width = encoded.width() as f64 * style.module_mm;
        for polygon in &mut polygons {
            for point in polygon {
                point.x += border;
                point.y += border;
            }
        }
        polygons.push(rect(
            0.0,
            0.0,
            width + 2.0 * border,
            symbol_height + 2.0 * border,
        ));
    }

    Ok(polygons_to_geometry(&polygons))
}

/// Dark module rectangles, unioned, plus the symbol height in mm.
fn dark_polygons(
    encoded: &Encoded,
    style: &BarcodeStyle,
) -> (Vec<Polygon>, f64) {
    let module = style.module_mm;
    if encoded.symbology == Symbology::Qr {
        let height = encoded.height() as f64 * module;
        let mut rects = Vec::new();
        for (row_index, row) in encoded.modules.iter().enumerate() {
            for (start, width) in runs(row) {
                rects.push(rect(
                    start as f64 * module,
                    height - (row_index + 1) as f64 * module,
                    width as f64 * module,
                    module,
                ));
            }
        }
        return (get_polygons_union(&rects), height);
    }

    let row = encoded.modules.first().map_or(&[][..], Vec::as_slice);
    let text_space = if style.show_text {
        (TEXT_HEIGHT_MODULES + TEXT_GAP_MODULES) * module
    } else {
        0.0
    };
    let mut rects = Vec::new();
    for (start, width) in runs(row) {
        let guard = encoded.is_guard(start) && text_space > 0.0;
        let extra = if guard { text_space / 2.0 } else { 0.0 };
        rects.push(rect(
            start as f64 * module,
            text_space - extra,
            width as f64 * module,
            style.bar_height_mm + extra,
        ));
    }
    (get_polygons_union(&rects), text_space + style.bar_height_mm)
}

/// Human-readable text below a 1D symbol.
fn human_readable(encoded: &Encoded, style: &BarcodeStyle) -> Vec<Polygon> {
    let module = style.module_mm;
    let top = (TEXT_HEIGHT_MODULES - TEXT_GAP_MODULES) * module;
    let height = (TEXT_HEIGHT_MODULES - 2.0 * TEXT_GAP_MODULES) * module;

    if encoded.symbology != Symbology::Ean13 {
        return text_polygons(
            &encoded.text,
            encoded.width() as f64 * module / 2.0,
            top,
            height,
        );
    }

    let text = encoded.text.as_str();
    let groups = [
        (text.get(0..1).unwrap_or(""), EAN_FIRST_DIGIT_X),
        (text.get(1..7).unwrap_or(""), EAN_LEFT_HALF_CENTRE),
        (text.get(7..13).unwrap_or(""), EAN_RIGHT_HALF_CENTRE),
    ];
    let mut polygons = Vec::new();
    for (digits, centre) in groups {
        polygons.extend(text_polygons(digits, centre * module, top, height));
    }
    polygons
}

/// Glyph outlines for `text`, centred at `centre_x` and fitted to
/// `height` millimetres.
fn text_polygons(
    text: &str,
    centre_x: f64,
    top: f64,
    height: f64,
) -> Vec<Polygon> {
    if text.is_empty() || height <= 0.0 {
        return Vec::new();
    }
    let Some(mut geometry) =
        text_to_geometry(text, &FontConfig::new("sans-serif", 10.0))
    else {
        return Vec::new();
    };
    let bounds = geometry.rect();
    let glyph_height = bounds.max.y - bounds.min.y;
    if glyph_height <= 0.0 {
        return Vec::new();
    }

    let scale = height / glyph_height;
    let centre = (bounds.min.x + bounds.max.x) / 2.0;
    let dx = centre_x - centre * scale;
    let dy = top - bounds.max.y * scale;
    geometry.transform_2d(scale, 0.0, 0.0, scale, dx, dy);

    geometry
        .to_polygons(TEXT_TOLERANCE_MM / scale)
        .into_iter()
        .filter(|polygon| polygon.len() >= 3)
        .collect()
}

/// Horizontal runs of dark modules in a row.
fn runs(row: &[bool]) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    let mut start = None;
    for (column, dark) in row.iter().copied().chain([false]).enumerate() {
        if dark && start.is_none() {
            start = Some(column);
        } else if !dark {
            if let Some(begin) = start.take() {
                result.push((begin, column - begin));
            }
        }
    }
    result
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Polygon {
    vec![
        Point::new(x, y),
        Point::new(x + width, y),
        Point::new(x + width, y + height),
        Point::new(x, y + height),
    ]
}

/// Builds a geometry from closed polygons and normalizes nesting.
fn polygons_to_geometry(polygons: &[Polygon]) -> Geometry {
    let contours: Vec<Geometry> = polygons
        .iter()
        .filter(|polygon| polygon.len() >= 3)
        .map(|polygon| {
            let mut geometry = Geometry::new();
            if let Some(first) = polygon.first() {
                geometry.move_to(first.x, first.y, 0.0);
            }
            for point in polygon.iter().skip(1) {
                geometry.line_to(point.x, point.y, 0.0);
            }
            geometry.close_path();
            geometry
        })
        .collect();
    if contours.is_empty() {
        return Geometry::new();
    }

    let refs: Vec<&Geometry> = contours.iter().collect();
    let mut result = Geometry::new();
    for contour in normalize_winding_orders(&refs) {
        result.extend(&contour);
    }
    result
}
