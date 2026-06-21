//! HSM (High-Speed Machining) motion assembly.
//!
//! These functions take pure geometric primitives (arcs, polygons, medial
//! axes) from the [`geo`](crate::geo) layer and assemble them into
//! [`Ops`](crate::ops::Ops) objects with appropriate motion classification
//! (cut vs travel) and state application.
//!
//! All geometric work (bite extraction, cutting-arc detection, filleting,
//! medial-axis computation) is delegated to geo-layer primitives.  This
//! module is the orchestrator that decides what to cut, in what order,
//! and how to traverse it.

use crate::geo::algo::cleared_area::ClearedArea;
use crate::geo::algo::fillet::{append_end_fillets, trim_to_safe_fillet_span};
use crate::geo::algo::helix::{generate_helix, HelixDirection, HelixOptions};
use crate::geo::algo::hsm::find_cutting_arc;
use crate::geo::algo::intersect::get_ray_polygon_intersection;
use crate::geo::algo::medial_axis::{
    compute_medial_axis, mat_path, MedialAxis,
};
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::ramp::{generate_ramp, RampOptions, RampStyle};
use crate::geo::algo::smooth::smooth_path;
use crate::geo::algo::spiral::{generate_spiral, SpiralOptions};
use crate::geo::shape::arc::get_polyline_turn_sign;
use crate::geo::shape::line::{
    does_line_cross_polygon, get_segment_segment_distance,
    longest_line_through_point,
};
use crate::geo::shape::polygon::{
    get_circle_polygon, get_polygon_area, get_polygon_bounds,
    get_polygon_centroid, get_polygons_group_difference, get_polyline_bounds,
    get_segment_swept_polygon, trim_polyline_at,
};
use crate::ops::container::Ops;
use crate::ops::state::State;
use crate::types::{Point, Point3D, Polygon};

const MAX_WAVEFRONT_ITERATIONS: usize = 1000;

// ── Entry strategy ─────────────────────────────────────────────

/// Options for [`adaptive_entry`].
#[derive(Clone, Debug)]
pub struct AdaptiveEntryOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub safe_z: f64,
    pub target_z: f64,
    pub plunge_pitch: f64,
    pub safe_margin: f64,
    pub angular_step: f64,
}

/// Return type of [`adaptive_entry`].
#[derive(Clone, Debug)]
pub struct AdaptiveEntryResult {
    pub ops: Ops,
    pub cleared_polygons: Vec<Polygon>,
}

/// Fast central clearing entry.
///
/// Given a pocket boundary (with optional islands), finds the optimal
/// entry pole and generates either:
///
/// - **Helix → Spiral** (wide area): helical plunge to depth followed by
///   a flat Archimedean spiral.
/// - **ZigZag Ramp** (tight slot): a trochoidal ramp along the longest
///   axis of the slot.
///
/// The result includes the Ops (with `cut_state` applied) and the swept
/// polygons that should be added to the [`ClearedArea`].
pub fn adaptive_entry(
    opts: &AdaptiveEntryOptions,
    cut_state: &State,
) -> AdaptiveEntryResult {
    let (entry_pt, r_max) =
        find_largest_circle(&opts.pocket_boundary, &opts.islands, 0.1)
            .unwrap_or_else(|| {
                let c = get_polygon_centroid(&opts.pocket_boundary);
                (c, 0.0)
            });

    let mut toolpath: Vec<Point3D> = Vec::new();

    if r_max > opts.tool_radius * 1.5 {
        let helix_r = (opts.tool_radius * 0.8).min(r_max * 0.5);

        if opts.target_z < opts.safe_z {
            let hp = generate_helix(&HelixOptions {
                center: entry_pt,
                start_radius: helix_r,
                end_radius: helix_r,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                pitch: opts.plunge_pitch,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
                min_revolutions: None,
            });
            toolpath.extend(hp);
        }

        let spiral_max_r =
            (r_max - opts.tool_radius - opts.safe_margin).max(helix_r + 0.01);
        let radial_dist = spiral_max_r - helix_r;

        if radial_dist > 0.0 && opts.step_over > 0.0 {
            let n_revs = radial_dist / opts.step_over;

            let sp = generate_spiral(&SpiralOptions {
                center: entry_pt,
                z: opts.target_z,
                start_radius: helix_r,
                end_radius: spiral_max_r,
                revolutions: n_revs,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            });
            toolpath.extend(sp);
        }

        let disk_r = spiral_max_r + opts.tool_radius;
        let cleared_polygons = vec![get_circle_polygon(entry_pt, disk_r, 64)];

        AdaptiveEntryResult {
            ops: points_to_ops(&toolpath, cut_state),
            cleared_polygons,
        }
    } else {
        let bbox = get_polygon_bounds(&opts.pocket_boundary);
        let (start, end) = longest_line_through_point(entry_pt, bbox);

        if opts.target_z < opts.safe_z {
            let rp = generate_ramp(&RampOptions {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: 45.0,
                style: RampStyle::ZigZag,
                lateral_amplitude: opts.tool_radius * 0.8,
            });
            toolpath.extend(rp);
        }

        let cleared_polygons =
            get_segment_swept_polygon(start, end, opts.tool_radius);

        AdaptiveEntryResult {
            ops: points_to_ops(&toolpath, cut_state),
            cleared_polygons,
        }
    }
}

