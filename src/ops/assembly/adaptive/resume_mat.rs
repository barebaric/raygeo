use prof_macros::prof;

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_closest_point,
};
use crate::ops::assembly::adaptive::resume::{
    offset_and_probe, require_fragments, ResumeCtx, ResumeStrategy,
    DETAIL_NODE_NOT_CLEARED, DETAIL_NO_CROSSING, DETAIL_NO_FRONTIER,
    DETAIL_NO_WALL_HIT, WALL_PROXIMITY,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::CutDirection;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Point3D, Polygon};

pub struct ResumeMat;

impl ResumeStrategy for ResumeMat {
    fn label(&self) -> &'static str {
        "mat"
    }

    fn find_next(
        &self,
        ctx: &ResumeCtx,
        tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        let axis = match ctx.mat {
            Some(m) => m,
            None => {
                *detail = DETAIL_NO_CROSSING;
                return None;
            }
        };
        let fragments = require_fragments(ctx, detail)?;
        let is_cleared = axis.build_cleared_mask(fragments);
        let from_idx = match axis
            .nearest_node(crate::types::Point::new(tool.pos.x, tool.pos.y))
        {
            Some(i) => i,
            None => {
                *detail = DETAIL_NODE_NOT_CLEARED;
                return None;
            }
        };
        if !is_cleared[from_idx] {
            *detail = DETAIL_NODE_NOT_CLEARED;
            return None;
        }
        let crossings = find_all_mat_crossings(axis, from_idx, &is_cleared);
        dbg_log!(
            "  MAT_RESUME  crossings={}  from_idx={}",
            crossings.len(),
            from_idx,
        );
        for crossing_idx in crossings {
            let mut cross_detail = 0u8;
            if let Some(rp) = mat_resume_from_crossing(
                axis,
                ctx.cleared,
                crossing_idx,
                ctx.opts.cut_direction,
                tool.radius,
                ctx.advance,
                ctx.opts.target_z,
                &ctx.opts.pocket_boundary,
                &ctx.opts.islands,
                &mut cross_detail,
            ) {
                if let Some(probed) =
                    offset_and_probe(ctx, tool.radius, rp.pos, rp.heading)
                {
                    return Some(probed);
                }
            } else if cross_detail != 0 {
                *detail = cross_detail;
            }
        }
        if *detail == 0 {
            *detail = DETAIL_NO_CROSSING;
        }
        None
    }
}

/// Try to find a resume point from a single MAT crossing.
///
/// 1. `P_CROSS` = crossing node position (marks where fresh material
///    begins on the MAT).
/// 2. Pick the **largest** frontier polygon by area (typically the outer
///    cleared-area boundary that touches the pocket wall) rather than
///    the one geometrically nearest to `P_CROSS` — which may be an
///    interior hole ring with no wall-adjacent vertices.
/// 3. Walk the chosen polygon **backward** (opposite to the cutting
///    rotational direction) from the vertex nearest `P_CROSS` until
///    hitting the pocket boundary (within `WALL_PROXIMITY`) or
///    completing a full loop (→ fail, try next crossing).
/// 4. Place the tool centre at `radius` from the nearest wall point,
/// along the wall→cleared direction.  The heading is the frontier
/// tangent in the cutting direction.
#[allow(clippy::too_many_arguments)]
#[prof]
pub fn mat_resume_from_crossing(
    axis: &MedialAxis,
    cleared: &ClearedArea,
    crossing_idx: usize,
    cut_direction: CutDirection,
    radius: f64,
    _advance: f64,
    cut_z: f64,
    pocket_boundary: &[Point],
    islands: &[Polygon],
    detail: &mut u8,
) -> Option<ToolPose> {
    let p_cross = axis.nodes[crossing_idx].point;

    let polys = cleared.frontier(0.001);
    if polys.is_empty() {
        *detail = DETAIL_NO_FRONTIER;
        return None;
    }

    // Pick the frontier polygon with the largest absolute area — this
    // is the outer cleared-area boundary that touches the pocket wall,
    // not an interior hole ring around remaining material.
    let poly_idx = polys
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            get_polygon_signed_area(a)
                .abs()
                .partial_cmp(&get_polygon_signed_area(b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
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
        .map(|(i, _)| i)
        .unwrap_or(0);

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

    let hit_idx = match hit_idx {
        Some(i) => i,
        None => {
            *detail = DETAIL_NO_WALL_HIT;
            return None;
        }
    };
    let p_hit = poly[hit_idx];

    let (_, _, wall_pt, _) =
        match get_polygons_closest_point(&wall_polys, p_hit) {
            Some(r) => r,
            None => {
                *detail = DETAIL_NO_WALL_HIT;
                return None;
            }
        };
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
        pos: Point3D::new(p_resume.x, p_resume.y, cut_z),
        heading,
    })
}

/// BFS through cleared MAT nodes from `start`, returning **all** cleared
/// nodes that have at least one uncleared neighbour, in BFS (nearest
/// first) order.
#[prof]
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
