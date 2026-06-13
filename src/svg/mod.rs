use crate::geo::geometry::Geometry;
use crate::types::Command;

const BORDER_SIZE: f64 = 2.0;

fn parse_path_coords(coords_str: &str) -> Vec<f64> {
    let mut coords = Vec::new();
    let mut chars = coords_str.chars().peekable();
    let mut buf = String::new();

    while let Some(&ch) = chars.peek() {
        if ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit() {
            buf.clear();
            if ch == '-' || ch == '+' {
                buf.push(chars.next().unwrap());
            }
            let mut has_dot = false;
            let mut has_exp = false;
            loop {
                match chars.peek() {
                    Some(&c) if c.is_ascii_digit() => {
                        buf.push(chars.next().unwrap());
                    }
                    Some(&'.') if !has_dot && !has_exp => {
                        has_dot = true;
                        buf.push(chars.next().unwrap());
                    }
                    Some(&'e') | Some(&'E') if !has_exp => {
                        has_exp = true;
                        buf.push(chars.next().unwrap());
                        if chars.peek() == Some(&'+')
                            || chars.peek() == Some(&'-')
                        {
                            buf.push(chars.next().unwrap());
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

fn parse_path_tokens(path_data: &str) -> Vec<(char, String)> {
    let mut tokens = Vec::new();
    let mut current_cmd = 0u8;
    let mut current_coords = String::new();

    for ch in path_data.chars() {
        if matches!(
            ch,
            'M' | 'm'
                | 'L'
                | 'l'
                | 'H'
                | 'h'
                | 'V'
                | 'v'
                | 'C'
                | 'c'
                | 'Z'
                | 'z'
        ) {
            if current_cmd != 0 {
                tokens.push((current_cmd as char, current_coords.clone()));
            }
            current_cmd = ch as u8;
            current_coords.clear();
        } else {
            current_coords.push(ch);
        }
    }
    if current_cmd != 0 {
        tokens.push((current_cmd as char, current_coords));
    }
    tokens
}

fn apply_transform(px: f64, py: f64, transform: &[[f64; 3]; 3]) -> (f64, f64) {
    let tx = transform[0][0] * px + transform[0][1] * py + transform[0][2];
    let ty = transform[1][0] * px + transform[1][1] * py + transform[1][2];
    (tx, ty)
}

fn transform_point(
    px: f64,
    py: f64,
    transform: &[[f64; 3]; 3],
    scale_x: f64,
    scale_y: f64,
) -> (f64, f64) {
    let (tx, ty) = apply_transform(px, py, transform);
    ((tx - BORDER_SIZE) / scale_x, (ty - BORDER_SIZE) / scale_y)
}

fn flatten_bezier(
    start: (f64, f64),
    c1: (f64, f64),
    c2: (f64, f64),
    end: (f64, f64),
    num_steps: usize,
) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(num_steps);
    for i in 1..=num_steps {
        let t = i as f64 / num_steps as f64;
        let omt = 1.0 - t;
        let px = omt * omt * omt * start.0
            + 3.0 * omt * omt * t * c1.0
            + 3.0 * omt * t * t * c2.0
            + t * t * t * end.0;
        let py = omt * omt * omt * start.1
            + 3.0 * omt * omt * t * c1.1
            + 3.0 * omt * t * t * c2.1
            + t * t * t * end.1;
        points.push((px, py));
    }
    points
}

pub fn parse_svg_transform(transform_str: &str) -> [[f64; 3]; 3] {
    let mut matrix = [[0.0f64; 3]; 3];
    matrix[0][0] = 1.0;
    matrix[1][1] = 1.0;
    matrix[2][2] = 1.0;

    if transform_str.is_empty() {
        return matrix;
    }

    if let Some(rest) = transform_str.strip_prefix("translate(") {
        if let Some(inner) = rest.strip_suffix(')') {
            let coords = parse_path_coords(inner);
            if !coords.is_empty() {
                matrix[0][2] = coords[0];
                if coords.len() > 1 {
                    matrix[1][2] = coords[1];
                }
            }
        }
    }

    matrix
}

pub fn parse_svg_path_data(
    path_data: &str,
    transform: &[[f64; 3]; 3],
    scale_x: f64,
    scale_y: f64,
) -> Vec<Geometry> {
    let tokens = parse_path_tokens(path_data);
    let mut geometries = Vec::new();
    let mut current_geo: Option<Geometry> = None;
    let mut pos = (0.0f64, 0.0f64);
    let mut subpath_start = (0.0f64, 0.0f64);

    for (cmd, coords_str) in &tokens {
        let coords = parse_path_coords(coords_str);

        match cmd {
            'M' | 'm' => {
                if let Some(geo) = current_geo.take() {
                    if !geo.is_empty() {
                        geometries.push(geo);
                    }
                }
                let mut geo = Geometry::new();
                if coords.len() >= 2 {
                    if *cmd == 'm' {
                        pos.0 += coords[0];
                        pos.1 += coords[1];
                    } else {
                        pos.0 = coords[0];
                        pos.1 = coords[1];
                    }
                    subpath_start = pos;
                    let (tx, ty) = transform_point(
                        pos.0, pos.1, transform, scale_x, scale_y,
                    );
                    geo.move_to(tx, ty, 0.0);

                    for i in (2..coords.len()).step_by(2) {
                        if i + 1 < coords.len() {
                            if *cmd == 'm' {
                                pos.0 += coords[i];
                                pos.1 += coords[i + 1];
                            } else {
                                pos.0 = coords[i];
                                pos.1 = coords[i + 1];
                            }
                            let (tx, ty) = transform_point(
                                pos.0, pos.1, transform, scale_x, scale_y,
                            );
                            geo.line_to(tx, ty, 0.0);
                        }
                    }
                }
                current_geo = Some(geo);
            }
            'L' | 'l' | 'H' | 'h' | 'V' | 'v' => {
                if let Some(ref mut geo) = current_geo {
                    match cmd {
                        'L' => {
                            pos.0 = coords[0];
                            pos.1 = coords[1];
                        }
                        'l' => {
                            pos.0 += coords[0];
                            pos.1 += coords[1];
                        }
                        'H' => {
                            pos.0 = coords[0];
                        }
                        'h' => {
                            pos.0 += coords[0];
                        }
                        'V' => {
                            pos.1 = coords[0];
                        }
                        'v' => {
                            pos.1 += coords[0];
                        }
                        _ => unreachable!(),
                    }
                    let (tx, ty) = transform_point(
                        pos.0, pos.1, transform, scale_x, scale_y,
                    );
                    geo.line_to(tx, ty, 0.0);
                }
            }
            'C' | 'c' => {
                if let Some(ref mut geo) = current_geo {
                    for i in (0..coords.len()).step_by(6) {
                        if i + 5 >= coords.len() {
                            break;
                        }
                        let (c1x, c1y, c2x, c2y, ex, ey) = if *cmd == 'C' {
                            (
                                coords[i],
                                coords[i + 1],
                                coords[i + 2],
                                coords[i + 3],
                                coords[i + 4],
                                coords[i + 5],
                            )
                        } else {
                            (
                                pos.0 + coords[i],
                                pos.1 + coords[i + 1],
                                pos.0 + coords[i + 2],
                                pos.1 + coords[i + 3],
                                pos.0 + coords[i + 4],
                                pos.1 + coords[i + 5],
                            )
                        };

                        let points = flatten_bezier(
                            pos,
                            (c1x, c1y),
                            (c2x, c2y),
                            (ex, ey),
                            20,
                        );
                        for (px, py) in points {
                            let (tx, ty) = transform_point(
                                px, py, transform, scale_x, scale_y,
                            );
                            geo.line_to(tx, ty, 0.0);
                        }
                        pos = (ex, ey);
                    }
                }
            }
            'Z' | 'z' => {
                if let Some(ref mut geo) = current_geo {
                    geo.close_path();
                }
                pos = subpath_start;
            }
            _ => {}
        }
    }

    if let Some(geo) = current_geo.take() {
        if !geo.is_empty() {
            geometries.push(geo);
        }
    }

    geometries
}

pub fn svg_string_to_geometries(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> Vec<Geometry> {
    let doc = match roxmltree::Document::parse(svg_str) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut all_geometries = Vec::new();
    let identity = parse_svg_transform("");
    traverse_svg_node(
        doc.root_element(),
        &identity,
        &mut all_geometries,
        scale_x,
        scale_y,
    );
    all_geometries
}

fn traverse_svg_node(
    node: roxmltree::Node,
    parent_transform: &[[f64; 3]; 3],
    all_geometries: &mut Vec<Geometry>,
    scale_x: f64,
    scale_y: f64,
) {
    let local_transform =
        parse_svg_transform(node.attribute("transform").unwrap_or(""));
    let combined = mat3_mul(parent_transform, &local_transform);

    let tag = node.tag_name().name();
    if tag == "path" {
        if let Some(path_data) = node.attribute("d") {
            let geos =
                parse_svg_path_data(path_data, &combined, scale_x, scale_y);
            all_geometries.extend(geos);
        }
    }

    for child in node.children() {
        if child.is_element() {
            traverse_svg_node(
                child,
                &combined,
                all_geometries,
                scale_x,
                scale_y,
            );
        }
    }
}

fn mat3_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut r = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

/// Converts a Geometry's commands into an SVG path `d` attribute string.
///
/// The geometry coordinates are in normalized [0, 1] space. They are scaled
/// to pixel dimensions via `width` and `height`, and the Y axis is flipped
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

    for cmd in data {
        let (ex, ey, _) = cmd.end_point();
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
                center_offset: (i, j),
                clockwise,
                ..
            } => {
                let radius = i.hypot(*j);
                let rx = radius * w;
                let ry = radius * h;
                let sweep = if *clockwise { 1 } else { 0 };
                parts.push(format!(
                    "A {rx:.3} {ry:.3} 0 0 {sweep} {x:.3} {y:.3}"
                ));
            }
            Command::Bezier {
                control1: (c1x, c1y),
                control2: (c2x, c2y),
                ..
            } => {
                let c1x = c1x * w;
                let c1y = h * (1.0 - c1y);
                let c2x = c2x * w;
                let c2y = h * (1.0 - c2y);
                parts.push(format!(
                    "C {c1x:.3} {c1y:.3} {c2x:.3} {c2y:.3} {x:.3} {y:.3}"
                ));
            }
        }
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_coords_basic() {
        let coords = parse_path_coords("10.0 20.0 30.0");
        assert_eq!(coords, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_parse_path_coords_negative() {
        let coords = parse_path_coords("-10.5,20.3 -5.0");
        assert_eq!(coords, vec![-10.5, 20.3, -5.0]);
    }

    #[test]
    fn test_parse_path_coords_scientific() {
        let coords = parse_path_coords("1e2 2E-1");
        assert!((coords[0] - 100.0).abs() < 1e-10);
        assert!((coords[1] - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_parse_svg_transform_empty() {
        let m = parse_svg_transform("");
        assert_eq!(m[0][0], 1.0);
        assert_eq!(m[1][1], 1.0);
        assert_eq!(m[0][2], 0.0);
    }

    #[test]
    fn test_parse_svg_transform_translate() {
        let m = parse_svg_transform("translate(10.5, 20.0)");
        assert_eq!(m[0][2], 10.5);
        assert_eq!(m[1][2], 20.0);
    }

    #[test]
    fn test_parse_svg_transform_translate_one_arg() {
        let m = parse_svg_transform("translate(5.0)");
        assert_eq!(m[0][2], 5.0);
        assert_eq!(m[1][2], 0.0);
    }

    #[test]
    fn test_flatten_bezier_line() {
        let points =
            flatten_bezier((0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (10.0, 0.0), 5);
        assert_eq!(points.len(), 5);
        assert!((points[4].0 - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_svg_path_moveto_lineto() {
        let identity = parse_svg_transform("");
        let geos =
            parse_svg_path_data("M 0 0 L 10 0 L 10 10 Z", &identity, 1.0, 1.0);
        assert_eq!(geos.len(), 1);
        assert!(!geos[0].is_empty());
    }

    #[test]
    fn test_parse_svg_path_relative() {
        let identity = parse_svg_transform("");
        let geos =
            parse_svg_path_data("m 2 2 l 10 0 l 0 10 z", &identity, 1.0, 1.0);
        assert_eq!(geos.len(), 1);
        assert!(!geos[0].is_empty());
    }

    #[test]
    fn test_parse_svg_path_curveto() {
        let identity = parse_svg_transform("");
        let geos = parse_svg_path_data(
            "M 0 0 C 10 0 10 10 0 10 Z",
            &identity,
            1.0,
            1.0,
        );
        assert_eq!(geos.len(), 1);
        assert!(!geos[0].is_empty());
    }

    #[test]
    fn test_parse_svg_path_multiple_subpaths() {
        let identity = parse_svg_transform("");
        let geos = parse_svg_path_data(
            "M 0 0 L 5 5 M 10 10 L 15 15",
            &identity,
            1.0,
            1.0,
        );
        assert_eq!(geos.len(), 2);
    }

    #[test]
    fn test_parse_svg_path_hv_commands() {
        let identity = parse_svg_transform("");
        let geos =
            parse_svg_path_data("M 0 0 H 10 V 10 H 0 Z", &identity, 1.0, 1.0);
        assert_eq!(geos.len(), 1);
        assert!(!geos[0].is_empty());
    }

    #[test]
    fn test_svg_string_to_geometries_basic() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0 0 L 10 0 L 10 10 Z"/></svg>"#;
        let geos = svg_string_to_geometries(svg, 1.0, 1.0);
        assert_eq!(geos.len(), 1);
    }

    #[test]
    fn test_svg_string_to_geometries_with_transform() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="translate(5,3)"><path d="M 0 0 L 10 0 L 10 10 Z"/></g></svg>"#;
        let geos = svg_string_to_geometries(svg, 1.0, 1.0);
        assert_eq!(geos.len(), 1);
    }

    #[test]
    fn test_svg_string_invalid_xml() {
        let geos = svg_string_to_geometries("not xml", 1.0, 1.0);
        assert!(geos.is_empty());
    }

    #[test]
    fn test_mat3_mul_identity() {
        let a = parse_svg_transform("");
        let b = parse_svg_transform("");
        let r = mat3_mul(&a, &b);
        assert_eq!(r[0][0], 1.0);
        assert_eq!(r[1][2], 0.0);
    }

    #[test]
    fn test_geometry_to_svg_path_empty() {
        let geo = Geometry::new();
        assert!(geometry_to_svg_path(&geo, 100, 100).is_empty());
    }

    #[test]
    fn test_geometry_to_svg_path_move_line() {
        let mut geo = Geometry::new();
        geo.move_to(0.0, 1.0, 0.0);
        geo.line_to(1.0, 0.0, 0.0);
        let path = geometry_to_svg_path(&geo, 100, 200);
        assert!(path.starts_with("M 0.000 0.000"));
        assert!(path.contains("L 100.000 200.000"));
    }

    #[test]
    fn test_geometry_to_svg_path_y_flip() {
        let mut geo = Geometry::new();
        geo.move_to(0.0, 1.0, 0.0);
        let path = geometry_to_svg_path(&geo, 100, 100);
        assert!(path.contains("M 0.000 0.000"));
    }

    #[test]
    fn test_geometry_to_svg_path_arc() {
        let mut geo = Geometry::new();
        geo.move_to(0.5, 0.5, 0.0);
        geo.arc_to(1.0, 1.0, 0.0, 0.5, true, 0.0);
        let path = geometry_to_svg_path(&geo, 100, 100);
        assert!(path.contains("A 50.000 50.000 0 0 1 100.000 0.000"));
    }

    #[test]
    fn test_geometry_to_svg_path_bezier() {
        let mut geo = Geometry::new();
        geo.move_to(0.0, 0.0, 0.0);
        geo.bezier_to(((0.25, 0.5), (0.75, 0.5), (1.0, 1.0)), 0.0);
        let path = geometry_to_svg_path(&geo, 100, 100);
        assert!(path.contains("C 25.000 50.000 75.000 50.000 100.000 0.000"));
    }
}
