use prof_macros::prof;

use crate::dbg_log;
use crate::geo::shape::polygon::{
    get_polygon_heading_at, get_polygon_signed_area,
    get_polygons_closest_point, get_polygons_group_intersection,
    walk_polygon_vertices,
};
use crate::ops::cut::ClearedArea;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Point3D, Polygon};

/// Compute the inward offset position from a frontier vertex.
///
/// During normal cutting the tool centre sits at distance `radius -
/// advance` inside the cleared area from the boundary.  Placing the
/// tool directly on the frontier gives ~50% engagement instead of
/// the target, causing the solver to waste iterations steering back
/// to the correct depth.
fn offset_inward(pt: Point, normal: Point, radius: f64, advance: f64) -> Point {
    let offset = (radius - advance).max(0.0);
    pt - normal * offset
}

/// Iterate over sample points along a single polygon, calling `accept`
/// at each sample point.  Returns the first `Some` result from `accept`,
/// or `None` if no sample is accepted.
///
/// At each vertex (and optionally at sub-sampled positions along edges),
/// `accept(pos, heading)` is called where `pos` is the frontier point
/// and `heading` is the tangent direction of travel (in the walk
/// direction) at that point.
///
/// * `forward` — walk direction relative to the polygon's storage order
///   (the caller normalises winding externally).
/// * `sample_spacing` — sub-sample edges at this interval (mm).
///   Pass `0.0` to visit vertices only.
/// * `skip_closest` — if `true`, skip the first vertex (used when the
///   start position is the tool's current stall point whose own vertex
///   would trivially fail).
/// * `start_frac` — fractional offset along the edge starting at
///   `start_idx` (0.0 = at the vertex, 0.5 = mid-edge).  Used for
///   fractional-start walks; pass `0.0` to start at the vertex.
pub(crate) fn walk_polygon_samples<F>(
    poly: &Polygon,
    start_idx: usize,
    forward: bool,
    sample_spacing: f64,
    skip_closest: bool,
    start_frac: f64,
    mut accept: F,
) -> Option<ToolPose>
where
    F: FnMut(Point, f64) -> Option<ToolPose>,
{
    let n = poly.len();
    if n < 3 {
        return None;
    }

    // Handle fractional-start sample on the first edge before the
    // vertex walk (walk_polygon_vertices visits vertices only).
    if !skip_closest && start_frac > 0.0 {
        let next_idx = if forward {
            (start_idx + 1) % n
        } else {
            (start_idx + n - 1) % n
        };
        let edge = poly[next_idx] - poly[start_idx];
        if edge.length() >= 1e-9 {
            let s = poly[start_idx] + edge * start_frac;
            let heading = edge.y.atan2(edge.x);
            if let Some(r) = accept(s, heading) {
                return Some(r);
            }
        }
    }

    let mut is_first = true;
    walk_polygon_vertices(poly, start_idx, forward, |idx, _pt| {
        if skip_closest && is_first {
            is_first = false;
            return None;
        }
        is_first = false;

        let next_idx = if forward {
            (idx + 1) % n
        } else {
            (idx + n - 1) % n
        };
        let prev_idx = if forward {
            (idx + n - 1) % n
        } else {
            (idx + 1) % n
        };
        let pt = poly[idx];
        let nxt = poly[next_idx];
        let edge = nxt - pt;
        let elen = edge.length();

        // Heading: frontier tangent in the walk direction. Use the
        // outgoing edge direction; fall back to the incoming edge
        // for degenerate edges.
        let heading = if elen < 1e-9 {
            (pt.y - poly[prev_idx].y).atan2(pt.x - poly[prev_idx].x)
        } else {
            edge.y.atan2(edge.x)
        };

        // Sub-sample outgoing edge at sample_spacing intervals.
        if sample_spacing > 0.0 && elen > sample_spacing {
            let n_edge = (elen / sample_spacing).ceil() as usize;
            for si in 1..n_edge {
                let f = si as f64 / n_edge as f64;
                let s = pt + edge * f;
                if let Some(r) = accept(s, heading) {
                    return Some(r);
                }
            }
        }

        accept(pt, heading)
    })
}

