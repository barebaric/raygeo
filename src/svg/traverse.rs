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

use std::collections::HashSet;
use std::ops::Range;

const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

pub(crate) const NO_COLOR_BUCKET: &str = "_no_color";

/// Tags whose children are not rendered directly. They are only
/// rendered when referenced by a `<use>` element.
const NON_RENDERING_CONTAINERS: &[&str] = &["defs", "symbol"];

/// Resolve the target element ID from a `<use>` element's `href` or
/// `xlink:href` attribute, stripping the leading `#`.
fn resolve_use_href(node: roxmltree::Node) -> Option<String> {
    if let Some(href) = node.attribute("href") {
        return Some(href.trim_start_matches('#').to_string());
    }
    if let Some(href) = node.attribute((XLINK_NS, "href")) {
        return Some(href.trim_start_matches('#').to_string());
    }
    None
}

/// Find the first element with the given `id` attribute in the document.
fn find_element_by_id<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    id: &'a str,
) -> Option<roxmltree::Node<'a, 'input>> {
    doc.descendants()
        .find(|n| n.is_element() && n.attribute("id") == Some(id))
}

/// Check whether `tag` is a non-rendering container whose children
/// should be skipped during normal tree traversal.
fn is_non_rendering_container(tag: &str) -> bool {
    NON_RENDERING_CONTAINERS.contains(&tag)
}

pub(crate) fn traverse(
    node: roxmltree::Node,
    parent_tfm: Matrix,
    geos: &mut Vec<Geometry>,
    scale_x: f64,
    scale_y: f64,
) {
    let mut visited = HashSet::new();
    traverse_impl(node, parent_tfm, geos, scale_x, scale_y, &mut visited);
}

fn traverse_impl(
    node: roxmltree::Node,
    parent_tfm: Matrix,
    geos: &mut Vec<Geometry>,
    scale_x: f64,
    scale_y: f64,
    visited: &mut HashSet<String>,
) {
    if is_hidden(&node) {
        return;
    }

    let local = parse_svg_transform(node.attribute("transform").unwrap_or(""));
    let combined = parent_tfm * local;

    match node.tag_name().name() {
        "use" => {
            if let Some(href) = resolve_use_href(node) {
                if !visited.contains(&href) {
                    let doc = node.document();
                    if let Some(target) = find_element_by_id(doc, &href) {
                        let x = node
                            .attribute("x")
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let y = node
                            .attribute("y")
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let use_tfm = combined * Matrix::from_translation(x, y);
                        visited.insert(href.clone());
                        traverse_impl(
                            target, use_tfm, geos, scale_x, scale_y, visited,
                        );
                        visited.remove(&href);
                    }
                }
            }
            return;
        }
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
            let tag = child.tag_name().name();
            if is_non_rendering_container(tag) {
                continue;
            }
            traverse_impl(child, combined, geos, scale_x, scale_y, visited);
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
    let mut visited = HashSet::new();
    traverse_by_color_impl(
        node,
        parent_tfm,
        mode,
        buckets,
        scale_x,
        scale_y,
        &mut visited,
    );
}

fn traverse_by_color_impl(
    node: roxmltree::Node,
    parent_tfm: Matrix,
    mode: ColorAttr,
    buckets: &mut std::collections::BTreeMap<String, Vec<Geometry>>,
    scale_x: f64,
    scale_y: f64,
    visited: &mut HashSet<String>,
) {
    if is_hidden(&node) {
        return;
    }

    let local = parse_svg_transform(node.attribute("transform").unwrap_or(""));
    let combined = parent_tfm * local;

    let tag = node.tag_name().name();

    if tag == "use" {
        if let Some(href) = resolve_use_href(node) {
            if !visited.contains(&href) {
                let doc = node.document();
                if let Some(target) = find_element_by_id(doc, &href) {
                    let x = node
                        .attribute("x")
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let y = node
                        .attribute("y")
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let use_tfm = combined * Matrix::from_translation(x, y);
                    visited.insert(href.clone());
                    traverse_by_color_impl(
                        target, use_tfm, mode, buckets, scale_x, scale_y,
                        visited,
                    );
                    visited.remove(&href);
                }
            }
        }
        return;
    }

    let is_shape = matches!(
        tag,
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
            let child_tag = child.tag_name().name();
            if is_non_rendering_container(child_tag) {
                continue;
            }
            traverse_by_color_impl(
                child, combined, mode, buckets, scale_x, scale_y, visited,
            );
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
    let mut visited = HashSet::new();
    collect_color_remove_ranges_impl(
        node,
        mode,
        color_key,
        remove,
        &mut visited,
    );
}

fn collect_color_remove_ranges_impl(
    node: roxmltree::Node,
    mode: ColorAttr,
    color_key: &str,
    remove: &mut Vec<Range<usize>>,
    visited: &mut HashSet<String>,
) {
    if is_hidden(&node) {
        return;
    }

    let tag = node.tag_name().name();

    if tag == "use" {
        if let Some(href) = resolve_use_href(node) {
            if !visited.contains(&href) {
                visited.insert(href.clone());
                let doc = node.document();
                if let Some(target) = find_element_by_id(doc, &href) {
                    if !subtree_matches_color(target, mode, color_key, visited)
                    {
                        remove.push(node.range());
                    }
                }
                visited.remove(&href);
            }
        }
        return;
    }

    let is_shape = matches!(
        tag,
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
            let child_tag = child.tag_name().name();
            if is_non_rendering_container(child_tag) {
                continue;
            }
            collect_color_remove_ranges_impl(
                child, mode, color_key, remove, visited,
            );
        }
    }
}

/// Check whether any shape in the subtree rooted at `node` resolves to
/// `color_key`. Used to decide whether a `<use>` element should be kept
/// when filtering by color.
fn subtree_matches_color(
    node: roxmltree::Node,
    mode: ColorAttr,
    color_key: &str,
    visited: &mut HashSet<String>,
) -> bool {
    if is_hidden(&node) {
        return false;
    }

    let tag = node.tag_name().name();

    if tag == "use" {
        if let Some(href) = resolve_use_href(node) {
            if !visited.contains(&href) {
                visited.insert(href.clone());
                let doc = node.document();
                if let Some(target) = find_element_by_id(doc, &href) {
                    let found =
                        subtree_matches_color(target, mode, color_key, visited);
                    visited.remove(&href);
                    if found {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    let is_shape = matches!(
        tag,
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
        if keys.iter().any(|k| k == color_key) {
            return true;
        }
    }

    for child in node.children() {
        if child.is_element() {
            let child_tag = child.tag_name().name();
            if is_non_rendering_container(child_tag) {
                continue;
            }
            if subtree_matches_color(child, mode, color_key, visited) {
                return true;
            }
        }
    }

    false
}
