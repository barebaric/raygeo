use std::str::FromStr;

// ── Color resolution ───────────────────────────────────────────────

/// A resolved paint: a concrete `#rrggbb` color, or `None` for `none`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Paint {
    None,
    Color(svgtypes::Color),
}

/// Parse a single CSS declaration block value for `property`.
///
/// SVG presentation attributes may also be set inline via the `style`
/// attribute, e.g. `style="fill: red; stroke: none"`. This returns the
/// trimmed value for `property` if present.
fn style_attr_value(node: &roxmltree::Node, property: &str) -> Option<String> {
    let style = node.attribute("style")?;
    for decl in style.split(';') {
        let mut parts = decl.splitn(2, ':');
        let key = parts.next()?.trim();
        if key.eq_ignore_ascii_case(property) {
            return parts.next().map(|v| v.trim().to_string());
        }
    }
    None
}

/// Read a presentation attribute that may be set as either an XML
/// attribute (`fill="red"`) or inside a `style="..."` CSS block.
fn presentation_attr(node: &roxmltree::Node, name: &str) -> Option<String> {
    if let Some(v) = style_attr_value(node, name) {
        return Some(v);
    }
    node.attribute(name).map(|v| v.trim().to_string())
}

/// Try to parse a paint string. Handles `none`, `currentColor`, the
/// `url(#id)` paint-server form (treated as `None`), and any color
/// form understood by `svgtypes::Color`.
fn parse_paint(value: &str) -> Option<Paint> {
    let v = value.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("none") {
        return Some(Paint::None);
    }
    if v.starts_with("url(") {
        return Some(Paint::None);
    }
    if v.eq_ignore_ascii_case("currentColor")
        || v.eq_ignore_ascii_case("inherit")
    {
        // Resolved by the caller; return None so the walker knows to keep
        // inheriting.
        return None;
    }
    match svgtypes::Color::from_str(v) {
        Ok(c) => Some(Paint::Color(c)),
        Err(_) => Some(Paint::None),
    }
}

/// Resolve the effective `color` attribute for an element, walking up
/// the ancestor chain. `currentColor` for the `color` property itself
/// falls back to black, per the user-facing decision.
fn resolve_color_attr(node: roxmltree::Node) -> svgtypes::Color {
    let mut n = Some(node);
    while let Some(n2) = n {
        if let Some(v) = presentation_attr(&n2, "color") {
            if v.eq_ignore_ascii_case("currentColor")
                || v.eq_ignore_ascii_case("inherit")
            {
                n = n2.parent_element();
                continue;
            }
            if let Some(Paint::Color(c)) = parse_paint(&v) {
                return c;
            }
        }
        n = n2.parent_element();
    }
    svgtypes::Color::black()
}

/// Resolve the effective paint for `property` (`"fill"` or `"stroke"`),
/// walking up the ancestor chain. `currentColor` resolves against the
/// nearest `color` attribute (defaulting to black). Returns `None`
/// (meaning "no explicit paint") when no ancestor sets the property;
/// callers decide how to treat that.
pub(crate) fn resolve_paint(
    node: roxmltree::Node,
    property: &str,
) -> Option<Paint> {
    let mut n = Some(node);
    while let Some(n2) = n {
        if let Some(v) = presentation_attr(&n2, property) {
            if v.eq_ignore_ascii_case("currentColor") {
                return Some(Paint::Color(resolve_color_attr(n2)));
            }
            if v.eq_ignore_ascii_case("inherit") {
                n = n2.parent_element();
                continue;
            }
            return parse_paint(&v);
        }
        n = n2.parent_element();
    }
    None
}

/// Which color attribute(s) of a shape determine its bucket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorAttr {
    /// Bucket by the resolved `fill` paint.
    Fill,
    /// Bucket by the resolved `stroke` paint.
    Stroke,
    /// Use the `fill` paint when present, otherwise the `stroke` paint.
    FillElseStroke,
    /// Bucket by fill and stroke independently: a shape whose fill differs
    /// from its stroke lands in two buckets (one per color).
    Any,
}

/// Resolve the bucketing paint for `node` per the requested [`ColorAttr`].
///
/// Returns `None` when the chosen color attribute is genuinely absent
/// across the whole ancestor chain, so the caller can place the element
/// in the `_no_color` bucket.
pub(crate) fn resolve_bucket_paint(
    node: roxmltree::Node,
    mode: ColorAttr,
) -> Option<Paint> {
    match mode {
        ColorAttr::Fill => resolve_paint(node, "fill"),
        ColorAttr::Stroke => resolve_paint(node, "stroke"),
        ColorAttr::FillElseStroke => match resolve_paint(node, "fill") {
            Some(Paint::Color(c)) => Some(Paint::Color(c)),
            Some(Paint::None) => resolve_paint(node, "stroke"),
            None => resolve_paint(node, "stroke"),
        },
        // The `any` mode resolves fill and stroke independently in
        // `traverse_by_color`; this helper is only used for single-bucket
        // modes. Falling back to fill keeps the return type total.
        ColorAttr::Any => resolve_paint(node, "fill"),
    }
}
