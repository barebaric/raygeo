use std::f64::consts::PI;

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::geometry::Geometry;
use glam::{DMat3, DVec3};

use crate::geo::math::{mat3_det2x2, mat3_transform};
use crate::types::Command;

type BezierSeg = ((f64, f64), (f64, f64), (f64, f64), (f64, f64));

const BORDER_SIZE: f64 = 2.0;

fn parse_coords(s: &str) -> Vec<f64> {
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

fn parse_path_tokens(d: &str) -> Vec<(char, String)> {
    let mut tokens = Vec::new();
    let mut cmd = 0u8;
    let mut args = String::new();
    for ch in d.chars() {
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
                | 'Q'
                | 'q'
                | 'T'
                | 't'
                | 'S'
                | 's'
                | 'A'
                | 'a'
                | 'Z'
                | 'z'
        ) {
            if cmd != 0 {
                tokens.push((cmd as char, args.clone()));
            }
            cmd = ch as u8;
            args.clear();
        } else {
            args.push(ch);
        }
    }
    if cmd != 0 {
        tokens.push((cmd as char, args));
    }
    tokens
}

/// Apply the affine transform + border/scale to a single SVG coordinate.
fn txfm_pt(px: f64, py: f64, m: DMat3, sx: f64, sy: f64) -> (f64, f64) {
    let (tx, ty) = mat3_transform(m, px, py);
    ((tx - BORDER_SIZE) / sx, (ty - BORDER_SIZE) / sy)
}

/// Transform an arc's center offset and determine if direction flips.
fn txfm_arc(
    start: (f64, f64),
    center: (f64, f64),
    clockwise: bool,
    m: DMat3,
    sx: f64,
    sy: f64,
) -> (f64, f64, f64, f64, bool) {
    let (tsx, tsy) = txfm_pt(start.0, start.1, m, sx, sy);
    let (tex, tey) = txfm_pt(center.0, center.1, m, sx, sy);
    let ti = tex - tsx;
    let tj = tey - tsy;
    let det = mat3_det2x2(m);
    let cw = if (det < 0.0) != (sx * sy < 0.0) {
        !clockwise
    } else {
        clockwise
    };
    (tex, tey, ti, tj, cw)
}

/// Sample N evenly-spaced points on a cubic bezier (used by C, S, Q, T).
fn flatten_cubic(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    n: usize,
) -> Vec<(f64, f64)> {
    let mut pts = Vec::with_capacity(n);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let u = 1.0 - t;
        let x = u * u * u * p0.0
            + 3.0 * u * u * t * p1.0
            + 3.0 * u * t * t * p2.0
            + t * t * t * p3.0;
        let y = u * u * u * p0.1
            + 3.0 * u * u * t * p1.1
            + 3.0 * u * t * t * p2.1
            + t * t * t * p3.1;
        pts.push((x, y));
    }
    pts
}

/// Result of converting an SVG endpoint-parameterised arc to center form.
struct ArcCenter {
    cx: f64,
    cy: f64,
    start_angle: f64,
    sweep: f64,
    rx: f64,
    ry: f64,
    phi: f64,
}

