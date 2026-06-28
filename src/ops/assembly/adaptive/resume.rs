//! Stall / re-engagement logic for [`super::adaptive_clearing`].
//!
//! When the forward-stepping solver stalls (lost engagement, boundary
//! hit, or stuck oscillation), the helpers in this module reposition the
//! tool: first via a frontier-engagement search, then via the pocket's
//! Medial Axis Transform (routed through cleared territory and shortened
//! with [`build_smoothed_path`]), and finally by jumping to the nearest
//! remaining-region centroid.

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::smooth::build_smoothed_path;
use crate::geo::shape::polygon::compute_polygon_bounds;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::ops::container::Ops;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::search_frontier_engagement;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

use super::tool::Tool;
use super::AdaptiveClearingOptions;

// ── Resume constants ─────────────────────────────────────────────────

/// Maximum number of resume / re-engagement attempts before giving up.
pub(super) const MAX_RESUMES: usize = 500;

// ── Helpers ──────────────────────────────────────────────────────────

/// Smooth and shorten a cleared-territory travel path.
///
/// `raw` is a waypoint list (e.g. from the MAT) that is known to stay
/// inside cleared territory.  It is fed through [`build_smoothed_path`]
/// against the uncleared obstacles (`islands` ∪ `remaining`) so that
/// redundant intermediate waypoints are shortcut away and any sharp
/// turns are rounded — keeping the tool disk clear of fresh stock while
/// minimising rapid travel length.
///
/// `from` is the tool's current position and is preserved verbatim as
/// the path's first point so the smoothing kernel never moves it.
pub fn smooth_travel_path(
    from: Point,
    raw: &[Point],
    islands: &[Polygon],
    remaining: &[Polygon],
    clearance: f64,
) -> Vec<Point> {
    if raw.is_empty() {
        return vec![from];
    }
    let last = raw[raw.len() - 1];
    // build_smoothed_path prepends `from` and appends `last`; the
    // intermediate waypoints are the MAT path minus its (already
    // supplied) endpoints to avoid duplicating them.
    let waypoints: Vec<Point> = if raw.len() > 2 {
        raw[1..raw.len() - 1].to_vec()
    } else {
        Vec::new()
    };
    let mut obstacles: Vec<Polygon> = islands.to_vec();
    obstacles.extend_from_slice(remaining);
    let obs_bounds = compute_polygon_bounds(&obstacles);
    let smoothed = build_smoothed_path(
        from,
        last,
        &waypoints,
        &obstacles,
        &obs_bounds,
        clearance,
        120,
    );
    if smoothed.is_empty() {
        vec![from, last]
    } else {
        smoothed
    }
}

/// Emit a safe resume travel from `from` to `to`.
///
/// When a Medial Axis is available, route the travel through cleared
/// territory by walking the MAT tree between the two endpoints' nearest
/// cleared nodes, then shorten and smooth the resulting waypoint list
/// via [`smooth_travel_path`] / [`build_smoothed_path`] so redundant
/// hops are shortcut away and sharp turns are rounded — producing the
/// shortest collision-free rapid path instead of the raw tree walk.
/// When no MAT is available (or no cleared path exists), fall back to a
/// single `move_to` — preserving the previous behaviour.
pub fn emit_resume_travel(
    ops: &mut Ops,
    cleared: &ClearedArea,
    mat: Option<&MedialAxis>,
    from: Point,
    to: Point,
    opts: &AdaptiveClearingOptions,
) {
    let z = opts.cut_z + 0.5;
    if let Some(axis) = mat {
        let fragments = cleared.fragments();
        if !fragments.is_empty() {
            if let Some(path) = axis.path_between_cleared(from, to, fragments) {
                if path.len() >= 2 {
                    let remaining = cleared.remaining();
                    let smoothed = smooth_travel_path(
                        from,
                        &path,
                        &opts.islands,
                        &remaining,
                        opts.radius,
                    );
                    for wp in &smoothed {
                        ops.move_to(wp.x, wp.y, z, None);
                    }
                    return;
                }
            }
        }
    }
    ops.move_to(to.x, to.y, z, None);
}

