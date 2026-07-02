use prof_macros::prof;

use crate::dbg_log;
use crate::geo::shape::polygon::get_polygon_signed_area;
use crate::ops::assembly::adaptive::resume::{
    probe_step, ResumeCtx, ResumeStrategy,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::CutDirection;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

pub struct ResumeIsland;

impl ResumeStrategy for ResumeIsland {
    fn label(&self) -> &'static str {
        "island"
    }

    fn find_next(&self, ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
        frontier_hole_resume(ctx, tool)
    }
}

/// Walk the CW hole boundaries of [`ClearedArea::frontier`] and probe
/// for re-engagement.
///
/// The frontier returns the unioned cleared region clipped to stock.
/// Around islands (and any uncleared bulge adjacent to them) this
/// produces **CW hole polygons** whose outline is the exact boundary
/// between cleared and uncleared material.
///
/// The strategy walks each hole boundary, placing the tool centre on
/// the cleared side at distance `radius − advance` (the normal
/// engagement depth).  At each sample point the tool probes forward
/// along the travel tangent; the first position where the stepper
/// finds productive engagement becomes the resume target.
///
/// Travel direction respects `cut_direction`: for CCW cutting the
/// tool walks CW around holes (storage order), keeping uncut material
/// on the right — matching the stepper's one-sided deflection bounds.
#[prof]
fn frontier_hole_resume(ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
    let frontier = ctx.cleared.frontier(0.001);

    // Only CW holes represent island-adjacent uncleared material
    // inside the cleared region.  CCW outer polygons are handled by
    // ResumeBoundary / search_frontier_engagement.
    let holes: Vec<&Polygon> = frontier
        .iter()
        .filter(|p| p.len() >= 3 && get_polygon_signed_area(p) < -0.5)
        .collect();

    if holes.is_empty() {
        return None;
    }

    dbg_log!(
        "  ISLAND  {} frontier hole(s) ({} total frontier polys)",
        holes.len(),
        frontier.len(),
    );

    // Sort holes by distance from the tool position (nearest first).
    let mut order: Vec<usize> = (0..holes.len()).collect();
    order.sort_by(|&a, &b| {
        let da = min_dist2(holes[a], tool.pos);
        let db = min_dist2(holes[b], tool.pos);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });

    let radius = tool.radius;
    let advance = ctx.opts.advance;
    let offset = (radius - advance).max(0.0);
    let sample_spacing = ctx.opts.step_length;

    // CCW cut → walk holes in storage order (CW); CW cut → reverse.
    let walk_fwd = ctx.opts.cut_direction == CutDirection::Ccw;

    for &hi in &order {
        let hole = holes[hi];
        let n = hole.len();
        if n < 3 {
            continue;
        }

        // Start from the vertex closest to the tool position.
        let start_idx = hole
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.distance_squared(tool.pos)
                    .partial_cmp(&b.distance_squared(tool.pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        let Some(start_idx) = start_idx else {
            continue;
        };

        for vi in 0..n {
            let idx = if walk_fwd {
                (start_idx + vi) % n
            } else {
                (start_idx + n - vi) % n
            };
            let next = (idx + 1) % n;
            let p0 = hole[idx];
            let p1 = hole[next];
            let edge = p1 - p0;
            let edge_len = edge.length();
            if edge_len < 1e-12 {
                continue;
            }

            // For a CW hole walked in storage order, the cleared area
            // (exterior of the hole) is on the LEFT.  Left
            // perpendicular of edge (dx,dy) = (−dy, dx).
            let into_cleared = Point::new(-edge.y, edge.x) / edge_len;

            let n_samples =
                ((edge_len / sample_spacing).ceil() as usize).max(1);
            for si in 0..n_samples {
                let frac = (si as f64 + 0.5) / n_samples as f64;
                let on_frontier = p0 + edge * frac;

                // The frontier hole boundary may lie inside the
                // expanded-island exclusion zone (the tool disc
                // clears material past the envelope edge, so the
                // cleared boundary can be closer to the island than
                // the tool centre is allowed to go).  Ray-march from
                // the frontier point into the cleared area until the
                // centre is both inside the valid tool envelope AND
                // inside the cleared area (not buried in uncleared
                // material).
                let dist_to_cleared =
                    |p: Point| ctx.cleared.signed_boundary_distance(p.x, p.y);

                let mut centre = on_frontier + into_cleared * offset;
                let mut ok = point_in_valid_area(centre, ctx.valid_tool_area)
                    && dist_to_cleared(centre) <= 0.0;

                if !ok {
                    let step = ctx.opts.step_length * 0.25;
                    let max_steps = (radius * 4.0 / step).ceil() as usize;
                    for s in 1..=max_steps {
                        let candidate = on_frontier
                            + into_cleared * (offset + s as f64 * step);
                        if !point_in_valid_area(candidate, ctx.valid_tool_area)
                        {
                            continue;
                        }
                        if dist_to_cleared(candidate) > 0.0 {
                            continue;
                        }
                        centre = candidate;
                        ok = true;
                        break;
                    }
                    if !ok {
                        continue;
                    }
                }

                // Heading along the travel tangent.
                let heading = if walk_fwd {
                    edge.y.atan2(edge.x)
                } else {
                    (-edge.y).atan2(-edge.x)
                };

                if let Some(probed) = probe_step(ctx, radius, centre, heading) {
                    dbg_log!(
                        "  ISLAND  resume=({:.3},{:.3})  hdg={:.1}  \
                         hole={}  idx={}/{}",
                        centre.x,
                        centre.y,
                        heading.to_degrees(),
                        hi,
                        idx,
                        n,
                    );
                    return Some(probed);
                }
            }
        }
    }

    dbg_log!("  ISLAND  no engagement found on any frontier hole");
    None
}

/// Squared distance from `pt` to the nearest vertex of `poly`.
fn min_dist2(poly: &Polygon, pt: Point) -> f64 {
    poly.iter()
        .map(|p| p.distance_squared(pt))
        .fold(f64::MAX, f64::min)
}