#[allow(clippy::too_many_arguments)]
fn svg_arc_center(
    x1: f64,
    y1: f64,
    rx: f64,
    ry: f64,
    phi_deg: f64,
    large: bool,
    sweep: bool,
    x2: f64,
    y2: f64,
) -> Option<ArcCenter> {
    if rx.abs() < 1e-9
        || ry.abs() < 1e-9
        || (x1 - x2).abs() < 1e-9 && (y1 - y2).abs() < 1e-9
    {
        return None;
    }
    let mut rx = rx.abs();
    let mut ry = ry.abs();
    let phi = phi_deg.to_radians();
    let (cp, sp) = phi.sin_cos();
    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    let x1p = cp * dx + sp * dy;
    let y1p = -sp * dx + cp * dy;
    let lambda = x1p * x1p / (rx * rx) + y1p * y1p / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let x1p2 = x1p * x1p;
    let y1p2 = y1p * y1p;
    let num = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2;
    let den = rx2 * y1p2 + ry2 * x1p2;
    let f = if den > 0.0 { (num / den).sqrt() } else { 0.0 };
    let f = if large == sweep { -f } else { f };
    let cxp = f * rx * y1p / ry;
    let cyp = -f * ry * x1p / rx;
    let cx = cp * cxp - sp * cyp + (x1 + x2) / 2.0;
    let cy = sp * cxp + cp * cyp + (y1 + y2) / 2.0;
    let sa = ((y1p - cyp) / ry).atan2((x1p - cxp) / rx);
    let ea = ((-y1p - cyp) / ry).atan2((-x1p - cxp) / rx);
    let mut sw = ea - sa;
    if sweep {
        if sw < 0.0 {
            sw += 2.0 * PI;
        }
    } else if sw > 0.0 {
        sw -= 2.0 * PI;
    }
    Some(ArcCenter {
        cx,
        cy,
        start_angle: sa,
        sweep: sw,
        rx,
        ry,
        phi,
    })
}

/// Convert an elliptical arc segment (≤ 90°) to a cubic bezier.
fn arc_seg_to_bezier(
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    phi: f64,
    t1: f64,
    t2: f64,
) -> BezierSeg {
    let k = 4.0 / 3.0 * ((t2 - t1) / 4.0).tan();
    let (cp, sp) = phi.sin_cos();
    let (c1, s1) = t1.sin_cos();
    let (c2, s2) = t2.sin_cos();
    let sx = cx + rx * c1 * cp - ry * s1 * sp;
    let sy = cy + rx * c1 * sp + ry * s1 * cp;
    let ex = cx + rx * c2 * cp - ry * s2 * sp;
    let ey = cy + rx * c2 * sp + ry * s2 * cp;
    let c1x = sx - k * (rx * s1 * cp + ry * c1 * sp);
    let c1y = sy - k * (rx * s1 * sp - ry * c1 * cp);
    let c2x = ex + k * (rx * s2 * cp + ry * c2 * sp);
    let c2y = ey + k * (rx * s2 * sp - ry * c2 * cp);
    ((sx, sy), (c1x, c1y), (c2x, c2y), (ex, ey))
}

/// Split an elliptical arc into ≤90° bezier segments.
fn elliptical_arc_to_beziers(ac: &ArcCenter) -> Vec<BezierSeg> {
    let mut segs = Vec::new();
    let step = if ac.sweep > 0.0 { PI / 2.0 } else { -PI / 2.0 };
    let end = ac.start_angle + ac.sweep;
    let mut t = ac.start_angle;
    loop {
        let next = if (ac.sweep > 0.0 && t + step >= end)
            || (ac.sweep < 0.0 && t + step <= end)
        {
            end
        } else {
            t + step
        };
        segs.push(arc_seg_to_bezier(
            ac.cx, ac.cy, ac.rx, ac.ry, ac.phi, t, next,
        ));
        if next == end {
            break;
        }
        t = next;
    }
    segs
}

fn push_line(geo: &mut Geometry, x: f64, y: f64, m: DMat3, sx: f64, sy: f64) {
    let (tx, ty) = txfm_pt(x, y, m, sx, sy);
    geo.line_to(tx, ty, 0.0);
}