/// Try to recover after the tool stalls or is detected as stuck.
///
/// 1. Backward wall-hugging resume via [`search_frontier_engagement`].
/// 2. Fallback: travel to the nearest uncut frontier via
///    [`ClearedArea::remaining`].
///
/// Returns `true` if the tool was repositioned (caller should update
/// `prev_pos` and continue the main loop).
#[allow(clippy::too_many_arguments)]
pub fn try_resume(
    cleared: &mut ClearedArea,
    ops: &mut Ops,
    tool: &mut Tool,
    opts: &AdaptiveClearingOptions,
    valid_tool_area: &[Polygon],
    target_area_pd: f64,
    _max_def: f64,
    _target_eng: f64,
    min_cut_area: f64,
    mat: Option<&MedialAxis>,
    last_resume_area: f64,
    _segment_start: Point,
) -> bool {
    let max_cut_area = opts.step_length * target_area_pd * 1.5;
    dbg_log!(
        "RESUME  from=({:.3},{:.3})  heading={:.4}  R={:.1}  \
         advance={:.3}  step_len={:.3}  max_cut_area={:.4}  \
         last_area={:.3}  cur_area={:.3}",
        tool.pos.x,
        tool.pos.y,
        tool.heading,
        opts.radius,
        opts.advance,
        opts.step_length,
        max_cut_area,
        last_resume_area,
        cleared.total_area(),
    );

    // ── Primary: frontier engagement search ──────────────────────
    // Skip the frontier search when the previous resume produced no
    // cleared-area growth (the tool stalled again immediately).  This
    // breaks the degenerate bounce between nearby frontier vertices
    // against an island wall: instead of re-trying the same local
    // frontier, fall straight through to the MAT walk that routes
    // around the island to fresh material.
    let area_grew = cleared.total_area() > last_resume_area + 1e-9;
    if area_grew {
        if let Some(rp) = search_frontier_engagement(
            cleared,
            ToolPose {
                pos: tool.pos,
                heading: tool.heading,
            },
            opts.radius,
            opts.step_length,
            opts.advance,
            min_cut_area,
            max_cut_area,
        ) {
            let moved = (rp.pos.x - tool.pos.x).powi(2)
                + (rp.pos.y - tool.pos.y).powi(2);
            if moved > (opts.step_length * 0.25).powi(2) {
                dbg_log!(
                    "  RESUME  path=search_frontier  → ({:.3},{:.3})  \
                     heading={:.4}",
                    rp.pos.x,
                    rp.pos.y,
                    rp.heading,
                );
                // The frontier search returns only the destination pose;
                // emit a *safe* travel that routes through cleared
                // territory (via the MAT when available) rather than a
                // straight-line jump that could cross an island.
                emit_resume_travel(ops, cleared, mat, tool.pos, rp.pos, opts);
                tool.pos = rp.pos;
                tool.heading = rp.heading;
                tool.reset_gyro();
                return true;
            }
            dbg_log!(
                "  RESUME  frontier_search returned no-progress \
                 ({:.3},{:.3}) ≈ current; skipping to fallback",
                rp.pos.x,
                rp.pos.y,
            );
        }
    } else {
        dbg_log!(
            "  RESUME  skipping frontier_search (no area growth since \
             last resume)"
        );
    }

    // ── Fallback: Medial Axis walk ─────────────────────────────────
    // Route from the stuck tool position along the medial axis of the
    // pocket (through cleared territory) to the nearest uncleared MAT
    // node, then resume cutting there.  The MAT path is shortened and
    // smoothed via [`smooth_travel_path`] so the tool traces the
    // shortest collision-free route through already-cut material rather
    // than following every tree node.
    if let Some(axis) = mat {
        if let Some((path, heading)) =
            mat_resume_target(axis, cleared, tool.pos, valid_tool_area)
        {
            dbg_log!(
                "  RESUME  path=mat_walk  {} waypoints  → ({:.3},{:.3})  \
                 heading={:.4}",
                path.len(),
                path[path.len() - 1].x,
                path[path.len() - 1].y,
                heading,
            );
            // Travel to the MAT target through cleared territory.
            // `mat_resume_target` already returned a path restricted to
            // cleared MAT nodes; `smooth_travel_path` shortcuts redundant
            // waypoints (collision-checked against the islands and
            // remaining stock) and rounds sharp turns so the tool follows
            // the shortest safe route instead of the raw tree walk.
            let remaining = cleared.remaining();
            let smoothed = smooth_travel_path(
                tool.pos,
                &path,
                &opts.islands,
                &remaining,
                opts.radius,
            );
            let dest = smoothed[smoothed.len() - 1];
            for wp in &smoothed {
                ops.move_to(wp.x, wp.y, opts.cut_z + 0.5, None);
            }
            tool.pos = dest;
            tool.heading = heading;
            tool.reset_gyro();
            return true;
        }
    }

    dbg_log!("  RESUME  path=centroid_fallback");

    // ── Last-resort: nearest valid-area centroid of a remaining region
    // (legacy behaviour, retained for when MAT is unavailable).
    let remaining = cleared.remaining();
    let mut best_centroid: Option<(f64, Point)> = None;
    for poly in &remaining {
        if poly.len() < 3 {
            continue;
        }
        let area = get_polygon_area(poly).abs();
        if area < 0.3 {
            continue;
        }
        let centroid = get_polygon_centroid(poly);
        if !point_in_valid_area(centroid, valid_tool_area) {
            continue;
        }
        let dist = (centroid.x - tool.pos.x).powi(2)
            + (centroid.y - tool.pos.y).powi(2);
        if best_centroid.is_none_or(|(bd, _)| dist < bd) {
            best_centroid = Some((dist, centroid));
        }
    }

    if let Some((_, centroid)) = best_centroid {
        emit_resume_travel(ops, cleared, mat, tool.pos, centroid, opts);
        tool.pos = centroid;
        tool.heading = 0.0;
        tool.reset_gyro();
        return true;
    }

    false
}

