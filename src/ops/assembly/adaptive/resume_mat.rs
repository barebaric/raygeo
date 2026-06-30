use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_closest_point,
};
use crate::ops::assembly::adaptive::resume::{
    probe_step, ResumeCtx, ResumeStrategy, WALL_PROXIMITY,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::CutDirection;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

fn can_step(
    rp: ToolPose,
    step_length: f64,
    valid_tool_area: &[Polygon],
) -> bool {
    let next = Point::new(
        rp.pos.x + rp.heading.cos() * step_length,
        rp.pos.y + rp.heading.sin() * step_length,
    );
    point_in_valid_area(next, valid_tool_area)
}

pub struct ResumeMat;

impl ResumeStrategy for ResumeMat {
    const NAME: &'static str = "ResumeMat";

    fn find_next(&self, ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
        let axis = ctx.mat?;
        let fragments = ctx.cleared.fragments();
        if fragments.is_empty() {
            return None;
        }
        let is_cleared = axis.build_cleared_mask(fragments);
        let from_idx = axis.nearest_node(tool.pos)?;
        if !is_cleared[from_idx] {
            return None;
        }
        let crossings = find_all_mat_crossings(axis, from_idx, &is_cleared);
        dbg_log!(
            "  MAT_RESUME  crossings={}  from_idx={}",
            crossings.len(),
            from_idx,
        );
        for crossing_idx in crossings {
            if let Some(rp) = mat_resume_from_crossing(
                axis,
                ctx.cleared,
                crossing_idx,
                ctx.opts.cut_direction,
                tool.radius,
                &ctx.opts.pocket_boundary,
                &ctx.opts.islands,
            ) {
                if let Some(probed) =
                    probe_step(ctx, tool.radius, rp.pos, rp.heading)
                {
                    return Some(probed);
                }
            }
        }
        None
    }
}

/// Pick a resume target by trying every MAT crossing in turn.
///
/// For each crossing, walks the cleared-area frontier backward until
/// hitting the pocket boundary, then offsets inward by `radius`.
/// Returns the first crossing whose resume position passes the
/// `can_step` valid-area check.
#[allow(clippy::too_many_arguments)]
pub fn mat_resume_target(
    axis: &MedialAxis,
    cleared: &ClearedArea,
    tool: &Tool,
    cut_direction: CutDirection,
    step_length: f64,
    pocket_boundary: &[Point],
    islands: &[Polygon],
    valid_tool_area: &[Polygon],
) -> Option<ToolPose> {
    let fragments = cleared.fragments();
    if fragments.is_empty() {
        return None;
    }
    let is_cleared = axis.build_cleared_mask(fragments);

    let from_idx = axis.nearest_node(tool.pos)?;
    if !is_cleared[from_idx] {
        return None;
    }

    let crossings = find_all_mat_crossings(axis, from_idx, &is_cleared);
    dbg_log!(
        "  MAT_RESUME  crossings={}  from_idx={}",
        crossings.len(),
        from_idx,
    );

    for crossing_idx in crossings {
        if let Some(rp) = mat_resume_from_crossing(
            axis,
            cleared,
            crossing_idx,
            cut_direction,
            tool.radius,
            pocket_boundary,
            islands,
        ) {
            if can_step(rp, step_length, valid_tool_area) {
                return Some(rp);
            }
        }
    }

    None
}

/// Try to find a resume point from a single MAT crossing.
///
/// 1. `P_CROSS` = crossing node position (marks where fresh material
///    begins on the MAT).
/// 2. Walk the cleared-area frontier **backward** (opposite to the
///    cutting rotational direction) from the vertex nearest `P_CROSS`
///    until hitting the pocket boundary (within `WALL_PROXIMITY`) or
///    completing a full loop (→ fail, try next crossing).
/// 3. Place the tool centre at `radius` from the nearest wall point,
///    along the wall→cleared direction.  The heading is the frontier
///    tangent in the cutting direction.
pub fn mat_resume_from_crossing(
    axis: &MedialAxis,
    cleared: &ClearedArea,
    crossing_idx: usize,
    cut_direction: CutDirection,
    radius: f64,
    pocket_boundary: &[Point],
    islands: &[Polygon],
) -> Option<ToolPose> {
    let p_cross = axis.nodes[crossing_idx].point;

    let polys = cleared.frontier(0.001);
    if polys.is_empty() {
        return None;
    }

    let (poly_idx, _, _, _) = get_polygons_closest_point(&polys, p_cross)?;
    let poly = &polys[poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }

    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(p_cross)
                .partial_cmp(&b.distance_squared(p_cross))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)?;

    let wall_polys: Vec<Polygon> = std::iter::once(pocket_boundary.to_vec())
        .chain(islands.iter().cloned())
        .collect();
    let wallprox2 = WALL_PROXIMITY * WALL_PROXIMITY;

    // Walk BACKWARD along the frontier (opposite to the cutting
    // rotational direction) until hitting the pocket wall.  The
    // cutting direction in index space depends on the polygon
    // winding: increasing index is geometric CCW on a CCW-wound poly
    // and geometric CW on a CW-wound poly.
    let poly_ccw = get_polygon_signed_area(poly) > 0.0;
    let cut_increasing = (cut_direction == CutDirection::Ccw) == poly_ccw;
    let walk_increasing = !cut_increasing;

    let mut hit_idx: Option<usize> = None;
    for offset in 0..n {
        let idx = if walk_increasing {
            (start_idx + offset) % n
        } else {
            (start_idx + n - offset) % n
        };
        let pt = poly[idx];

        if let Some((_, _, _, d2)) = get_polygons_closest_point(&wall_polys, pt)
        {
            if d2 <= wallprox2 {
                hit_idx = Some(idx);
                break;
            }
        }
    }

    let hit_idx = hit_idx?;
    let p_hit = poly[hit_idx];

    let (_, _, wall_pt, _) = get_polygons_closest_point(&wall_polys, p_hit)?;
    let wall_to_hit = p_hit - wall_pt;
    let wall_dist = wall_to_hit.length();
    let inward = if wall_dist > 1e-9 {
        wall_to_hit / wall_dist
    } else {
        let cx: f64 = poly.iter().map(|p| p.x).sum::<f64>() / n as f64;
        let cy: f64 = poly.iter().map(|p| p.y).sum::<f64>() / n as f64;
        let dir = Point::new(cx - wall_pt.x, cy - wall_pt.y);
        let dlen = dir.length();
        if dlen > 1e-9 {
            dir / dlen
        } else {
            Point::new(0.0, 1.0)
        }
    };
    let p_resume = wall_pt + inward * radius;

    // Heading: frontier tangent at hit_idx in the cutting direction.
    let tangent_pt = if cut_increasing {
        poly[(hit_idx + 1) % n]
    } else {
        poly[(hit_idx + n - 1) % n]
    };
    let t = tangent_pt - poly[hit_idx];
    let heading = t.y.atan2(t.x);

    dbg_log!(
        "  MAT_CROSS  walk_increasing={}  p_cross=({:.3},{:.3})  \
         p_hit=({:.3},{:.3})  p_resume=({:.3},{:.3})  heading={:.4}",
        walk_increasing,
        p_cross.x,
        p_cross.y,
        p_hit.x,
        p_hit.y,
        p_resume.x,
        p_resume.y,
        heading,
    );

    Some(ToolPose {
        pos: p_resume,
        heading,
    })
}

/// BFS through cleared MAT nodes from `start`, returning **all** cleared
/// nodes that have at least one uncleared neighbour, in BFS (nearest
/// first) order.
pub fn find_all_mat_crossings(
    axis: &MedialAxis,
    start: usize,
    is_cleared: &[bool],
) -> Vec<usize> {
    use std::collections::VecDeque;
    let n = axis.nodes.len();
    if n == 0 {
        return Vec::new();
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in &axis.edges {
        adj[a].push(b);
        adj[b].push(a);
    }
    let mut visited = vec![false; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut crossings = Vec::new();
    visited[start] = true;
    queue.push_back(start);
    while let Some(idx) = queue.pop_front() {
        let mut has_uncleared = false;
        for &nb in &adj[idx] {
            if !is_cleared[nb] {
                has_uncleared = true;
            }
        }
        if has_uncleared {
            crossings.push(idx);
        }
        for &nb in &adj[idx] {
            if is_cleared[nb] && !visited[nb] {
                visited[nb] = true;
                queue.push_back(nb);
            }
        }
    }
    crossings
}