// ── Wavefront expansion ────────────────────────────────────────

/// Options for [`adaptive_wavefronts`].
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub z: f64,
    pub area_tolerance: f64,
}

/// Inside-out adaptive wavefronts.
///
/// Starting from the cleared area, each iteration expands the frontier
/// (outer boundary) outward by `step_over`, clips to the valid tool
/// area, traces the wavefront, and updates the cleared state.
///
/// Each ring fragment is emitted as `MoveTo` (first point) + `LineTo`
/// (rest), all at height `z`, with `cut_state` applied.
pub fn adaptive_wavefronts(
    cleared: &mut ClearedArea,
    opts: &AdaptiveWavefrontOptions,
    cut_state: &State,
) -> Ops {
    let (valid_tool_area, valid_total_area) = compute_inset_region(
        &opts.pocket_boundary,
        opts.tool_radius,
        &opts.islands,
    );

    let mut ops = Ops::new();
    let mut state_applied = false;

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let bounded = cleared.bites(opts.step_over, &valid_tool_area, 0.01);
        if bounded.is_empty() {
            break;
        }

        let new_ring = cleared.incorporate(&bounded);
        if new_ring.is_empty() {
            continue;
        }

        for frag in &new_ring {
            if frag.len() < 2 {
                continue;
            }
            if !state_applied {
                ops.apply_state(cut_state);
                state_applied = true;
            }
            ops.move_to(frag[0].x, frag[0].y, opts.z, None);
            for p in &frag[1..] {
                ops.line_to(p.x, p.y, opts.z, None);
            }
        }

        let ring_area: f64 = new_ring.iter().map(get_polygon_area).sum();
        if ring_area < opts.area_tolerance
            || cleared.total_area() >= valid_total_area - 0.1
        {
            break;
        }
    }

    ops
}

/// Build Ops from a 3-D polyline: apply state, MoveTo first point,
/// LineTo the rest.
fn points_to_ops(path: &[Point3D], cut_state: &State) -> Ops {
    let mut ops = Ops::new();
    if path.is_empty() {
        return ops;
    }
    ops.apply_state(cut_state);
    ops.move_to(path[0].x, path[0].y, path[0].z, None);
    for p in &path[1..] {
        ops.line_to(p.x, p.y, p.z, None);
    }
    ops
}

// ── Internal types ─────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MotionRole {
    Cut,
    Travel,
}

struct MotionSegment {
    role: MotionRole,
    points: Vec<Point3D>,
}

// ── Public API ─────────────────────────────────────────────────