/// Walk the frontier from `start`, using the heading direction to
/// decide whether to walk forward (CCW) or backward (CW).
///
/// Returns the first vertex whose outward cut-area probe by
/// `step_length` falls in `[min_cut_area, max_cut_area]`.
/// The returned position is offset inward by `radius - advance`.
///
/// Set `max_cut_area` to `f64::MAX` when only a lower bound matters.
#[prof]
pub fn search_frontier_engagement(
    cleared: &ClearedArea,
    start: ToolPose,
    radius: f64,
    step_length: f64,
    advance: f64,
    min_cut_area: f64,
    max_cut_area: f64,
) -> Option<ToolPose> {
    let frontier = cleared.frontier(0.001);
    if frontier.is_empty() {
        return None;
    }

    let envelope = cleared.envelope(radius);

    // ── Cleared-frontier search (primary: continue the spiral) ────
    // Walk the cleared-area frontier clipped to the envelope.  This is
    // the normal resume path: the tool continues cutting along the
    // existing frontier where engagement is in the target range.
    let polys = if envelope.is_empty() {
        frontier
    } else {
        get_polygons_group_intersection(&frontier, &envelope)
    };
    if !polys.is_empty() {
        let start_2d = crate::types::Point::new(start.pos.x, start.pos.y);
        if let Some((closest_poly_idx, _t, _closest_pt, _d2)) =
            get_polygons_closest_point(&polys, start_2d)
        {
            let poly = &polys[closest_poly_idx];
            let n = poly.len();
            if n >= 3 {
                let is_ccw = get_polygon_signed_area(poly) > 0.0;
                let start_idx = poly
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.distance_squared(start_2d)
                            .partial_cmp(&b.distance_squared(start_2d))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)?;

                let tangent = poly[(start_idx + 1) % n] - poly[start_idx];
                let heading_vec =
                    Point::new(start.heading.cos(), start.heading.sin());
                let forward = heading_vec.x * tangent.x
                    + heading_vec.y * tangent.y
                    >= 0.0;
                let actual_forward = if is_ccw { forward } else { !forward };

                let accept = |a: f64| a >= min_cut_area && a <= max_cut_area;
                if let Some(rp) = walk_polygon_samples(
                    poly,
                    start_idx,
                    actual_forward,
                    0.0,
                    true,
                    0.0,
                    |pt, heading| {
                        let normal_heading = get_polygon_heading_at(poly, pt);
                        let normal = Point::new(
                            normal_heading.cos(),
                            normal_heading.sin(),
                        );
                        let tangent = Point::new(heading.cos(), heading.sin());
                        let offset_pos =
                            offset_inward(pt, normal, radius, advance);
                        let probe = offset_pos + tangent * step_length;
                        let area = cleared.cut_area(offset_pos, probe, radius);
                        if accept(area) {
                            dbg_log!(
                                "  RESUME_SEARCH  frontier_pt=({:.3},{:.3})  \
                                 normal=({:.3},{:.3})  \
                                 offset_pos=({:.3},{:.3})  inward={:.3}  \
                                 probe_area={:.4}  heading={:.4}",
                                pt.x,
                                pt.y,
                                normal.x,
                                normal.y,
                                offset_pos.x,
                                offset_pos.y,
                                (radius - advance).max(0.0),
                                area,
                                heading,
                            );
                            Some(ToolPose {
                                pos: Point3D::new(
                                    offset_pos.x,
                                    offset_pos.y,
                                    start.pos.z,
                                ),
                                heading,
                            })
                        } else {
                            None
                        }
                    },
                ) {
                    return Some(rp);
                }
            }
        }
    }

    // ── Envelope-boundary search (rescue: finish wall bands) ──────
    // The frontier search failed — either because the frontier is empty
    // within the envelope, or because no frontier vertex has engagement
    // in the target range (the frontier has retreated from the envelope
    // edge and the remaining material is only in the wall band).  Walk
    // the tool-centre envelope boundary itself, sampling along each
    // edge for a point where probing along the tangent cuts material.
    // This places the tool *on* the envelope edge so the disk overhang
    // reaches the wall band.
    if !envelope.is_empty() {
        if let Some(rp) = walk_envelope_boundary(
            cleared,
            &envelope,
            start,
            radius,
            step_length,
            min_cut_area,
            max_cut_area,
        ) {
            return Some(rp);
        }
    }

    None
}

