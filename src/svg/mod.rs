//! SVG parsing and geometry extraction.
//!
//! Provides parsers for path data and transforms, shape conversion,
//! color resolution, and layer/color-aware geometry extraction.

pub mod arc;
pub mod color;
pub mod length;
pub mod metadata;
pub mod path;
pub mod shape;
pub mod transform;
pub(crate) mod traverse;

use std::f64::consts::PI;

use svgtypes::PathSegment;

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::geometry::Geometry;
use crate::geo::matrix::Matrix;
use crate::geo::shape::arc::get_arc_sweep;
use crate::geo::types::Command;
use crate::svg::color::ColorAttr;
use crate::svg::path::PathBuildContext;
use crate::svg::traverse::{
    collect_color_remove_ranges, traverse, traverse_by_color,
};

use std::ops::Range;

pub(crate) fn parse_coords(s: &str) -> Vec<f64> {
    let mut coords = Vec::new();
    let mut chars = s.chars().peekable();
    let mut buf = String::new();
    while let Some(&ch) = chars.peek() {
        if ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit() {
            buf.clear();
            if ch == '-' || ch == '+' {
                if let Some(c) = chars.next() {
                    buf.push(c);
                }
            }
            let mut has_dot = false;
            let mut has_exp = false;
            loop {
                match chars.peek() {
                    Some(&c) if c.is_ascii_digit() => {
                        if let Some(c) = chars.next() {
                            buf.push(c);
                        }
                    }
                    Some(&'.') if !has_dot && !has_exp => {
                        has_dot = true;
                        if let Some(c) = chars.next() {
                            buf.push(c);
                        }
                    }
                    Some(&'e') | Some(&'E') if !has_exp => {
                        has_exp = true;
                        if let Some(c) = chars.next() {
                            buf.push(c);
                        }
                        if chars.peek() == Some(&'+')
                            || chars.peek() == Some(&'-')
                        {
                            if let Some(c) = chars.next() {
                                buf.push(c);
                            }
                        }
                    }
                    _ => break,
                }
            }
            if !buf.is_empty() {
                if let Ok(v) = buf.parse::<f64>() {
                    coords.push(v);
                }
            }
        } else {
            chars.next();
        }
    }
    coords
}

/// Parse an SVG path `d` attribute into a list of geometries.
///
/// Supports M/m, L/l, H/h, V/v, C/c, S/s, Q/q, T/t, A/a, Z/z.
/// Cubic and quadratic curves are flattened to line segments.
/// Circular arcs (rx ≈ ry) are preserved as native Arc commands;
/// elliptical arcs are approximated with cubic beziers.
pub fn parse_svg_path_data(
    path_data: &str,
    transform: &Matrix,
    scale_x: f64,
    scale_y: f64,
) -> RaygeoResult<Vec<Geometry>> {
    let mut ctx = PathBuildContext::new(transform, scale_x, scale_y);
    let mut has_valid = false;
    let mut parse_error = false;

    for segment in svgtypes::PathParser::from(path_data) {
        let seg = match segment {
            Ok(s) => {
                has_valid = true;
                s
            }
            Err(_) => {
                parse_error = true;
                continue;
            }
        };

        match seg {
            PathSegment::MoveTo { abs, x, y } => ctx.handle_moveto(abs, x, y),
            PathSegment::LineTo { abs, x, y } => ctx.handle_lineto(abs, x, y),
            PathSegment::HorizontalLineTo { abs, x } => {
                ctx.handle_hline_to(abs, x)
            }
            PathSegment::VerticalLineTo { abs, y } => {
                ctx.handle_vline_to(abs, y)
            }
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => ctx.handle_cubic_to(abs, x1, y1, x2, y2, x, y),
            PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                ctx.handle_smooth_cubic_to(abs, x2, y2, x, y)
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                ctx.handle_quadratic(abs, x1, y1, x, y)
            }
            PathSegment::SmoothQuadratic { abs, x, y } => {
                ctx.handle_smooth_quadratic(abs, x, y)
            }
            PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => ctx.handle_elliptical_arc(
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            ),
            PathSegment::ClosePath { .. } => ctx.handle_close_path(),
        }
    }

    if !has_valid && parse_error {
        return Err(RaygeoError::SvgInvalidPath(
            "no valid SVG path commands found".into(),
        ));
    }

    Ok(ctx.finish())
}