/// Link pre-computed arcs into an [`Ops`] with MAT-routed travel.
///
/// Consecutive arcs are joined by travel segments at `safe_z`.  When the
/// direct line would cross (or pass within `safe_margin` of) any polygon
/// in `uncleared`, the connection uses the Medial Axis to route around
/// obstacles, then smoothed.
///
/// The resulting Ops contains `LineTo` commands (at `cut_z`) for cutting
/// and `MoveTo` commands (at `safe_z`) for travel, with `cut_state` and
/// `travel_state` applied respectively.
#[allow(clippy::too_many_arguments)]
pub fn link_arcs_to_ops(
    arcs: &[Vec<Point>],
    uncleared: &[Polygon],
    mat: Option<&MedialAxis>,
    cut_z: f64,
    safe_z: f64,
    safe_margin: f64,
    smoothing_amount: i32,
    preserve_order: bool,
    cut_state: &State,
    travel_state: &State,
) -> Ops {
    let segments = build_link_segments(
        arcs,
        uncleared,
        mat,
        cut_z,
        safe_z,
        safe_margin,
        smoothing_amount,
        preserve_order,
    );
    segments_to_ops(&segments, cut_state, travel_state)
}

/// Run the peeling (D-cut) clearing strategy and return an [`Ops`].
///
/// All geometric work is delegated to geo primitives; this function
/// is the orchestrator that decides what to cut, in what order, and
/// how to traverse it.
///
/// The algorithm:
///
/// 1. Compute the Medial Axis to guide clearing directions.
/// 2. Directional phase: clear toward each MAT branch endpoint (60° cone).
/// 3. Fallback isotropic phase for any remaining material.
/// 4. Filter, fillet, and link cutting arcs into Ops.
#[allow(clippy::too_many_arguments)]
pub fn adaptive_peeling(
    cleared: &mut ClearedArea,
    pocket_boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    step_over: f64,
    cut_z: f64,
    safe_z: f64,
    wall_margin: f64,
    travel_smoothing: i32,
    area_tolerance: f64,
    cut_state: &State,
    travel_state: &State,
) -> Ops {
    let (valid_tool_area, valid_total_area) =
        compute_inset_region(pocket_boundary, tool_radius, islands);

    let holes: Vec<Vec<Point>> = islands.iter().map(|h| h.to_vec()).collect();
    let mat = compute_medial_axis(
        pocket_boundary,
        &holes,
        tool_radius,
        step_over * 0.5,
    )
    .ok();

    let centre = get_polygon_centroid(pocket_boundary);
    let mut targets: Vec<Point> = Vec::new();
    if let Some(ref ma) = mat {
        let mut branches: Vec<usize> = (0..ma.branches.len()).collect();
        branches.sort_by(|&a, &b| {
            let ca = ma.branches[a]
                .clearances
                .iter()
                .cloned()
                .fold(f64::MIN, f64::max);
            let cb = ma.branches[b]
                .clearances
                .iter()
                .cloned()
                .fold(f64::MIN, f64::max);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });
        for &bi in &branches {
            let branch = &ma.branches[bi];
            let mut best_idx = 0usize;
            let mut best_cl = f64::MIN;
            for (j, &cl) in branch.clearances.iter().enumerate() {
                if cl > best_cl {
                    best_cl = cl;
                    best_idx = j;
                }
            }
            let mat_pt = branch.points[best_idx];
            let dir = (mat_pt - centre).normalize();
            let boundary_pt =
                get_ray_polygon_intersection(centre, dir, pocket_boundary)
                    .unwrap_or(mat_pt);
            targets.push(boundary_pt);
        }
    }

    let mut cut_arcs: Vec<Vec<Point>> = Vec::new();

    let max_angle = std::f64::consts::FRAC_PI_3;
    for &target in &targets {
        for _ in 0..MAX_WAVEFRONT_ITERATIONS {
            let bites = cleared.bite_in_direction(
                step_over,
                &valid_tool_area,
                0.01,
                target,
                max_angle,
            );
            if bites.is_empty() {
                break;
            }
            for bite in &bites {
                if let Some((ref arc, _, _)) =
                    find_cutting_arc(bite, cleared.fragments())
                {
                    if arc.len() >= 3 {
                        cut_arcs.push(arc.clone());
                    }
                }
            }
            cleared.incorporate(&bites);
            if cleared.total_area() >= valid_total_area - area_tolerance {
                return finish_peeling(
                    cut_arcs,
                    &valid_tool_area,
                    pocket_boundary,
                    islands,
                    cleared,
                    mat.as_ref(),
                    cut_z,
                    safe_z,
                    tool_radius,
                    wall_margin,
                    travel_smoothing,
                    step_over,
                    cut_state,
                    travel_state,
                );
            }
        }
    }

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let bites = cleared.bites(step_over, &valid_tool_area, 0.01);
        if bites.is_empty() {
            break;
        }
        for bite in &bites {
            if let Some((ref arc, _, _)) =
                find_cutting_arc(bite, cleared.fragments())
            {
                if arc.len() >= 3 {
                    cut_arcs.push(arc.clone());
                }
            }
        }
        cleared.incorporate(&bites);
        if cleared.total_area() >= valid_total_area - area_tolerance {
            break;
        }
    }

    finish_peeling(
        cut_arcs,
        &valid_tool_area,
        pocket_boundary,
        islands,
        cleared,
        mat.as_ref(),
        cut_z,
        safe_z,
        tool_radius,
        wall_margin,
        travel_smoothing,
        step_over,
        cut_state,
        travel_state,
    )
}