/// Walk the tool-centre envelope boundary, finding the vertex with the
/// best tangent-direction cut-area probe that satisfies `accept`.
///
/// Unlike [`search_frontier_engagement`], the tool is placed *on* the envelope edge
/// (zero inward offset) so the disk overhang reaches into the wall
/// band.  Rather than walking from the nearest vertex (which may be on
/// the opposite side of the pocket from the uncleared material), this
/// scans all envelope vertices and picks the one with the best
/// engagement — finding wall-band material wherever it is.  The score
/// is weighted toward nearer vertices to avoid gratuitous travel.
fn walk_envelope_boundary(
    cleared: &ClearedArea,
    envelope: &[Polygon],
    start: ToolPose,
    radius: f64,
    step_length: f64,
    min_cut_area: f64,
    max_cut_area: f64,
) -> Option<ToolPose> {
    let accept = |a: f64| a >= min_cut_area && a <= max_cut_area;

    // Score: prefer the *nearest* candidate with the *lowest* engagement
    // (just above `min_cut_area`).  Maximising engagement picks corners
    // with large uncleared quadrants — far from the stall point and
    // over-engaged — whereas a LostEngagement resume only needs *some*
    // material to bite into, ideally close to where the tool stalled.
    let mut best: Option<(f64, ToolPose)> = None;
    for poly in envelope {
        let n = poly.len();
        if n < 3 {
            continue;
        }

        // Sample along each edge, not just vertices.  The envelope may
        // be a coarse polygon (e.g. a rectangle with 4 vertices), but
        // the engagement varies along the edge — corners often have
        // very high engagement (large uncleared quadrants), while
        // mid-edge points have moderate engagement closer to target.
        let sample_spacing = step_length * 2.0;
        for idx in 0..n {
            let next_idx = (idx + 1) % n;
            let p0 = poly[idx];
            let p1 = poly[next_idx];
            let edge_len = (p1.x - p0.x).hypot(p1.y - p0.y);
            let n_samples = (edge_len / sample_spacing).ceil() as usize;
            if n_samples == 0 {
                continue;
            }
            for s in 0..=n_samples {
                let t = s as f64 / n_samples as f64;
                let pt = Point::new(
                    p0.x + t * (p1.x - p0.x),
                    p0.y + t * (p1.y - p0.y),
                );

                // Tangent along the edge (p0→p1 direction).
                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let len = dx.hypot(dy);
                if len < 1e-9 {
                    continue;
                }
                let heading = dy.atan2(dx);
                let tangent = Point::new(dx / len, dy / len);

                // Probe both directions along the edge.
                for sign in &[1.0, -1.0] {
                    let probe = pt + tangent * sign * step_length;
                    let area = cleared.cut_area(pt, probe, radius);
                    if accept(area) {
                        let dist2 = (pt.x - start.pos.x).powi(2)
                            + (pt.y - start.pos.y).powi(2);
                        // Lower score is better: a large engagement
                        // penalty keeps the tool out of over-engaged
                        // corners, and the distance penalty keeps the
                        // resume near the stall point.
                        let score = area * area + dist2 * 0.001;
                        let probe_heading = if *sign > 0.0 {
                            heading
                        } else {
                            heading + std::f64::consts::PI
                        };
                        if best.is_none_or(|(bs, _)| score < bs) {
                            best = Some((
                                score,
                                ToolPose {
                                    pos: Point3D::new(pt.x, pt.y, start.pos.z),
                                    heading: probe_heading,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    if let Some((_, _tp)) = &best {
        dbg_log!(
            "  ENVELOPE_SEARCH  → ({:.3},{:.3})  heading={:.4}",
            _tp.pos.x,
            _tp.pos.y,
            _tp.heading,
        );
    }
    best.map(|(_, tp)| tp)
}
