//! Sweep-line disk-increment area computation.
//!
//! Computes the area of ``disk(c2) − disk(c1) − fragments``, intersected
//! with ``valid_area``, split into total / left-of-step portions.
//! Used by [`ClearedArea`](super::cleared_area::ClearedArea) to measure fresh
//! material a tool encounters when stepping forward.

use crate::geo::shape::circle::{
    get_circle_circle_intersections, get_line_circle_intersections,
};
use crate::geo::shape::polygon::{
    does_polygon_enclose_circle, get_polygon_bounds, rotate_polygon,
};
use crate::geo::shape::rect::do_rects_intersect;
use crate::types::{Point, Polygon, Rect};
use prof_macros::prof;

/// Rotate a point by the pre-computed (cos, sin) of an angle.
fn rotate_crescent(p: Point, ca: f64, sa: f64) -> Point {
    Point::new(ca * p.x - sa * p.y, sa * p.x + ca * p.y)
}

/// Compute the rotation that maps the step vector `c1 → c2` onto the +y
/// axis, returning `(cos, sin, angle_deg)` (`angle_deg` is for
/// [`rotate_polygon`]).
fn step_rotation(c1: Point, c2: Point) -> (f64, f64, f64) {
    let angle = std::f64::consts::FRAC_PI_2 - (c2.y - c1.y).atan2(c2.x - c1.x);
    (angle.cos(), angle.sin(), angle.to_degrees())
}

/// Rotated, bounds-filtered inputs ready for the vertical sweep.
///
/// `polygons` holds fragments first (subtracted / "negative" shapes) then
/// valid-area polygons (intersected / "positive" shapes).
struct SweepContext {
    c1: Point,
    c2: Point,
    radius: f64,
    polygons: Vec<Vec<Point>>,
    /// Number of fragment polygons at the front of [`polygons`].
    num_frags: usize,
    /// Number of valid-area polygons after the fragments.
    num_valid: usize,
}

/// Rotate `c1`/`c2` so the step is vertical, then collect the fragments
/// and valid-area polygons that interact with the `c2` disk.
///
/// Returns `None` for any of the short-circuit cases that yield zero
/// area (coincident centres, a fragment fully enclosing the disk, or a
/// non-empty `valid_area` that misses the disk entirely).
fn prepare_sweep(
    c1: Point,
    c2: Point,
    radius: f64,
    fragments: &[Polygon],
    valid_area: &[Polygon],
) -> Option<SweepContext> {
    let dist = (c2 - c1).length();
    if dist < 1e-9 {
        return None;
    }

    let (ca, sa, angle_deg) = step_rotation(c1, c2);
    let c1 = rotate_crescent(c1, ca, sa);
    let c2 = rotate_crescent(c2, ca, sa);
    let c2_bb =
        Rect::new(c2.x - radius, c2.y - radius, c2.x + radius, c2.y + radius);

    let mut polygons: Vec<Vec<Point>> = Vec::new();

    for frag in fragments {
        if frag.len() < 3 {
            continue;
        }
        let rotated = rotate_polygon(frag, angle_deg);
        let bounds = get_polygon_bounds(&rotated);
        if !do_rects_intersect(bounds, c2_bb) {
            continue;
        }
        if does_polygon_enclose_circle(c2, radius, &rotated) {
            return None;
        }
        polygons.push(rotated);
    }
    let num_frags = polygons.len();

    let mut valid_overlaps_disk = false;
    for valid in valid_area {
        if valid.len() < 3 {
            continue;
        }
        let rotated = rotate_polygon(valid, angle_deg);
        let bounds = get_polygon_bounds(&rotated);
        if !do_rects_intersect(bounds, c2_bb) {
            continue;
        }
        valid_overlaps_disk = true;
        // A valid polygon clips the result to its interior whether or not
        // it encloses the whole disk — an enclosing polygon is simply a
        // harmless no-op (it can only relax the constraint).
        polygons.push(rotated);
    }

    if !valid_area.is_empty() && !valid_overlaps_disk {
        return None;
    }
    let num_valid = polygons.len() - num_frags;

    Some(SweepContext {
        c1,
        c2,
        radius,
        polygons,
        num_frags,
        num_valid,
    })
}