// ── Internal helpers ───────────────────────────────────────────

/// Filter, fillet, and link cutting arcs into Ops.
#[allow(clippy::too_many_arguments)]
fn finish_peeling(
    cut_arcs: Vec<Vec<Point>>,
    valid_tool_area: &[Polygon],
    pocket_boundary: &Polygon,
    islands: &[Polygon],
    cleared: &ClearedArea,
    mat: Option<&MedialAxis>,
    cut_z: f64,
    safe_z: f64,
    tool_radius: f64,
    wall_margin: f64,
    travel_smoothing: i32,
    step_over: f64,
    cut_state: &State,
    travel_state: &State,
) -> Ops {
    if cut_arcs.is_empty() {
        return Ops::new();
    }

    let min_span = step_over;

    let filleted: Vec<Vec<Point>> = cut_arcs
        .iter()
        .filter_map(|arc| {
            if arc.len() < 3 {
                return None;
            }
            let b = get_polyline_bounds(arc);
            let span = (b.max.x - b.min.x).max(b.max.y - b.min.y);
            if span < min_span {
                return None;
            }
            let fa = if let Some((enter, exit)) = trim_to_safe_fillet_span(
                arc,
                pocket_boundary,
                islands,
                tool_radius,
                wall_margin,
            ) {
                let trimmed = trim_polyline_at(arc, enter, exit);
                if trimmed.len() < 3 {
                    arc.to_vec()
                } else {
                    let side = get_polyline_turn_sign(arc);
                    append_end_fillets(
                        &trimmed,
                        tool_radius,
                        std::f64::consts::FRAC_PI_2,
                        side,
                    )
                }
            } else {
                arc.to_vec()
            };
            if fa.len() >= 3 {
                Some(fa)
            } else {
                None
            }
        })
        .collect();

    if filleted.is_empty() {
        return Ops::new();
    }

    let mut uncleared = islands.to_vec();
    uncleared.extend(get_polygons_group_difference(
        valid_tool_area,
        cleared.fragments(),
    ));

    link_arcs_to_ops(
        &filleted,
        &uncleared,
        mat,
        cut_z,
        safe_z,
        tool_radius,
        travel_smoothing,
        true,
        cut_state,
        travel_state,
    )
}

