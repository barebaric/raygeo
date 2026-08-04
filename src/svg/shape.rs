use crate::svg::parse_coords;

fn attr_f64(node: &roxmltree::Node, name: &str) -> Option<f64> {
    node.attribute(name).and_then(|v| v.parse::<f64>().ok())
}

pub(crate) fn is_hidden(node: &roxmltree::Node) -> bool {
    if let Some(d) = node.attribute("display") {
        if d == "none" {
            return true;
        }
    }
    if let Some(v) = node.attribute("visibility") {
        if v == "hidden" || v == "collapse" {
            return true;
        }
    }
    false
}

pub(crate) fn rect_to_d(node: &roxmltree::Node) -> Option<String> {
    let x = attr_f64(node, "x").unwrap_or(0.0);
    let y = attr_f64(node, "y").unwrap_or(0.0);
    let w = attr_f64(node, "width")?;
    let h = attr_f64(node, "height")?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let rx = attr_f64(node, "rx").unwrap_or(0.0).min(w / 2.0);
    let ry = attr_f64(node, "ry").unwrap_or(0.0).min(h / 2.0);
    let rx = if rx > 0.0 || ry > 0.0 {
        if rx > 0.0 {
            rx
        } else {
            ry
        }
    } else {
        0.0
    };
    let ry = if ry > 0.0 { ry } else { rx };
    if rx > 0.0 && ry > 0.0 {
        Some(format!(
            "M {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 1 {} {} Z",
            x, y + ry, rx, ry, x + rx, y,
            x + w - rx, y, rx, ry, x + w, y + ry,
            x + w, y + h - ry, rx, ry, x + w - rx, y + h,
            x + rx, y + h, rx, ry, x, y + h - ry,
        ))
    } else {
        Some(format!(
            "M {} {} L {} {} L {} {} L {} {} Z",
            x,
            y,
            x + w,
            y,
            x + w,
            y + h,
            x,
            y + h
        ))
    }
}

pub(crate) fn circle_to_d(node: &roxmltree::Node) -> Option<String> {
    let cx = attr_f64(node, "cx").unwrap_or(0.0);
    let cy = attr_f64(node, "cy").unwrap_or(0.0);
    let r = attr_f64(node, "r")?;
    if r <= 0.0 {
        return None;
    }
    Some(format!(
        "M {} {} A {} {} 0 1 1 {} {} A {} {} 0 1 1 {} {} Z",
        cx,
        cy - r,
        r,
        r,
        cx,
        cy + r,
        r,
        r,
        cx,
        cy - r
    ))
}

pub(crate) fn ellipse_to_d(node: &roxmltree::Node) -> Option<String> {
    let cx = attr_f64(node, "cx").unwrap_or(0.0);
    let cy = attr_f64(node, "cy").unwrap_or(0.0);
    let rx = attr_f64(node, "rx")?;
    let ry = attr_f64(node, "ry")?;
    if rx <= 0.0 || ry <= 0.0 {
        return None;
    }
    Some(format!(
        "M {} {} A {} {} 0 1 1 {} {} A {} {} 0 1 1 {} {} Z",
        cx,
        cy - ry,
        rx,
        ry,
        cx,
        cy + ry,
        rx,
        ry,
        cx,
        cy - ry
    ))
}

pub(crate) fn line_to_d(node: &roxmltree::Node) -> Option<String> {
    let x1 = attr_f64(node, "x1").unwrap_or(0.0);
    let y1 = attr_f64(node, "y1").unwrap_or(0.0);
    let x2 = attr_f64(node, "x2").unwrap_or(0.0);
    let y2 = attr_f64(node, "y2").unwrap_or(0.0);
    Some(format!("M {} {} L {} {}", x1, y1, x2, y2))
}

pub(crate) fn poly_to_d(node: &roxmltree::Node) -> Option<String> {
    let tag = node.tag_name().name();
    let pts = node.attribute("points")?;
    let coords = parse_coords(pts);
    if coords.len() < 2 {
        return None;
    }
    let mut d = format!("M {} {}", coords[0], coords[1]);
    for i in (2..coords.len()).step_by(2) {
        if i + 1 < coords.len() {
            d.push_str(&format!(" L {} {}", coords[i], coords[i + 1]));
        }
    }
    if tag == "polygon" {
        d.push_str(" Z");
    }
    Some(d)
}
