use prof_macros::prof;

use crate::dbg_log;
use crate::geo::shape::polygon::{
    get_polygon_heading_at, get_polygon_signed_area,
    get_polygons_closest_point, get_polygons_group_intersection,
};
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

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

/// Walk along the cleared-area frontier, clipped to the tool-centre
/// envelope, returning the first vertex whose outward cut-area probe
/// satisfies `accept`.
///
/// `forward` is relative to the polygon's natural storage order (not
/// to a global CCW/CW convention).  Use
/// [`search_frontier_engagement`] which chooses the direction from the
/// start heading.
///
/// The returned position is offset inward (into the cleared area)
/// by `radius - advance` so the tool starts at the correct
/// engagement depth rather than directly on the boundary.
#[allow(clippy::too_many_arguments)]
#[prof]
fn walk_frontier(
    cleared: &ClearedArea,
    start_pos: Point,
    radius: f64,
    step_length: f64,
    advance: f64,
    forward: bool,
    skip_closest: bool,
    mut accept: impl FnMut(f64) -> bool,
) -> Option<ToolPose> {
    let frontier = cleared.frontier(0.001);
    if frontier.is_empty() {
        return None;
    }

    let polys = {
        let envelope = cleared.envelope(radius);
        if envelope.is_empty() {
            frontier
        } else {
            get_polygons_group_intersection(&frontier, &envelope)
        }
    };
    if polys.is_empty() {
        return None;
    }

    let (closest_poly_idx, _t, _closest_pt, _d2) =
        get_polygons_closest_point(&polys, start_pos)?;
    let poly = &polys[closest_poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }

    // Normalise to CCW so "forward" always means increasing index.
    let is_ccw = get_polygon_signed_area(poly) > 0.0;
    let actual_forward = if is_ccw { forward } else { !forward };

    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(start_pos)
                .partial_cmp(&b.distance_squared(start_pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)?;

    let first = if skip_closest { 1 } else { 0 };
    for offset in first..n {
        let idx = if actual_forward {
            (start_idx + offset) % n
        } else {
            (start_idx + n - offset) % n
        };
        let pt = poly[idx];

        let normal_heading = get_polygon_heading_at(poly, pt);
        let normal = Point::new(normal_heading.cos(), normal_heading.sin());

        // Compute the travel direction (tangent) at this vertex.
        let prev_idx = if actual_forward {
            (idx + n - 1) % n
        } else {
            (idx + 1) % n
        };
        let prev_pt = poly[prev_idx];
        let heading = (pt.y - prev_pt.y).atan2(pt.x - prev_pt.x);
        let tangent = Point::new(heading.cos(), heading.sin());

        // Probe from the offset position along the travel direction,
        // measuring the actual cut area the tool would experience on
        // its first step.  This ensures the reengagement point has
        // approximately the target engagement, not just "any" material.
        let offset_pos = offset_inward(pt, normal, radius, advance);
        let probe = offset_pos + tangent * step_length;
        let area = cleared.cut_area(offset_pos, probe, radius);
        if accept(area) {
            dbg_log!(
                "  RESUME_SEARCH  frontier_pt=({:.3},{:.3})  \
                 normal=({:.3},{:.3})  offset_pos=({:.3},{:.3})  \
                 inward={:.3}  probe_area={:.4}  heading={:.4}",
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
            return Some(ToolPose {
                pos: offset_pos,
                heading,
            });
        }
    }
    None
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
        if let Some((closest_poly_idx, _t, _closest_pt, _d2)) =
            get_polygons_closest_point(&polys, start.pos)
        {
            let poly = &polys[closest_poly_idx];
            let n = poly.len();
            if n >= 3 {
                let start_idx = poly
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.distance_squared(start.pos)
                            .partial_cmp(&b.distance_squared(start.pos))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i);

                if let Some(si) = start_idx {
                    let heading_vec =
                        Point::new(start.heading.cos(), start.heading.sin());
                    let tangent = poly[(si + 1) % n] - poly[si];
                    let forward = heading_vec.x * tangent.x
                        + heading_vec.y * tangent.y
                        >= 0.0;

                    if let Some(rp) = walk_frontier(
                        cleared,
                        start.pos,
                        radius,
                        step_length,
                        advance,
                        forward,
                        true,
                        |a| a >= min_cut_area && a <= max_cut_area,
                    ) {
                        return Some(rp);
                    }
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
/// Unlike [`walk_frontier`], the tool is placed *on* the envelope edge
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
                                    pos: pt,
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

/// SegmentResume: walk forward from `segment_start` along
/// `cut_direction` until probing finds a position where the tool can
/// re-engage uncut material.
///
/// The tool was cutting a segment that started at `segment_start`.
/// When it stalls, we jump back to the segment start and walk
/// forward.  At each position, probe straight ahead and at several
/// outward angles.  The first position where any probe finds
/// engagement above `min_cut_area` becomes the resume target.
///
/// Positions outside `valid_tool_area` are skipped — the tool can
/// never operate there.
#[prof]
#[allow(clippy::too_many_arguments)]
pub fn search_reengagement(
    cleared: &ClearedArea,
    segment_start: Point,
    cut_direction: Point,
    radius: f64,
    step_length: f64,
    _advance: f64,
    min_cut_area: f64,
    valid_tool_area: &[Polygon],
) -> Option<ToolPose> {
    if cleared.fragments().is_empty() {
        return None;
    }

    let tangent = cut_direction;

    // Probe directions relative to the cutting tangent.
    const PROBE_ANGLES: [f64; 7] = [
        0.0,                          // straight
        std::f64::consts::FRAC_PI_6,  // +30°
        -std::f64::consts::FRAC_PI_6, // -30°
        std::f64::consts::FRAC_PI_3,  // +60°
        -std::f64::consts::FRAC_PI_3, // -60°
        std::f64::consts::FRAC_PI_2,  // +90°
        -std::f64::consts::FRAC_PI_2, // -90°
    ];

    // Walk FORWARD from the segment start along cut_direction.
    // At each position, probe 7 angles for engagement.
    let max_steps = 2000;
    let mut pos = segment_start;

    for step in 0..max_steps {
        // Skip positions outside valid_tool_area.
        if !point_in_valid_area(pos, valid_tool_area) {
            pos += tangent * step_length;
            continue;
        }

        for &angle in &PROBE_ANGLES {
            let dir = Point::new(
                tangent.x * angle.cos() - tangent.y * angle.sin(),
                tangent.x * angle.sin() + tangent.y * angle.cos(),
            );
            let probe = pos + dir * step_length;
            let area = cleared.cut_area(pos, probe, radius);
            if area >= min_cut_area {
                let heading = dir.y.atan2(dir.x);
                dbg_log!(
                    "  REENGAGE  step={}  pos=({:.3},{:.3})  \
                     area={:.4}  angle={:.1}°  heading={:.4}",
                    step,
                    pos.x,
                    pos.y,
                    area,
                    angle.to_degrees(),
                    heading,
                );
                return Some(ToolPose { pos, heading });
            }
        }
        pos += tangent * step_length;
    }

    dbg_log!("  REENGAGE  no engagement within {} steps", max_steps);
    None
}