/// Build motion segments from arcs using NN ordering, MAT routing, and
/// smoothing — the same algorithm as the former geo `link_filleted_arcs`.
#[allow(clippy::too_many_arguments)]
fn build_link_segments(
    arcs: &[Vec<Point>],
    uncleared: &[Polygon],
    mat: Option<&MedialAxis>,
    cut_z: f64,
    safe_z: f64,
    safe_margin: f64,
    smoothing_amount: i32,
    preserve_order: bool,
) -> Vec<MotionSegment> {
    let mut segments: Vec<MotionSegment> = Vec::new();

    let order: Vec<usize> = if preserve_order || arcs.is_empty() {
        (0..arcs.len()).collect()
    } else {
        let mut used = vec![false; arcs.len()];
        let mut o = Vec::with_capacity(arcs.len());
        let start_idx = (0..arcs.len())
            .max_by(|&i, &j| arcs[i].len().cmp(&arcs[j].len()))
            .unwrap_or(0);
        o.push(start_idx);
        used[start_idx] = true;
        while o.len() < arcs.len() {
            let last_end = *arcs[*o.last().unwrap()].last().unwrap();
            let mut best = None;
            let mut best_d2 = f64::MAX;
            for (i, arc) in arcs.iter().enumerate() {
                if used[i] || arc.len() < 2 {
                    continue;
                }
                let d2 = (arc[0] - last_end).length_squared();
                if d2 < best_d2 {
                    best_d2 = d2;
                    best = Some(i);
                }
            }
            if let Some(i) = best {
                o.push(i);
                used[i] = true;
            } else {
                break;
            }
        }
        o
    };

    for &oi in &order {
        let arc = &arcs[oi];
        if arc.len() < 2 {
            continue;
        }
        if segments.is_empty() {
            let pts: Vec<Point3D> =
                arc.iter().map(|p| Point3D::new(p.x, p.y, cut_z)).collect();
            segments.push(MotionSegment {
                role: MotionRole::Cut,
                points: pts,
            });
        } else {
            let last: Point = {
                let last_seg = segments.last().unwrap();
                let p = *last_seg.points.last().unwrap();
                Point::new(p.x, p.y)
            };
            let first: Point = arc[0];

            let blocked = if safe_margin > 0.0 {
                let margin2 = safe_margin * safe_margin;
                uncleared.iter().any(|poly| {
                    if does_line_cross_polygon(last, first, poly) {
                        return true;
                    }
                    for i in 0..poly.len() {
                        let a = poly[i];
                        let b = poly[(i + 1) % poly.len()];
                        let d = get_segment_segment_distance(last, first, a, b);
                        if d * d < margin2 {
                            return true;
                        }
                    }
                    false
                })
            } else {
                uncleared
                    .iter()
                    .any(|poly| does_line_cross_polygon(last, first, poly))
            };

            let link: Vec<Point> = if blocked {
                let mat_link = mat
                    .and_then(|ma| mat_path(ma, last, first))
                    .unwrap_or_else(|| vec![last, first]);
                if mat_link.len() < 2 {
                    vec![last, first]
                } else {
                    let mut full = Vec::with_capacity(mat_link.len() + 2);
                    full.push(last);
                    full.extend(mat_link);
                    if (full.last().unwrap() - first).length_squared() > 1e-12 {
                        full.push(first);
                    }
                    smooth_path(&full, uncleared, safe_margin, smoothing_amount)
                }
            } else {
                vec![last, first]
            };

            let travel_pts: Vec<Point3D> = link
                .iter()
                .map(|p| Point3D::new(p.x, p.y, safe_z))
                .collect();
            segments.push(MotionSegment {
                role: MotionRole::Travel,
                points: travel_pts,
            });

            let skip_start =
                (arc[0] - *link.last().unwrap()).length_squared() < 1e-12;
            let cut_pts: Vec<Point3D> = arc
                .iter()
                .enumerate()
                .filter(|(i, _)| !(*i == 0 && skip_start))
                .map(|(_, p)| Point3D::new(p.x, p.y, cut_z))
                .collect();
            if !cut_pts.is_empty() {
                segments.push(MotionSegment {
                    role: MotionRole::Cut,
                    points: cut_pts,
                });
            }
        }
    }

    segments
}

/// Convert motion segments to Ops with state application on role change.
fn segments_to_ops(
    segments: &[MotionSegment],
    cut_state: &State,
    travel_state: &State,
) -> Ops {
    let mut ops = Ops::new();
    let mut current_role: Option<MotionRole> = None;
    let mut is_first = true;

    for seg in segments {
        if current_role != Some(seg.role) {
            match seg.role {
                MotionRole::Cut => ops.apply_state(cut_state),
                MotionRole::Travel => ops.apply_state(travel_state),
            }
            current_role = Some(seg.role);
        }

        for p in &seg.points {
            if is_first {
                ops.move_to(p.x, p.y, p.z, None);
                is_first = false;
            } else {
                match seg.role {
                    MotionRole::Cut => ops.line_to(p.x, p.y, p.z, None),
                    MotionRole::Travel => ops.move_to(p.x, p.y, p.z, None),
                }
            }
        }
    }

    ops
}