/// Pick a resume target by walking the Medial Axis Transform of the
/// pocket from the tool's current position to the nearest uncleared MAT
/// node.
///
/// The MAT is a tree of maximally-inscribed-circle centres.  Each node
/// carries a clearance radius.  A node is "cleared" when it lies inside
/// one of the cleared fragments; "uncleared" nodes mark pockets of fresh
/// material still to be cut.
///
/// The strategy:
///  1. Build a cleared/uncleared mask over the MAT nodes.
///  2. Find the nearest *uncleared* node to the tool whose clearance
///     circle fits inside the valid tool-centre region (so the tool can
///     actually sit there).
///  3. Route from the tool's nearest cleared node to that target along
///     the MAT tree, staying within cleared nodes (the tool travels
///     through already-cut material at cut depth).
///  4. Return the full travel path (MAT nodes through cleared territory,
///     plus the final step into the uncleared target) and a heading that
///     faces fresh material so the solver immediately engages.
pub fn mat_resume_target(
    axis: &MedialAxis,
    cleared: &ClearedArea,
    tool_pos: Point,
    valid_tool_area: &[Polygon],
) -> Option<(Vec<Point>, f64)> {
    let fragments = cleared.fragments();
    if fragments.is_empty() {
        return None;
    }
    let is_cleared = axis.build_cleared_mask(fragments);

    // Nearest MAT node to the tool (the routing start).
    let from_idx = axis.nearest_node(tool_pos)?;
    // If the tool is already on an uncleared node there is nothing to
    // route to — engagement should have been available.
    if !is_cleared[from_idx] {
        return None;
    }

    // Search for the nearest uncleared node (BFS over the MAT tree) that
    // lies inside the valid tool-centre region.  BFS over the tree from
    // `from_idx` visits nodes in order of tree-distance, which is a good
    // proxy for travel distance through cleared territory.
    let target_idx = nearest_uncleared_node(axis, from_idx, &is_cleared)
        .filter(|&idx| {
            point_in_valid_area(axis.nodes[idx].point, valid_tool_area)
        })?;

    // Route through cleared nodes only.  The path ends at the cleared
    // ancestor of the (uncleared) target — the last cleared position
    // adjacent to fresh material.
    let mut path =
        axis.path_between_indices_cleared(from_idx, target_idx, &is_cleared)?;
    if path.len() < 2 {
        return None;
    }
    let last_cleared = path[path.len() - 1];
    // Append the uncleared target node as the final waypoint — the tool
    // travels the cleared path, then takes one final step into fresh
    // material.  Heading faces from the last cleared node toward the
    // target so the solver immediately engages.
    let target_pt = axis.nodes[target_idx].point;
    path.push(target_pt);
    let heading =
        (target_pt.y - last_cleared.y).atan2(target_pt.x - last_cleared.x);
    Some((path, heading))
}

/// BFS over the Medial Axis tree from `start`, returning the index of the
/// nearest (fewest hops) node that is **not** cleared.
pub fn nearest_uncleared_node(
    axis: &MedialAxis,
    start: usize,
    is_cleared: &[bool],
) -> Option<usize> {
    use std::collections::VecDeque;
    let n = axis.nodes.len();
    if n == 0 {
        return None;
    }
    let mut visited = vec![false; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    visited[start] = true;
    queue.push_back(start);
    // Build adjacency from the edge list.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in &axis.edges {
        adj[a].push(b);
        adj[b].push(a);
    }
    while let Some(idx) = queue.pop_front() {
        if idx != start && !is_cleared[idx] {
            return Some(idx);
        }
        for &nb in &adj[idx] {
            if !visited[nb] {
                visited[nb] = true;
                queue.push_back(nb);
            }
        }
    }
    None
}