#[allow(clippy::too_many_arguments)]
fn handle_move(
    tokens: &[(char, String)],
    i: &mut usize,
    geo: &mut Option<Geometry>,
    geos: &mut Vec<Geometry>,
    pos: &mut (f64, f64),
    sub_start: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
    m: DMat3,
    sx: f64,
    sy: f64,
) {
    *prev_c2 = None;
    *prev_q = None;
    if let Some(g) = geo.take() {
        if !g.is_empty() {
            geos.push(g);
        }
    }
    let (cmd, ref args) = tokens[*i];
    let coords = parse_coords(args);
    let mut g = Geometry::new();
    if coords.len() >= 2 {
        if cmd == 'm' {
            pos.0 += coords[0];
            pos.1 += coords[1];
        } else {
            pos.0 = coords[0];
            pos.1 = coords[1];
        }
        *sub_start = *pos;
        let (tx, ty) = txfm_pt(pos.0, pos.1, m, sx, sy);
        g.move_to(tx, ty, 0.0);
        for j in (2..coords.len()).step_by(2) {
            if j + 1 < coords.len() {
                if cmd == 'm' {
                    pos.0 += coords[j];
                    pos.1 += coords[j + 1];
                } else {
                    pos.0 = coords[j];
                    pos.1 = coords[j + 1];
                }
                push_line(&mut g, pos.0, pos.1, m, sx, sy);
            }
        }
    }
    *geo = Some(g);
}

#[allow(clippy::too_many_arguments)]
fn handle_linelike(
    tokens: &[(char, String)],
    i: &mut usize,
    geo: &mut Option<Geometry>,
    pos: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
    m: DMat3,
    sx: f64,
    sy: f64,
) {
    *prev_c2 = None;
    *prev_q = None;
    let (cmd, ref args) = tokens[*i];
    let coords = parse_coords(args);
    let g = match geo.as_mut() {
        Some(g) => g,
        None => return,
    };
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
        _ => {}
    }
    push_line(g, pos.0, pos.1, m, sx, sy);
}