/// Build the sorted, de-duplicated set of x-coordinates at which the
/// sweep topology can change: polygon vertices, every edge×circle and
/// circle×circle intersection, and the `c2` disk extents.
fn build_xcoords(cx: &SweepContext) -> Vec<f64> {
    let c1 = cx.c1;
    let c2 = cx.c2;
    let radius = cx.radius;
    let polygons = &cx.polygons;

    let mut xs = Vec::new();
    for poly in polygons {
        for p in poly {
            xs.push(p.x);
        }
        let n = poly.len();
        for i in 0..n {
            let p0 = poly[i];
            let p1 = poly[(i + 1) % n];
            for &pt in get_line_circle_intersections(p0, p1, c1, radius).iter()
            {
                xs.push(pt.x);
            }
            for &pt in get_line_circle_intersections(p0, p1, c2, radius).iter()
            {
                xs.push(pt.x);
            }
        }
    }

    for &pt in get_circle_circle_intersections(c1, radius, c2, radius).iter() {
        xs.push(pt.x);
    }

    let xmin = c2.x - radius;
    let xmax = c2.x + radius;
    xs.push(c1.x - radius);
    xs.push(c1.x + radius);
    xs.push(xmin);
    xs.push(xmax);
    xs.push(c2.x);

    xs.retain(|&x| x >= xmin && x <= xmax);
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    xs
}

/// A vertical crossing event: `(y, shape_index, part_index)`.
/// `part_index` is the polygon edge index, or `0`/`1` for a circle's
/// upper/lower arc.
type Crossing = (f64, usize, usize);

/// Collect every boundary crossing at `xtest`, tagged by shape and part.
/// Shapes are indexed as: fragments + valid polys (`0..total_polys`),
/// then `c2`, then `c1`.
fn slab_crossings(cx: &SweepContext, xtest: f64) -> Vec<Crossing> {
    let c1 = cx.c1;
    let c2 = cx.c2;
    let radius = cx.radius;
    let polygons = &cx.polygons;
    let total_polys = polygons.len();
    let circles = [c2, c1];

    let mut ys: Vec<Crossing> = Vec::new();
    for (ip, poly) in polygons.iter().enumerate() {
        let n = poly.len();
        for ie in 0..n {
            let p0 = poly[ie];
            let p1 = poly[(ie + 1) % n];
            if p0.x.min(p1.x) < xtest && p0.x.max(p1.x) > xtest {
                let t = (xtest - p0.x) / (p1.x - p0.x);
                let y = p0.y + t * (p1.y - p0.y);
                ys.push((y, ip, ie));
            }
        }
    }

    for (ic, &c) in circles.iter().enumerate() {
        let dx = (xtest - c.x).abs();
        if dx < radius {
            let dy = (radius * radius - dx * dx).sqrt();
            let sh = total_polys + ic;
            ys.push((c.y + dy, sh, 0));
            ys.push((c.y - dy, sh, 1));
        }
    }
    ys
}

/// Trapezoid area under a straight edge between `x0..x1` (linear
/// interpolation of the edge's `y`).
fn edge_slab_area(p0: Point, p1: Point, x0: f64, x1: f64) -> f64 {
    let t0 = (x0 - p0.x) / (p1.x - p0.x);
    let t1 = (x1 - p0.x) / (p1.x - p0.x);
    let y0 = p0.y + t0 * (p1.y - p0.y);
    let y1 = p0.y + t1 * (p1.y - p0.y);
    (y0 + y1) * 0.5 * (x1 - x0)
}

/// Signed area contribution of a circular-arc slab between `x0..x1`
/// around centre `c`.  `cs = +1` for the upper arc, `-1` for the lower.
fn arc_slab_area(c: Point, radius: f64, x0: f64, x1: f64, cs: f64) -> f64 {
    let clamp = |v: f64| v.clamp(-1.0, 1.0);

    let phi0 = clamp((x0 - c.x) / radius).acos() * cs;
    let phi1 = clamp((x1 - c.x) / radius).acos() * cs;
    let area_sector = radius * radius * 0.5 * (phi1 - phi0).abs();

    let y0 =
        c.y + cs * (radius * radius - (x0 - c.x) * (x0 - c.x)).max(0.0).sqrt();
    let y1 =
        c.y + cs * (radius * radius - (x1 - c.x) * (x1 - c.x)).max(0.0).sqrt();
    let tbase = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
    let tmidx = (x0 + x1) * 0.5;
    let tmidy = (y0 + y1) * 0.5;
    let th = ((tmidx - c.x).powi(2) + (tmidy - c.y).powi(2))
        .max(0.0)
        .sqrt();
    let area_segment = area_sector - tbase * th * 0.5;

    let area_trapezoid = (x1 - x0) * (y0 + y1) * 0.5;
    cs * area_segment + area_trapezoid
}

