use std::f64::consts::PI;

use crate::geo::types::{CubicBezier, Point};

/// Center-parameterisation of an arc from the SVG endpoint
/// parameterisation used by the path `A` command.
pub struct ArcCenter {
    pub center: Point,
    pub radii: (f64, f64),
    pub phi: f64,
    pub start_angle: f64,
    pub sweep: f64,
}

/// Convert an SVG endpoint-parameterised elliptical arc to center form.
///
/// Applies the correction factor for radii that are too small, and
/// resolves the center and sweep according to the `large_arc` and `sweep`
/// flags. Returns `None` for degenerate arcs.
#[allow(clippy::too_many_arguments)]
pub fn get_arc_center(
    x1: f64,
    y1: f64,
    rx: f64,
    ry: f64,
    phi_deg: f64,
    large_arc: bool,
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
    let cp = phi.cos();
    let sp = phi.sin();
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
    let f = if den > 0.0 {
        (num.max(0.0) / den).sqrt()
    } else {
        0.0
    };
    let f = if large_arc == sweep { -f } else { f };
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
        center: Point::new(cx, cy),
        radii: (rx, ry),
        phi,
        start_angle: sa,
        sweep: sw,
    })
}

/// Convert an arc segment (≤ 90°) to a cubic bezier.
pub fn convert_arc_segment_to_bezier(
    center: Point,
    rx: f64,
    ry: f64,
    phi: f64,
    t1: f64,
    t2: f64,
) -> CubicBezier {
    let k = 4.0 / 3.0 * ((t2 - t1) / 4.0).tan();
    let cp = phi.cos();
    let sp = phi.sin();
    let c1 = t1.cos();
    let s1 = t1.sin();
    let c2 = t2.cos();
    let s2 = t2.sin();
    let sx = center.x + rx * c1 * cp - ry * s1 * sp;
    let sy = center.y + rx * c1 * sp + ry * s1 * cp;
    let ex = center.x + rx * c2 * cp - ry * s2 * sp;
    let ey = center.y + rx * c2 * sp + ry * s2 * cp;
    let c1x = sx - k * (rx * s1 * cp + ry * c1 * sp);
    let c1y = sy - k * (rx * s1 * sp - ry * c1 * cp);
    let c2x = ex + k * (rx * s2 * cp + ry * c2 * sp);
    let c2y = ey + k * (rx * s2 * sp - ry * c2 * cp);
    CubicBezier(
        Point::new(sx, sy),
        Point::new(c1x, c1y),
        Point::new(c2x, c2y),
        Point::new(ex, ey),
    )
}

/// Split an arc into ≤90° cubic bezier segments.
pub fn convert_arc_to_beziers(ac: &ArcCenter) -> Vec<CubicBezier> {
    if !ac.sweep.is_finite() || !ac.start_angle.is_finite() {
        return vec![];
    }
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
        segs.push(convert_arc_segment_to_bezier(
            ac.center, ac.radii.0, ac.radii.1, ac.phi, t, next,
        ));
        if next == end {
            break;
        }
        t = next;
    }
    segs
}
