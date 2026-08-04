use crate::geo::geometry::Geometry;
use crate::geo::matrix::Matrix;
use crate::svg::color::{
    resolve_bucket_paint, resolve_paint, ColorAttr, Paint,
};
use crate::svg::parse_svg_path_data;
use crate::svg::shape::{
    circle_to_d, ellipse_to_d, is_hidden, line_to_d, poly_to_d, rect_to_d,
};
use crate::svg::transform::parse_svg_transform;

use std::ops::Range;

pub(crate) const NO_COLOR_BUCKET: &str = "_no_color";

pub(crate) fn traverse(
    node: roxmltree::Node,
    parent_tfm: Matrix,
    geos: &mut Vec<Geometry>,
    scale_x: f64,
    scale_y: f64,
) {
    if is_hidden(&node) {
        return;
    }

    let local = parse_svg_transform(node.attribute("transform").unwrap_or(""));
    let combined = parent_tfm * local;

    match node.tag_name().name() {
        "path" => {
            if let Some(d) = node.attribute("d") {
                if let Ok(g) =
                    parse_svg_path_data(d, &combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "rect" => {
            if let Some(d) = rect_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, &combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "circle" => {
            if let Some(d) = circle_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, &combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "ellipse" => {
            if let Some(d) = ellipse_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, &combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "line" => {
            if let Some(d) = line_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, &combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "polyline" | "polygon" => {
            if let Some(d) = poly_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, &combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        _ => {}
    }

    for child in node.children() {
        if child.is_element() {
            traverse(child, combined, geos, scale_x, scale_y);
        }
    }
}

/// Recursively walk the SVG tree, parsing shapes and accumulating
/// geometries keyed by their resolved color bucket. `mode` selects
/// which presentation attribute (`fill`, `stroke`, fill-else-stroke, or
/// `any`) determines the bucket. In `any` mode a shape whose fill differs
/// from its stroke lands in two buckets, one per color. Shapes for which
/// the chosen attribute is `none` (or wholly absent) go into the
/// [`NO_COLOR_BUCKET`].
pub(crate) fn traverse_by_color(
    node: roxmltree::Node,
    parent_tfm: Matrix,
    mode: ColorAttr,
    buckets: &mut std::collections::BTreeMap<String, Vec<Geometry>>,
    scale_x: f64,
    scale_y: f64,
) {
    if is_hidden(&node) {
        return;
    }

    let local = parse_svg_transform(node.attribute("transform").unwrap_or(""));
    let combined = parent_tfm * local;

    let is_shape = matches!(
        node.tag_name().name(),
        "path"
            | "rect"
            | "circle"
            | "ellipse"
            | "line"
            | "polyline"
            | "polygon"
    );
    if is_shape {
        let bucket_keys = bucket_keys_for(node, mode);
        for key in bucket_keys {
            add_to_bucket(node, combined, &key, buckets, scale_x, scale_y);
        }
    }

    for child in node.children() {
        if child.is_element() {
            traverse_by_color(child, combined, mode, buckets, scale_x, scale_y);
        }
    }
}

/// Compute the bucket key(s) a shape belongs to for the given mode.
///
/// In [`ColorAttr::Any`] mode the shape is bucketed by both its fill and
/// its stroke when they differ, producing two buckets.
fn bucket_keys_for(node: roxmltree::Node, mode: ColorAttr) -> Vec<String> {
    if mode == ColorAttr::Any {
        let fill_key = paint_color_key(resolve_paint(node, "fill"));
        let stroke_key = paint_color_key(resolve_paint(node, "stroke"));
        let mut keys: Vec<String> = Vec::new();
        if let Some(key) = &fill_key {
            keys.push(key.clone());
        }
        if let Some(key) = &stroke_key {
            if stroke_key != fill_key {
                keys.push(key.clone());
            }
        }
        if keys.is_empty() {
            keys.push(NO_COLOR_BUCKET.to_string());
        }
        return keys;
    }

    let key = match resolve_bucket_paint(node, mode) {
        Some(Paint::Color(c)) => {
            format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue)
        }
        _ => NO_COLOR_BUCKET.to_string(),
    };
    vec![key]
}

/// Convert a resolved paint to a lowercase `#rrggbb` bucket key, or `None`
/// when the paint is absent or `none`.
fn paint_color_key(paint: Option<Paint>) -> Option<String> {
    match paint {
        Some(Paint::Color(c)) => {
            Some(format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue))
        }
        _ => None,
    }
}

/// Parse a shape's `d` data and append it to the given bucket.
fn add_to_bucket(
    node: roxmltree::Node,
    combined: Matrix,
    key: &str,
    buckets: &mut std::collections::BTreeMap<String, Vec<Geometry>>,
    scale_x: f64,
    scale_y: f64,
) {
    let entry = buckets.entry(key.to_string()).or_default();
    let d = match node.tag_name().name() {
        "path" => node.attribute("d").map(String::from),
        "rect" => rect_to_d(&node),
        "circle" => circle_to_d(&node),
        "ellipse" => ellipse_to_d(&node),
        "line" => line_to_d(&node),
        "polyline" | "polygon" => poly_to_d(&node),
        _ => None,
    };
    if let Some(d) = d {
        if let Ok(g) = parse_svg_path_data(&d, &combined, scale_x, scale_y) {
            entry.extend(g);
        }
    }
}

/// Recursively collect the byte ranges of shape elements whose resolved
/// color bucket does not include `color_key`.
///
/// The collected ranges can be sliced out of the original SVG string to
/// leave only shapes of the requested color, preserving all surrounding
/// structure (groups, defs, namespaces) untouched.
pub(crate) fn collect_color_remove_ranges(
    node: roxmltree::Node,
    mode: ColorAttr,
    color_key: &str,
    remove: &mut Vec<Range<usize>>,
) {
    if is_hidden(&node) {
        return;
    }

    let is_shape = matches!(
        node.tag_name().name(),
        "path"
            | "rect"
            | "circle"
            | "ellipse"
            | "line"
            | "polyline"
            | "polygon"
    );
    if is_shape {
        let keys = bucket_keys_for(node, mode);
        if !keys.iter().any(|k| k == color_key) {
            remove.push(node.range());
        }
    }

    for child in node.children() {
        if child.is_element() {
            collect_color_remove_ranges(child, mode, color_key, remove);
        }
    }
}