/// Run the vertical sweep over the slabs defined by `xs`, accumulating
/// `(total, left)` area where `left` is the portion with `x < c2.x`.
fn sweep_area(cx: &SweepContext, xs: &[f64]) -> (f64, f64) {
    let c2 = cx.c2;
    let num_frags = cx.num_frags;
    let num_valid = cx.num_valid;
    let polygons = &cx.polygons;
    let total_polys = polygons.len();
    let circles = [cx.c2, cx.c1];
    let radius = cx.radius;
    let nshapes = total_polys + circles.len();

    let mut total = 0.0f64;
    let mut left = 0.0f64;

    for ix in 0..xs.len() - 1 {
        let x0 = xs[ix];
        let x1 = xs[ix + 1];
        if x0 >= x1 {
            continue;
        }
        let xtest = (x0 + x1) * 0.5;

        let mut ys = slab_crossings(cx, xtest);
        ys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // All shapes start outside at y→−∞.  Outside-positive shapes:
        // c2 (+1) and each non-enclosing valid_area (+1 each).
        // Negative shapes (fragments, c1) contribute 0 when outside.
        let mut outside = vec![true; nshapes];
        let mut outside_count: i32 = 1 + num_valid as i32;

        for &(_y, ishape, ipart) in &ys {
            let prev_outside = outside[ishape];
            let prev_count = outside_count;
            outside[ishape] = !outside[ishape];

            // Negative shapes (fragments, c1) invert the count delta so
            // entering them moves AWAY from the result (count ↑).
            let is_negative = ishape < num_frags || ishape == total_polys + 1;
            outside_count += if is_negative {
                if prev_outside {
                    1
                } else {
                    -1
                }
            } else if prev_outside {
                -1
            } else {
                1
            };

            let sign: f64 = if prev_outside { -1.0 } else { 1.0 };
            // Negative shapes contribute with opposite sign — their
            // boundary is traversed CW (holes) in the result.
            let sign = if is_negative { -sign } else { sign };

            if outside_count == 0 || prev_count == 0 {
                let da = if ishape < total_polys {
                    let poly = &polygons[ishape];
                    let n = poly.len();
                    let p0 = poly[ipart];
                    let p1 = poly[(ipart + 1) % n];
                    edge_slab_area(p0, p1, x0, x1)
                } else {
                    let c = circles[ishape - total_polys];
                    let cs = if ipart == 0 { 1.0 } else { -1.0 };
                    arc_slab_area(c, radius, x0, x1, cs)
                };
                total += sign * da;
                if xtest < c2.x {
                    left += sign * da;
                }
            }
        }
    }

    (total, left)
}

/// Compute the area of the region inside a disk at `c2` but outside
/// the disk at `c1`, minus any overlap with `fragments`, intersected
/// with `valid_area`.  Returns `(total, left)` where `left` is the
/// portion to the left of the step vector `c1 → c2` in the rotated
/// frame.
///
/// `fragments` are closed polygons subtracted from the increment.
/// `valid_area` constrains the result to its interior (intersection).
/// Pass `&[]` for either argument to skip that constraint.
#[prof]
pub fn cut_area(
    c1: Point,
    c2: Point,
    radius: f64,
    fragments: &[Polygon],
    valid_area: &[Polygon],
) -> (f64, f64) {
    let cx = match prepare_sweep(c1, c2, radius, fragments, valid_area) {
        Some(cx) => cx,
        None => return (0.0, 0.0),
    };
    let xs = build_xcoords(&cx);
    if xs.len() < 2 {
        return (0.0, 0.0);
    }
    sweep_area(&cx, &xs)
}