#[allow(clippy::too_many_arguments)]
fn handle_cubic(
    tokens: &[(char, String)],
    i: &mut usize,
    geo: &mut Option<Geometry>,
    pos: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
    m: DMat3,
    sx: f64,
    sy: f64,
) {
    *prev_q = None;
    let (cmd, ref args) = tokens[*i];
    let coords = parse_coords(args);
    let g = match geo.as_mut() {
        Some(g) => g,
        None => return,
    };
    for j in (0..coords.len()).step_by(6) {
        if j + 5 >= coords.len() {
            break;
        }
        let (c1x, c1y, c2x, c2y, ex, ey) = if cmd == 'C' {
            (
                coords[j],
                coords[j + 1],
                coords[j + 2],
                coords[j + 3],
                coords[j + 4],
                coords[j + 5],
            )
        } else {
            (
                pos.0 + coords[j],
                pos.1 + coords[j + 1],
                pos.0 + coords[j + 2],
                pos.1 + coords[j + 3],
                pos.0 + coords[j + 4],
                pos.1 + coords[j + 5],
            )
        };
        *prev_c2 = Some((c2x, c2y));
        for (px, py) in
            flatten_cubic(*pos, (c1x, c1y), (c2x, c2y), (ex, ey), 20)
        {
            push_line(g, px, py, m, sx, sy);
        }
        *pos = (ex, ey);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_smooth_cubic(
    tokens: &[(char, String)],
    i: &mut usize,
    geo: &mut Option<Geometry>,
    pos: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
    m: DMat3,
    sx: f64,
    sy: f64,
) {
    *prev_q = None;
    let (cmd, ref args) = tokens[*i];
    let coords = parse_coords(args);
    let g = match geo.as_mut() {
        Some(g) => g,
        None => return,
    };
    for j in (0..coords.len()).step_by(4) {
        if j + 3 >= coords.len() {
            break;
        }
        let (c2x, c2y, ex, ey) = if cmd == 'S' {
            (coords[j], coords[j + 1], coords[j + 2], coords[j + 3])
        } else {
            (
                pos.0 + coords[j],
                pos.1 + coords[j + 1],
                pos.0 + coords[j + 2],
                pos.1 + coords[j + 3],
            )
        };
        let (c1x, c1y) = match *prev_c2 {
            Some((px, py)) => (2.0 * pos.0 - px, 2.0 * pos.1 - py),
            None => *pos,
        };
        *prev_c2 = Some((c2x, c2y));
        for (px, py) in
            flatten_cubic(*pos, (c1x, c1y), (c2x, c2y), (ex, ey), 20)
        {
            push_line(g, px, py, m, sx, sy);
        }
        *pos = (ex, ey);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_quad(
    tokens: &[(char, String)],
    i: &mut usize,
    geo: &mut Option<Geometry>,
    pos: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
    m: DMat3,
    sx: f64,
    sy: f64,
) {
    *prev_c2 = None;
    let (cmd, ref args) = tokens[*i];
    let coords = parse_coords(args);
    let g = match geo.as_mut() {
        Some(g) => g,
        None => return,
    };
    for j in (0..coords.len()).step_by(4) {
        if j + 3 >= coords.len() {
            break;
        }
        let (qx, qy, ex, ey) = if cmd == 'Q' {
            (coords[j], coords[j + 1], coords[j + 2], coords[j + 3])
        } else {
            (
                pos.0 + coords[j],
                pos.1 + coords[j + 1],
                pos.0 + coords[j + 2],
                pos.1 + coords[j + 3],
            )
        };
        *prev_q = Some((qx, qy));
        let c1x = pos.0 + 2.0 / 3.0 * (qx - pos.0);
        let c1y = pos.1 + 2.0 / 3.0 * (qy - pos.1);
        let c2x = qx + 1.0 / 3.0 * (ex - qx);
        let c2y = qy + 1.0 / 3.0 * (ey - qy);
        for (px, py) in
            flatten_cubic(*pos, (c1x, c1y), (c2x, c2y), (ex, ey), 20)
        {
            push_line(g, px, py, m, sx, sy);
        }
        *pos = (ex, ey);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_smooth_quad(
    tokens: &[(char, String)],
    i: &mut usize,
    geo: &mut Option<Geometry>,
    pos: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
    m: DMat3,
    sx: f64,
    sy: f64,
) {
    *prev_c2 = None;
    let (cmd, ref args) = tokens[*i];
    let coords = parse_coords(args);
    let g = match geo.as_mut() {
        Some(g) => g,
        None => return,
    };
    for j in (0..coords.len()).step_by(2) {
        if j + 1 >= coords.len() {
            break;
        }
        let (ex, ey) = if cmd == 'T' {
            (coords[j], coords[j + 1])
        } else {
            (pos.0 + coords[j], pos.1 + coords[j + 1])
        };
        let (qx, qy) = match *prev_q {
            Some((px, py)) => (2.0 * pos.0 - px, 2.0 * pos.1 - py),
            None => *pos,
        };
        *prev_q = Some((qx, qy));
        let c1x = pos.0 + 2.0 / 3.0 * (qx - pos.0);
        let c1y = pos.1 + 2.0 / 3.0 * (qy - pos.1);
        let c2x = qx + 1.0 / 3.0 * (ex - qx);
        let c2y = qy + 1.0 / 3.0 * (ey - qy);
        for (px, py) in
            flatten_cubic(*pos, (c1x, c1y), (c2x, c2y), (ex, ey), 20)
        {
            push_line(g, px, py, m, sx, sy);
        }
        *pos = (ex, ey);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_arc(
    tokens: &[(char, String)],
    i: &mut usize,
    geo: &mut Option<Geometry>,
    pos: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
    m: DMat3,
    sx: f64,
    sy: f64,
) {
    *prev_c2 = None;
    *prev_q = None;
    let (cmd, ref args) = tokens[*i];
    let coords = parse_coords(args);
    let g = match geo.as_mut() {
        Some(g) => g,
        None => return,
    };
    for j in (0..coords.len()).step_by(7) {
        if j + 6 >= coords.len() {
            break;
        }
        let rx = coords[j];
        let ry = coords[j + 1];
        let x_rot = coords[j + 2];
        let large = coords[j + 3] != 0.0;
        let sweep = coords[j + 4] != 0.0;
        let (ex, ey) = if cmd == 'A' {
            (coords[j + 5], coords[j + 6])
        } else {
            (pos.0 + coords[j + 5], pos.1 + coords[j + 6])
        };

        let ac = match svg_arc_center(
            pos.0, pos.1, rx, ry, x_rot, large, sweep, ex, ey,
        ) {
            Some(ac) => ac,
            None => {
                *pos = (ex, ey);
                continue;
            }
        };

        let is_circular = (ac.rx - ac.ry).abs() / ac.rx.max(ac.ry) < 1e-3;

        if is_circular {
            let (tex, tey, ti, tj, cw) =
                txfm_arc(*pos, (ac.cx, ac.cy), sweep, m, sx, sy);
            g.arc_to(tex, tey, ti, tj, cw, 0.0);
        } else {
            for (_, (c1x, c1y), (c2x, c2y), (ex, ey)) in
                elliptical_arc_to_beziers(&ac)
            {
                let (tc1x, tc1y) = txfm_pt(c1x, c1y, m, sx, sy);
                let (tc2x, tc2y) = txfm_pt(c2x, c2y, m, sx, sy);
                let (tex, tey) = txfm_pt(ex, ey, m, sx, sy);
                g.bezier_to(
                    crate::types::Point3D(tc1x, tc1y, 0.0),
                    crate::types::Point3D(tc2x, tc2y, 0.0),
                    crate::types::Point3D(tex, tey, 0.0),
                );
            }
        }
        *pos = (ex, ey);
    }
}

fn handle_close(
    geo: &mut Option<Geometry>,
    pos: &mut (f64, f64),
    sub_start: &mut (f64, f64),
    prev_c2: &mut Option<(f64, f64)>,
    prev_q: &mut Option<(f64, f64)>,
) {
    *prev_c2 = None;
    *prev_q = None;
    if let Some(g) = geo.as_mut() {
        g.close_path();
    }
    *pos = *sub_start;
}

/// Parse an SVG path `d` attribute into a list of geometries.
///
/// Supports M/m, L/l, H/h, V/v, C/c, S/s, Q/q, T/t, A/a, Z/z.
/// Cubic and quadratic curves are flattened to line segments.
/// Circular arcs (rx ≈ ry) are preserved as native Arc commands;
/// elliptical arcs are approximated with cubic beziers.
pub fn parse_svg_path_data(
    path_data: &str,
    transform: DMat3,
    scale_x: f64,
    scale_y: f64,
) -> RaygeoResult<Vec<Geometry>> {
    let tokens = parse_path_tokens(path_data);
    if tokens.is_empty() && !path_data.trim().is_empty() {
        return Err(RaygeoError::SvgInvalidPath(
            "no recognized SVG path commands found".into(),
        ));
    }

    let mut geometries = Vec::new();
    let mut current_geo: Option<Geometry> = None;
    let mut pos = (0.0, 0.0);
    let mut subpath_start = (0.0, 0.0);
    let mut prev_c2: Option<(f64, f64)> = None;
    let mut prev_q: Option<(f64, f64)> = None;

    let mut i = 0;
    while i < tokens.len() {
        let cmd = tokens[i].0;
        match cmd {
            'M' | 'm' => handle_move(
                &tokens,
                &mut i,
                &mut current_geo,
                &mut geometries,
                &mut pos,
                &mut subpath_start,
                &mut prev_c2,
                &mut prev_q,
                transform,
                scale_x,
                scale_y,
            ),
            'L' | 'l' | 'H' | 'h' | 'V' | 'v' => handle_linelike(
                &tokens,
                &mut i,
                &mut current_geo,
                &mut pos,
                &mut prev_c2,
                &mut prev_q,
                transform,
                scale_x,
                scale_y,
            ),
            'C' | 'c' => handle_cubic(
                &tokens,
                &mut i,
                &mut current_geo,
                &mut pos,
                &mut prev_c2,
                &mut prev_q,
                transform,
                scale_x,
                scale_y,
            ),
            'S' | 's' => handle_smooth_cubic(
                &tokens,
                &mut i,
                &mut current_geo,
                &mut pos,
                &mut prev_c2,
                &mut prev_q,
                transform,
                scale_x,
                scale_y,
            ),
            'Q' | 'q' => handle_quad(
                &tokens,
                &mut i,
                &mut current_geo,
                &mut pos,
                &mut prev_c2,
                &mut prev_q,
                transform,
                scale_x,
                scale_y,
            ),
            'T' | 't' => handle_smooth_quad(
                &tokens,
                &mut i,
                &mut current_geo,
                &mut pos,
                &mut prev_c2,
                &mut prev_q,
                transform,
                scale_x,
                scale_y,
            ),
            'A' | 'a' => handle_arc(
                &tokens,
                &mut i,
                &mut current_geo,
                &mut pos,
                &mut prev_c2,
                &mut prev_q,
                transform,
                scale_x,
                scale_y,
            ),
            'Z' | 'z' => handle_close(
                &mut current_geo,
                &mut pos,
                &mut subpath_start,
                &mut prev_c2,
                &mut prev_q,
            ),
            _ => {}
        }
        i += 1;
    }

    if let Some(geo) = current_geo.take() {
        if !geo.is_empty() {
            geometries.push(geo);
        }
    }

    Ok(geometries)
}

fn translate_m(coords: &[f64]) -> DMat3 {
    let mut m = DMat3::IDENTITY;
    if !coords.is_empty() {
        let tx = coords[0];
        let ty = if coords.len() > 1 { coords[1] } else { 0.0 };
        m = DMat3::from_cols(
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(tx, ty, 1.0),
        );
    }
    m
}

fn scale_m(coords: &[f64]) -> DMat3 {
    if coords.is_empty() {
        return DMat3::IDENTITY;
    }
    let sx = coords[0];
    let sy = if coords.len() > 1 { coords[1] } else { sx };
    DMat3::from_cols(
        DVec3::new(sx, 0.0, 0.0),
        DVec3::new(0.0, sy, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    )
}

fn rotate_m(coords: &[f64]) -> DMat3 {
    if coords.is_empty() {
        return DMat3::IDENTITY;
    }
    let a = coords[0].to_radians();
    let (c, s) = a.sin_cos();
    if coords.len() >= 3 {
        let (cx, cy) = (coords[1], coords[2]);
        DMat3::from_cols(
            DVec3::new(c, s, 0.0),
            DVec3::new(-s, c, 0.0),
            DVec3::new(cx - cx * c + cy * s, cy - cx * s - cy * c, 1.0),
        )
    } else {
        DMat3::from_cols(
            DVec3::new(c, s, 0.0),
            DVec3::new(-s, c, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    }
}

fn skew_x_m(coords: &[f64]) -> DMat3 {
    if let Some(&a) = coords.first() {
        let t = a.to_radians().tan();
        DMat3::from_cols(
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(t, 1.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    } else {
        DMat3::IDENTITY
    }
}

fn skew_y_m(coords: &[f64]) -> DMat3 {
    if let Some(&a) = coords.first() {
        let t = a.to_radians().tan();
        DMat3::from_cols(
            DVec3::new(1.0, t, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    } else {
        DMat3::IDENTITY
    }
}

fn affine_m(coords: &[f64]) -> DMat3 {
    if coords.len() < 6 {
        return DMat3::IDENTITY;
    }
    DMat3::from_cols(
        DVec3::new(coords[0], coords[1], 0.0),
        DVec3::new(coords[2], coords[3], 0.0),
        DVec3::new(coords[4], coords[5], 1.0),
    )
}

/// Parse an SVG `transform` attribute into a 3×3 affine matrix.
///
/// Supports: `translate`, `scale`, `rotate`, `skewX`, `skewY`, `matrix`.
/// Multiple functions can be chained (e.g. `translate(10,20) scale(2)`).
pub fn parse_svg_transform(transform_str: &str) -> DMat3 {
    let mut matrix = DMat3::IDENTITY;
    if transform_str.is_empty() {
        return matrix;
    }
    let mut remaining = transform_str.trim();
    while !remaining.is_empty() {
        let name_start = match remaining.find(|c: char| c.is_ascii_alphabetic())
        {
            Some(i) => i,
            None => break,
        };
        remaining = &remaining[name_start..];
        let name_end = remaining
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(remaining.len());
        let name = &remaining[..name_end];
        remaining = remaining[name_end..].trim_start();
        if remaining.starts_with('(') {
            if let Some(close) = remaining.find(')') {
                let coords = parse_coords(&remaining[1..close]);
                let fm = match name {
                    "translate" => translate_m(&coords),
                    "scale" => scale_m(&coords),
                    "rotate" => rotate_m(&coords),
                    "skewX" => skew_x_m(&coords),
                    "skewY" => skew_y_m(&coords),
                    "matrix" => affine_m(&coords),
                    _ => DMat3::IDENTITY,
                };
                matrix *= fm;
                remaining = remaining[close + 1..].trim_start();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    matrix
}

fn attr_f64(node: &roxmltree::Node, name: &str) -> Option<f64> {
    node.attribute(name).and_then(|v| v.parse::<f64>().ok())
}

fn is_hidden(node: &roxmltree::Node) -> bool {
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

fn rect_to_d(node: &roxmltree::Node) -> Option<String> {
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

fn circle_to_d(node: &roxmltree::Node) -> Option<String> {
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

fn ellipse_to_d(node: &roxmltree::Node) -> Option<String> {
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

fn line_to_d(node: &roxmltree::Node) -> Option<String> {
    let x1 = attr_f64(node, "x1").unwrap_or(0.0);
    let y1 = attr_f64(node, "y1").unwrap_or(0.0);
    let x2 = attr_f64(node, "x2").unwrap_or(0.0);
    let y2 = attr_f64(node, "y2").unwrap_or(0.0);
    Some(format!("M {} {} L {} {}", x1, y1, x2, y2))
}

fn poly_to_d(node: &roxmltree::Node) -> Option<String> {
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
            let identity = DMat3::IDENTITY;
            traverse(doc.root_element(), identity, &mut geos, scale_x, scale_y);
            geos
        }
        Err(_) => Vec::new(),
    };
    Ok(all_geometries)
}

fn traverse(
    node: roxmltree::Node,
    parent_tfm: DMat3,
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
                    parse_svg_path_data(d, combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "rect" => {
            if let Some(d) = rect_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "circle" => {
            if let Some(d) = circle_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "ellipse" => {
            if let Some(d) = ellipse_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "line" => {
            if let Some(d) = line_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, combined, scale_x, scale_y)
                {
                    geos.extend(g);
                }
            }
        }
        "polyline" | "polygon" => {
            if let Some(d) = poly_to_d(&node) {
                if let Ok(g) =
                    parse_svg_path_data(&d, combined, scale_x, scale_y)
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
    for cmd in data {
        let (ex, ey, _) =
            (cmd.end_point().0, cmd.end_point().1, cmd.end_point().2);
        let x = ex * w;
        let y = h * (1.0 - ey);
        match cmd {
            Command::Move { .. } => parts.push(format!("M {x:.3} {y:.3}")),
            Command::Line { .. } => parts.push(format!("L {x:.3} {y:.3}")),
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => {
                let r = center_offset.0.hypot(center_offset.1);
                let sweep = if *clockwise { 1 } else { 0 };
                parts.push(format!(
                    "A {:.3} {:.3} 0 0 {sweep} {x:.3} {y:.3}",
                    r * w,
                    r * h
                ));
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let (c1x, c1y, _) = (control1.0, control1.1, control1.2);
                let (c2x, c2y, _) = (control2.0, control2.1, control2.2);
                parts.push(format!(
                    "C {:.3} {:.3} {:.3} {:.3} {x:.3} {y:.3}",
                    c1x * w,
                    h * (1.0 - c1y),
                    c2x * w,
                    h * (1.0 - c2y),
                ));
            }
        }
    }
    parts.join(" ")
}
