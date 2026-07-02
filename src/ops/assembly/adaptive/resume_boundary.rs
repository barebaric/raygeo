use prof_macros::prof;

use crate::dbg_log;
use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_closest_point,
    get_polygons_group_intersection,
};
use crate::ops::assembly::adaptive::resume::ResumeCtx;
use crate::ops::assembly::adaptive::resume::ResumeStrategy;
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::search::walk_polygon_samples;
use crate::ops::cut::CutDirection;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

pub struct ResumeBoundary;

impl ResumeStrategy for ResumeBoundary {
    fn label(&self) -> &'static str {
        "boundary"
    }

    fn find_next(&self, ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
        envelope_resume(ctx, tool)
    }
}

/// Find a re-engagement point on the part of the cleared-area frontier
/// that lies **inside** the tool-centre envelope.
///
/// The frontier (cleared-area boundary) is intersected with the
/// envelope so only the portion of the frontier where the tool centre
/// is legally allowed (i.e. inside the envelope, not on the pocket
/// wall) is considered.  The walk starts at the vertex of this
/// resulting shape nearest the last wall-hug departure point (falling
/// back to the tool's current position) and proceeds in the cutting
/// rotational direction, probing each sample point for engagement.  The
/// first sampled point whose forward-step probe yields at least
/// `min_cut_area` becomes the resume target.
///
/// The resume point is the boundary sample point itself — the tool
/// centre is placed directly on the `frontier ∩ envelope` shape so the
/// disk overhang reaches into uncleared stock at the correct engagement
/// depth.  No inward offset is applied; the boundary is the line this
/// strategy walks.
///
/// Long edges are sub-sampled at `step_length` spacing so the walk does
/// not skip over a productive band between two far-apart vertices.
#[prof]
fn envelope_resume(ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
    let envelope = ctx.cleared.envelope(tool.radius);
    if envelope.is_empty() {
        return None;
    }

    // Frontier intersected with the envelope: the portion of the
    // cleared-area boundary that lies inside the tool-centre envelope
    // (i.e. where the tool centre is legally allowed).  This excludes
    // the pocket-wall portion of the frontier and is where productive
    // re-engagement is possible.
    let frontier = ctx.cleared.frontier(0.001);
    if frontier.is_empty() {
        return None;
    }
    let polys: Vec<Polygon> =
        get_polygons_group_intersection(&frontier, &envelope);
    if polys.is_empty() {
        return None;
    }

    // Reference pose for the nearest-point search: use the position
    // and heading where the current cutting segment began
    // (`segment_start`).  This is where the tool was last productively
    // cutting before stalling, so the frontier closest to it is the
    // natural place to resume the wall band walk.
    let ref_pos = ctx.segment_start.pos;

    // Find the true closest point on the intersection shape to the
    // reference point.  `get_polygons_closest_point` returns the exact
    // closest point (mid-edge when applicable) and the polygon index,
    // but not the edge index, so we re-derive the closest edge and its
    // parametric position `t` ourselves.  This is the actual nearest
    // point on the frontier — not just the nearest vertex — so the walk
    // starts precisely where the tool should resume.
    let (closest_poly_idx, _t, _closest_pt, _d2) =
        get_polygons_closest_point(&polys, ref_pos)?;
    let poly = &polys[closest_poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }

    // Find the closest edge and the parametric position along it.
    let mut start_idx = 0usize;
    let mut start_frac = 0.0f64;
    {
        let mut best_d2 = f64::MAX;
        for i in 0..n {
            let j = (i + 1) % n;
            let edge = poly[j] - poly[i];
            let elen2 = edge.length_squared();
            if elen2 < 1e-18 {
                continue;
            }
            let tt = ((ref_pos.x - poly[i].x) * edge.x
                + (ref_pos.y - poly[i].y) * edge.y)
                / elen2;
            let tt = tt.clamp(0.0, 1.0);
            let cp = poly[i] + edge * tt;
            let d2 = cp.distance_squared(ref_pos);
            if d2 < best_d2 {
                best_d2 = d2;
                start_idx = i;
                start_frac = tt;
            }
        }
    }

    // Determine the walk direction from `cut_direction` and the
    // polygon winding — the same approach as `resume_mat.rs`.
    // Clipper output winding is not guaranteed, so the signed area
    // fixes the mapping from "increasing index" to a geometric
    // rotational sense.
    //   * CCW polygon + CCW cut → increasing index = forward
    //   * CW  polygon + CW  cut → increasing index = forward
    //   * mismatched sense      → decreasing index = forward
    //
    // Using `cut_direction` (rather than the tool's heading, which may
    // have drifted or reversed after a stall) guarantees the walk
    // proceeds in the configured rotational direction and the probe
    // finds material on the correct side of the tool.
    let is_ccw = get_polygon_signed_area(poly) > 0.0;
    let cut_increasing =
        (ctx.opts.cut_direction == CutDirection::Ccw) == is_ccw;
    let actual_forward = cut_increasing;

    // Minimum engagement the stepper needs on its first step from the
    // resume point.  Half the target engagement per distance ensures a
    // productive bite without being so strict that no tangent position
    // qualifies.
    let min_cut_area = ctx.opts.step_length * ctx.target_area_pd * 0.5;

    // Sub-sample long edges so the walk does not skip over a productive
    // band between two far-apart vertices.
    let sample_spacing = ctx.opts.step_length;
    let dir_sign = ctx.opts.cut_direction.sign();

    // Walk starting at the true closest point (vertex `start_idx` plus
    // `start_frac` along the edge to the next vertex), using the shared
    // walk engine from `search.rs`.
    let step_length = ctx.opts.step_length;
    let radius = tool.radius;
    walk_polygon_samples(
        poly,
        start_idx,
        actual_forward,
        sample_spacing,
        false,
        start_frac,
        |pos, heading| {
            let dir = Point::new(heading.cos(), heading.sin());
            let probe = pos + dir * step_length;
            let valid_probe = point_in_valid_area(probe, ctx.valid_tool_area);
            let (area, left) = if valid_probe {
                ctx.cleared.cut_area_split(pos, probe, radius)
            } else {
                (0.0, 0.0)
            };
            let right = area - left;
            let correct_side = if dir_sign < 0.0 { right } else { left };
            let wrong_side = if dir_sign < 0.0 { left } else { right };
            let direction_ok = area <= 0.0 || correct_side >= wrong_side;
            let dest_eng = if valid_probe && area >= min_cut_area {
                ctx.cleared.point_engagement(probe, radius).angle
            } else {
                0.0
            };
            let eng_ok = dest_eng <= 2.7;
            if valid_probe && area >= min_cut_area && direction_ok && eng_ok {
                dbg_log!(
                    "  ENVELOPE  resume=({:.3},{:.3})  heading={:.4}  \
                 probe_area={:.4}  min_cut_area={:.5}  L={:.4}  R={:.4}",
                    pos.x,
                    pos.y,
                    heading,
                    area,
                    min_cut_area,
                    left,
                    right,
                );
                Some(ToolPose { pos, heading })
            } else {
                None
            }
        },
    )
    .or_else(|| {
        dbg_log!("  ENVELOPE  no suitable point found");
        None
    })
}