/// Parse a complete SVG XML string and extract all geometries from `path`,
/// `rect`, `circle`, `ellipse`, `line`, `polyline` and `polygon` elements.
/// Hidden elements (`display="none"`, `visibility="hidden"`) are skipped.
pub fn svg_string_to_geometries(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> RaygeoResult<Vec<Geometry>> {
    let all_geometries = match roxmltree::Document::parse(svg_str) {
        Ok(doc) => {
            let mut geos = Vec::new();
            let identity = Matrix::identity();
            traverse(doc.root_element(), identity, &mut geos, scale_x, scale_y);
            geos
        }
        Err(_) => Vec::new(),
    };
    Ok(all_geometries)
}

/// Like [`svg_string_to_geometries`] but merges all subpaths into a single
/// [`Geometry`].  This avoids a Python-side loop when you only need one
/// combined path.
pub fn svg_string_to_geometry(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> RaygeoResult<Geometry> {
    let geos = svg_string_to_geometries(svg_str, scale_x, scale_y)?;
    let mut combined = Geometry::new();
    for g in geos {
        combined.extend(&g);
    }
    Ok(combined)
}

// ── Layer-aware geometry extraction ──────────────────────────────

/// Extract geometries grouped by layer (top-level `<g>` elements with an `id`
/// attribute).
///
/// Only top-level groups (immediate children of `<svg>`) with a non-empty `id`
/// are treated as layers. If no such groups exist, the returned vector is empty.
///
/// Hidden elements (`display="none"`, `visibility="hidden"`) within a layer
/// are skipped, matching the behaviour of [`svg_string_to_geometries`].
pub fn svg_string_to_geometries_by_layer(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> RaygeoResult<Vec<(String, Vec<Geometry>)>> {
    let doc = roxmltree::Document::parse(svg_str)
        .map_err(|e| RaygeoError::SvgParseError(format!("{e}")))?;
    let root = doc.root_element();

    let mut layers: Vec<(String, Vec<Geometry>)> = Vec::new();

    for child in root.children() {
        if !child.is_element() {
            continue;
        }
        if child.tag_name().name() == "g" {
            if let Some(id) = child.attribute("id") {
                if !id.is_empty() {
                    let mut geos = Vec::new();
                    traverse(
                        child,
                        Matrix::identity(),
                        &mut geos,
                        scale_x,
                        scale_y,
                    );
                    if !geos.is_empty() {
                        layers.push((id.to_string(), geos));
                    }
                }
            }
        }
    }

    Ok(layers)
}

/// Like [`svg_string_to_geometries_by_layer`] but merges each layer's
/// subpaths into a single [`Geometry`] per layer.
pub fn svg_string_to_geometry_by_layer(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> RaygeoResult<Vec<(String, Geometry)>> {
    let layers = svg_string_to_geometries_by_layer(svg_str, scale_x, scale_y)?;
    Ok(layers
        .into_iter()
        .map(|(id, geos)| {
            let mut combined = Geometry::new();
            for g in geos {
                combined.extend(&g);
            }
            (id, combined)
        })
        .collect())
}

// ── Color-aware geometry extraction ───────────────────────────────

/// Extract geometries grouped by color.
///
/// Walks the entire SVG tree (not just top-level `<g>` elements) and
/// buckets shapes by their resolved color attribute, selected with
/// [`ColorAttr`]. Colors are resolved with SVG inheritance: an element's
/// own presentation attribute (or `style` declaration) wins, otherwise
/// the nearest ancestor's value is used. `currentColor` resolves against
/// the nearest `color` attribute (defaulting to black). Shapes whose
/// chosen color attribute is `none` or unset go into a `_no_color`
/// bucket.
///
/// Bucket keys are lowercase `#rrggbb` hex strings (alpha discarded).
/// In [`ColorAttr::Any`] mode a shape whose fill differs from its stroke
/// lands in two buckets, one per color.
pub fn svg_string_to_geometries_by_color(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
    mode: ColorAttr,
) -> RaygeoResult<Vec<(String, Vec<Geometry>)>> {
    let doc = roxmltree::Document::parse(svg_str)
        .map_err(|e| RaygeoError::SvgParseError(format!("{e}")))?;
    let root = doc.root_element();

    let mut buckets: std::collections::BTreeMap<String, Vec<Geometry>> =
        std::collections::BTreeMap::new();
    traverse_by_color(
        root,
        Matrix::identity(),
        mode,
        &mut buckets,
        scale_x,
        scale_y,
    );

    Ok(buckets
        .into_iter()
        .filter(|(_, geos)| !geos.is_empty())
        .collect())
}

/// Like [`svg_string_to_geometries_by_color`] but merges each color
/// bucket's subpaths into a single [`Geometry`].
pub fn svg_string_to_geometry_by_color(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
    mode: ColorAttr,
) -> RaygeoResult<Vec<(String, Geometry)>> {
    let buckets =
        svg_string_to_geometries_by_color(svg_str, scale_x, scale_y, mode)?;
    Ok(buckets
        .into_iter()
        .map(|(key, geos)| {
            let mut combined = Geometry::new();
            for g in geos {
                combined.extend(&g);
            }
            (key, combined)
        })
        .collect())
}

/// Return a copy of `svg_str` containing only the shapes whose resolved
/// color includes `color_key`.
///
/// Non-matching shape elements are removed by byte range, so the rest of
/// the document (groups, defs, namespaces) is preserved verbatim. Shapes
/// in `_no_color` are kept when `color_key` is `_no_color`.
///
/// Returns `Err` when the SVG cannot be parsed.
pub fn filter_svg_by_color(
    svg_str: &str,
    mode: ColorAttr,
    color_key: &str,
) -> RaygeoResult<String> {
    let doc = roxmltree::Document::parse(svg_str)
        .map_err(|e| RaygeoError::SvgParseError(format!("{e}")))?;
    let root = doc.root_element();

    let mut remove: Vec<Range<usize>> = Vec::new();
    collect_color_remove_ranges(root, mode, color_key, &mut remove);
    remove.sort_by_key(|r| r.start);

    let mut out = String::with_capacity(svg_str.len());
    let mut pos = 0usize;
    for range in remove {
        if range.start >= pos {
            out.push_str(&svg_str[pos..range.start]);
            pos = range.end;
        }
    }
    out.push_str(&svg_str[pos..]);
    Ok(out)
}

/// Convert a normalised Geometry into an SVG path `d` string.
///
/// Coordinates are scaled by (`width`, `height`) and Y is flipped
/// (SVG Y increases downward).
pub fn geometry_to_svg_path(
    geometry: &Geometry,
    width: i32,
    height: i32,
) -> String {
    let data = geometry.data();
    if data.is_empty() {
        return String::new();
    }
    let w = width as f64;
    let h = height as f64;
    let mut parts = Vec::with_capacity(data.len());
    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    for cmd in data {
        let (ex, ey, _) =
            (cmd.end_point().x, cmd.end_point().y, cmd.end_point().z);
        let x = ex * w;
        let y = h * (1.0 - ey);
        match cmd {
            Command::Move { .. } => {
                parts.push(format!("M {x:.3} {y:.3}"));
            }
            Command::Line { .. } => {
                parts.push(format!("L {x:.3} {y:.3}"));
            }
            Command::Arc {
                center_offset,
                normal,
                ..
            } => {
                let clockwise = normal.z < 0.0;
                let r = center_offset.x.hypot(center_offset.y);
                let sweep_flag = if clockwise { 1 } else { 0 };
                let cx = prev_x + center_offset.x;
                let cy = prev_y + center_offset.y;
                let start_angle = (prev_y - cy).atan2(prev_x - cx);
                let end_angle = (ey - cy).atan2(ex - cx);
                let sweep = get_arc_sweep(start_angle, end_angle, clockwise);
                let large = if sweep.abs() > PI + 1e-9 { 1 } else { 0 };
                parts.push(format!(
                    "A {:.3} {:.3} 0 {large} {sweep_flag} {x:.3} {y:.3}",
                    r * w,
                    r * h
                ));
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let (c1x, c1y, _) = (control1.x, control1.y, control1.z);
                let (c2x, c2y, _) = (control2.x, control2.y, control2.z);
                parts.push(format!(
                    "C {:.3} {:.3} {:.3} {:.3} {x:.3} {y:.3}",
                    c1x * w,
                    h * (1.0 - c1y),
                    c2x * w,
                    h * (1.0 - c2y),
                ));
            }
        }
        prev_x = ex;
        prev_y = ey;
    }
    parts.join(" ")
}
