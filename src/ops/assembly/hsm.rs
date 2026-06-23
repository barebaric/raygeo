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

use crate::prof::prof_report;
use prof_macros::prof;

use crate::geo::algo::cleared_area::ClearedArea;

use crate::geo::algo::fillet::descending_radius_fillet;
use crate::geo::algo::helix::{
    generate_helix_3d, HelixDirection, HelixOptions,
};
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::ordering::order_nearest_neighbor;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::ramp::{generate_ramp_3d, RampOptions, RampStyle};
use crate::geo::algo::smooth::{
    blend_tangent, build_smoothed_path, chaikin_corner_cut,
};
use crate::geo::algo::spiral::{generate_spiral_3d, SpiralOptions};
use crate::geo::shape::line::longest_line_through_point;
use crate::geo::shape::polygon::{
    compute_polygon_bounds, does_path_sweep_intersect_polygon,
    get_circle_polygon, get_polygon_area, get_polygon_boundary_distance,
    get_polygon_bounds, get_polygon_centroid, get_polygon_closest_point,
    get_polygon_group_bounds, get_polygon_vertex_centroid,
    get_polygons_group_difference, get_segment_swept_polygon,
};
use crate::geo::shape::polyline::{
    get_polyline_bounds, split_polyline_at_v_junctions,
    trim_polyline_angular_ends,
};
use crate::ops::container::Ops;
use crate::ops::state::State;
use crate::types::{Point, Point3D, Polygon, Rect};

const MAX_WAVEFRONT_ITERATIONS: usize = 1000;
const DERIV_THRESHOLD: f64 = 0.436_332_312_998_582_4;
const V_JUNCTION_THRESHOLD: f64 = 0.1; // upstream processes MUST provide higher precision than this

// ── Cutting-arc extraction ─────────────────────────────────────

/// Find the longest contiguous run of outer vertices from pre-computed
/// boundary distances.
///
/// `n` is `bite.len()`, `dists` has length `n` with the squared distance
/// from each bite vertex to the nearest cleared boundary.
///
/// Returns `(arc_vertices, cut_start, cut_len)` — see
/// [`find_cutting_arc`] for details.
fn find_cutting_arc_from_dists(
    bite: &[Point],
    n: usize,
    dists: &[f64],
) -> Option<(Vec<Point>, usize, usize)> {
    let max_d = dists.iter().copied().fold(0.0, f64::max);
    let threshold = if dists.iter().any(|d| *d > 1e-4) {
        1e-4
    } else if max_d > 1e-9 {
        max_d * 0.3
    } else {
        return None;
    };

    let is_outer: Vec<bool> = dists.iter().map(|d| *d > threshold).collect();

    let extended: Vec<bool> = is_outer
        .iter()
        .copied()
        .chain(is_outer.iter().copied())
        .collect();

    let mut cut_start = 0usize;
    let mut cut_len = 0usize;
    {
        let mut cs: Option<usize> = None;
        let mut cl = 0usize;
        for (i, &val) in extended.iter().enumerate() {
            if val {
                if cs.is_none() {
                    cs = Some(i);
                    cl = 1;
                } else {
                    cl += 1;
                }
                if cl > cut_len {
                    cut_start = cs.unwrap();
                    cut_len = cl;
                }
            } else {
                cs = None;
                cl = 0;
            }
        }
    }

    if cut_len < 2 {
        return None;
    }

    // Trim transition vertices at the tips where the outer arc meets
    // the inner arc by detecting sharp jumps in interior angle.
    // Safe for cut_len < 3 (no-op).
    (cut_start, cut_len) =
        trim_polyline_angular_ends(bite, cut_start, cut_len, DERIV_THRESHOLD);

    if cut_len < 2 {
        return None;
    }

    let mut vertices: Vec<Point> =
        (0..cut_len).map(|i| bite[(cut_start + i) % n]).collect();

    // When only two outer vertices remain (e.g. a closing corner),
    // interpolate a midpoint so the downstream code always sees an
    // arc of at least three points.
    if vertices.len() == 2 {
        let mid = (vertices[0] + vertices[1]) * 0.5;
        vertices.insert(1, mid);
    }

    let len = vertices.len();
    Some((vertices, cut_start, len))
}

/// Extract the outer (cutting) arc from a bite polygon, computing
/// each vertex's distance from the cleared boundary via `dist_fn`.
fn find_cutting_arc_with(
    bite: &[Point],
    n: usize,
    dist_fn: impl Fn(f64, f64) -> f64,
) -> Option<(Vec<Point>, usize, usize)> {
    if n < 3 {
        return None;
    }
    let dists: Vec<f64> = bite.iter().map(|p| dist_fn(p.x, p.y)).collect();
    find_cutting_arc_from_dists(bite, n, &dists)
}

/// Extract the outer (cutting) arc from a bite polygon.
///
/// The bite is a crescent between the cleared frontier and the expanded
/// boundary.  The cutting arc is the longest contiguous run of bite
/// vertices that lie *outside* all cleared fragments.
///
/// Uses a static distance threshold (1e-4).  When no vertex clears that
/// threshold the function falls back to a relative threshold (30 % of
/// the maximum distance) so that the outermost ridge of even a tight
/// sliver is identified.
///
/// Returns `(arc_vertices, cut_start, cut_len)` where `arc_vertices` is
/// the contiguous slice of `bite` forming the outer arc, `cut_start`
/// is the index into `bite`, and `cut_len` is the number of vertices
/// in the arc.  Returns `None` when the bite is degenerate (no outer
/// arc found).
#[prof]
pub fn find_cutting_arc(
    bite: &Polygon,
    cleared_fragments: &[Polygon],
) -> Option<(Vec<Point>, usize, usize)> {
    let n = bite.len();
    find_cutting_arc_with(bite, n, |x, y| {
        cleared_fragments
            .iter()
            .filter_map(|frag| {
                get_polygon_closest_point(frag, x, y).map(|(_, _, d2)| d2)
            })
            .fold(f64::MAX, f64::min)
    })
}

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
            let hp = generate_helix_3d(&HelixOptions {
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

            let start_angle = if let Some(last) = toolpath.last() {
                (last.y - entry_pt.y).atan2(last.x - entry_pt.x)
            } else {
                0.0
            };

            let sp = generate_spiral_3d(&SpiralOptions {
                center: entry_pt,
                z: opts.target_z,
                start_radius: helix_r,
                end_radius: spiral_max_r,
                revolutions: n_revs,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
                start_angle,
            });
            toolpath.extend(sp);
        }

        // Final circular pass at the outer radius to smooth out the
        // scalloped boundary left by the Archimedean spiral, giving the
        // adaptive peeling a clean circular frontier.
        if !toolpath.is_empty() {
            let last = *toolpath.last().unwrap();
            let start_a = (last.y - entry_pt.y).atan2(last.x - entry_pt.x);
            let n_circ = ((2.0 * std::f64::consts::PI / opts.angular_step)
                .ceil() as usize)
                .max(8);
            for i in 1..=n_circ {
                let a = start_a
                    - i as f64 * 2.0 * std::f64::consts::PI / n_circ as f64;
                toolpath.push(Point3D::new(
                    entry_pt.x + spiral_max_r * a.cos(),
                    entry_pt.y + spiral_max_r * a.sin(),
                    opts.target_z,
                ));
            }
        }

        let disk_r = spiral_max_r;
        let cleared_polygons = vec![get_circle_polygon(entry_pt, disk_r, 64)];

        AdaptiveEntryResult {
            ops: points_to_ops(&toolpath, cut_state),
            cleared_polygons,
        }
    } else {
        let bbox = get_polygon_bounds(&opts.pocket_boundary);
        let (start, end) = longest_line_through_point(entry_pt, bbox);

        let lateral_amplitude = opts.tool_radius * 0.8;

        if opts.target_z < opts.safe_z {
            let rp = generate_ramp_3d(&RampOptions {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: 45.0,
                style: RampStyle::ZigZag,
                lateral_amplitude,
            });
            toolpath.extend(rp);
        }

        let cleared_polygons =
            get_segment_swept_polygon(start, end, lateral_amplitude);

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
#[prof]
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
#[prof]
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
#[prof]
pub fn link_arcs_to_ops(
    arcs: &[Vec<Point>],
    uncleared: &[Polygon],
    mat: Option<&MedialAxis>,
    cleared: Option<&[Polygon]>,
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
        cleared,
        cut_z,
        safe_z,
        safe_margin,
        smoothing_amount,
        preserve_order,
    );
    segments_to_ops(&segments, cut_state, travel_state)
}

/// Directed bite graph produced by [`split_ordered_wavefronts`].
///
/// Nodes are individual bite polygons, identified by a *global index*
/// computed from `bite_offsets`:
///
/// ```text
/// global = bite_offsets[pass] + local_index_within_pass
/// ```
///
/// Each bite has exactly one parent (the nearest previous-pass bite
/// sharing boundary), forming a tree.  Branches split when an island
/// separates the frontier and merge back when the island is cleared.
/// `visit_order` lists global bite indices in DFS traversal order.
#[derive(Debug, Clone)]
pub struct WavefrontGraph {
    /// Cutting arcs in DFS visit order.
    pub arcs: Vec<Vec<Point>>,
    /// Pass index for each arc in `arcs` (same length, same order).
    pub arc_passes: Vec<usize>,
    /// Per-pass bite polygons: `bite_polys[pass][local]`.
    pub bite_polys: Vec<Vec<Polygon>>,
    /// Per-bite arc indices into `arcs` (DFS order):
    /// `bite_arcs[global_bite] = [arc_idx, ...]`.
    pub bite_arcs: Vec<Vec<usize>>,
    /// `parent[global]` = parent bite index, or `None` for roots.
    pub parent: Vec<Option<usize>>,
    /// Pass start offsets for global↔local conversion.
    pub bite_offsets: Vec<usize>,
    /// Global bite indices in the order visited by DFS.
    pub visit_order: Vec<usize>,
    /// V-junction-split sub-segments from each arc, flattened in arc order.
    pub segments: Vec<Vec<Point>>,
    /// Outward normal (unit vector) for each segment in `segments`.
    pub segment_directions: Vec<Point>,
    /// For each arc in `arcs`, indices into `segments`.
    pub arc_segments: Vec<Vec<usize>>,
}

/// Run the peeling clearing strategy, ordering arcs in one pass.
///
/// The ordering is derived from a **parent tree** built during the
/// clearing loop:
///
/// 1. Each pass saves its bite polygons.
/// 2. Each bite is assigned ONE parent — the nearest previous-pass
///    bite sharing boundary (midpoint-segment distance ≈ 0).
/// 3. DFS from pass-0 roots produces the processing order: follow
///    one branch outward, backtrack, continue.
///
/// Branches split when an island separates the frontier and merge
/// back into a single branch when the island is cleared.
#[prof]
pub fn split_ordered_wavefronts(
    cleared: &mut ClearedArea,
    step_over: f64,
    valid_area: &[Polygon],
    simplify_tol: f64,
    entry: Point,
) -> WavefrontGraph {
    if cleared.fragments().is_empty() {
        return WavefrontGraph {
            arcs: Vec::new(),
            arc_passes: Vec::new(),
            bite_polys: Vec::new(),
            bite_arcs: Vec::new(),
            parent: Vec::new(),
            bite_offsets: Vec::new(),
            visit_order: Vec::new(),
            segments: Vec::new(),
            segment_directions: Vec::new(),
            arc_segments: Vec::new(),
        };
    }

    let (bite_polys_per_pass, bite_arcs_per_pass, all_arcs, all_arc_pass) =
        collect_bites(cleared, step_over, valid_area, simplify_tol);

    if bite_polys_per_pass.is_empty() {
        return WavefrontGraph {
            arcs: Vec::new(),
            arc_passes: Vec::new(),
            bite_polys: Vec::new(),
            bite_arcs: Vec::new(),
            parent: Vec::new(),
            bite_offsets: Vec::new(),
            visit_order: Vec::new(),
            segments: Vec::new(),
            segment_directions: Vec::new(),
            arc_segments: Vec::new(),
        };
    }

    let (bite_offsets, total_bites) =
        compute_bite_offsets(&bite_polys_per_pass);
    let parent = build_parent_tree(&bite_polys_per_pass, &bite_offsets);

    let ctx = BiteOrderCtx {
        entry,
        parent: &parent,
        bite_offsets: &bite_offsets,
        bite_polys_per_pass: &bite_polys_per_pass,
        bite_arcs_per_pass: &bite_arcs_per_pass,
        all_arcs: &all_arcs,
        all_arc_pass: &all_arc_pass,
    };
    let (arcs, arc_passes, bite_arcs, visit_order) =
        order_bites_dfs(&ctx, total_bites);

    let (segments, segment_directions, arc_segments) =
        split_arcs_at_v_junctions(&arcs, entry);

    WavefrontGraph {
        arcs,
        arc_passes,
        bite_polys: bite_polys_per_pass,
        bite_arcs,
        parent,
        bite_offsets,
        visit_order,
        segments,
        segment_directions,
        arc_segments,
    }
}

/// Convert a global bite index to (pass_index, bite_index_within_pass).
#[prof]
fn global_to_local(global: usize, offsets: &[usize]) -> (usize, usize) {
    for (pass, &off) in offsets.iter().enumerate() {
        let next = if pass + 1 < offsets.len() {
            offsets[pass + 1]
        } else {
            usize::MAX
        };
        if global >= off && global < next {
            return (pass, global - off);
        }
    }
    (0, global)
}

type CollectedBites = (
    Vec<Vec<Polygon>>,
    Vec<Vec<Vec<usize>>>,
    Vec<Vec<Point>>,
    Vec<usize>,
);

type OrderedBites = (Vec<Vec<Point>>, Vec<usize>, Vec<Vec<usize>>, Vec<usize>);

/// Read-only context shared by bite ordering and DFS.
struct BiteOrderCtx<'a> {
    entry: Point,
    parent: &'a [Option<usize>],
    bite_offsets: &'a [usize],
    bite_polys_per_pass: &'a [Vec<Polygon>],
    bite_arcs_per_pass: &'a [Vec<Vec<usize>>],
    all_arcs: &'a [Vec<Point>],
    all_arc_pass: &'a [usize],
}

// ── Helpers for split_ordered_wavefronts ────────────────────────

/// Accumulate bites and cutting arcs from wavefront expansion.
///
/// Each iteration computes bites via
/// [`ClearedArea::compute_bites`], extracts cutting arcs against the
/// current (unchanged) frontier, then absorbs the bites into the
/// frontier via [`ClearedArea::absorb_frontier`].
/// Returns per-pass data, arc data, and pass indices for each arc.
#[prof]
fn collect_bites(
    cleared: &mut ClearedArea,
    step_over: f64,
    valid_area: &[Polygon],
    simplify_tol: f64,
) -> CollectedBites {
    let mut bite_polys_per_pass: Vec<Vec<Polygon>> = Vec::new();
    let mut bite_arcs_per_pass: Vec<Vec<Vec<usize>>> = Vec::new();
    let mut all_arcs: Vec<Vec<Point>> = Vec::new();
    // Pass index for each arc in `all_arcs`.
    let mut all_arc_pass: Vec<usize> = Vec::new();

    // Compute a max_passes cap from the pocket bounding box so that
    // the loop always has enough iterations to finish naturally, while
    // preventing hangs when bites stall (slivers that incorporate()
    // can't absorb due to clipper precision).
    let bbox = get_polygon_group_bounds(valid_area);
    let diameter =
        ((bbox.max.x - bbox.min.x) + (bbox.max.y - bbox.min.y)).max(0.0);
    let max_passes = if step_over > 1e-9 {
        ((diameter / step_over) * 2.0 + 100.0) as usize
    } else {
        10000
    };

    loop {
        if bite_polys_per_pass.len() >= max_passes {
            break;
        }

        let pass_idx = bite_polys_per_pass.len();
        let bites = cleared.compute_bites(step_over, valid_area, simplify_tol);
        if bites.is_empty() {
            break;
        }

        let mut any_arc = false;
        let mut bite_arcs: Vec<Vec<usize>> = Vec::with_capacity(bites.len());
        for bite in &bites {
            let mut arcs = Vec::new();
            if let Some((ref arc, _, _)) =
                find_cutting_arc_with(bite, bite.len(), |x, y| {
                    cleared.closest_boundary_distance_sq(x, y)
                })
            {
                if arc.len() >= 3 {
                    any_arc = true;
                    arcs.push(all_arcs.len());
                    all_arcs.push(arc.clone());
                    all_arc_pass.push(pass_idx);
                }
            }
            // When no arc is found the bite is either
            // fully inside the cleared area (all vertices at distance
            // zero) or has too few outer vertices (< 3).  In both
            // cases the material is still tracked via
            // add_cleared_polygons below; emitting a fallback arc
            // would create duplicate cutlines because the bite
            // includes shared inner vertices from the previous pass.
            bite_arcs.push(arcs);
        }

        // Absorb bites AFTER arc extraction so that
        // closest_boundary_distance_sq still queries the old frontier.
        cleared.absorb_frontier(&bites);

        bite_polys_per_pass.push(bites);
        bite_arcs_per_pass.push(bite_arcs);

        // If no bite in this pass produced a cutting arc, all remaining
        // bites are Clipper2 slivers — real corners always have at least
        // one outer vertex.  Stop to avoid processing phantom passes.
        if !any_arc {
            break;
        }
    }

    (
        bite_polys_per_pass,
        bite_arcs_per_pass,
        all_arcs,
        all_arc_pass,
    )
}

/// Compute global bite offsets and total bite count from per-pass data.
#[prof]
fn compute_bite_offsets(
    bite_polys_per_pass: &[Vec<Polygon>],
) -> (Vec<usize>, usize) {
    let mut bite_offsets = Vec::with_capacity(bite_polys_per_pass.len());
    let mut off = 0;
    for pass in bite_polys_per_pass {
        bite_offsets.push(off);
        off += pass.len();
    }
    (bite_offsets, off)
}

/// Build parent tree from bite polygons.
///
/// Each bite in pass N+1 is assigned its nearest boundary-sharing
/// bite in pass N as its parent, forming a forest.
#[prof]
fn build_parent_tree(
    bite_polys_per_pass: &[Vec<Polygon>],
    bite_offsets: &[usize],
) -> Vec<Option<usize>> {
    let total_bites = bite_offsets.last().copied().unwrap_or(0)
        + bite_polys_per_pass.last().map_or(0, Vec::len);
    let mut parent: Vec<Option<usize>> = vec![None; total_bites];

    // Tree property: each bite gets exactly ONE parent — the nearest
    // previous-pass bite that shares boundary with it.  Branches split
    // when an island separates the frontier and merge back into a
    // single branch when the island is cleared.
    for pi in 1..bite_polys_per_pass.len() {
        let prev_polys = &bite_polys_per_pass[pi - 1];
        let curr_polys = &bite_polys_per_pass[pi];
        let prev_off = bite_offsets[pi - 1];
        let curr_off = bite_offsets[pi];

        for (ci, curr_bite) in curr_polys.iter().enumerate() {
            let curr_c = get_polygon_vertex_centroid(curr_bite);
            let mut best: Option<usize> = None;
            let mut best_dist = f64::MAX;
            for (pi2, prev_bite) in prev_polys.iter().enumerate() {
                if get_polygon_boundary_distance(curr_bite, prev_bite) < 1e-6 {
                    let d = (get_polygon_vertex_centroid(prev_bite) - curr_c)
                        .length_squared();
                    if d < best_dist {
                        best_dist = d;
                        best = Some(prev_off + pi2);
                    }
                }
            }
            parent[curr_off + ci] = best;
        }
    }

    parent
}

/// Order bites via DFS traversal of the parent tree.
///
/// Starts from pass-0 roots nearest to `entry`, follows children in
/// distance-from-entry order, then picks up stragglers.
#[prof]
fn order_bites_dfs(ctx: &BiteOrderCtx, total_bites: usize) -> OrderedBites {
    // Each bite has at most one parent (tree property).
    // Start from pass-0 roots (nearest to entry first).
    // Follow one child as deep as possible before backtracking.
    let mut visited = vec![false; total_bites];
    let mut result: Vec<Vec<Point>> = Vec::with_capacity(ctx.all_arcs.len());
    let mut arc_passes: Vec<usize> = Vec::with_capacity(ctx.all_arcs.len());
    let mut bite_arcs: Vec<Vec<usize>> = vec![Vec::new(); total_bites];
    let mut visit_order: Vec<usize> = Vec::with_capacity(total_bites);

    let mut roots: Vec<(usize, f64)> = Vec::new();
    for (bi, bite) in ctx.bite_polys_per_pass[0].iter().enumerate() {
        let c = get_polygon_vertex_centroid(bite);
        roots.push((bi, (c - ctx.entry).length_squared()));
    }
    roots.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    for &(root, _) in &roots {
        dfs(
            ctx,
            root,
            &mut visited,
            &mut result,
            &mut arc_passes,
            &mut bite_arcs,
            &mut visit_order,
        );
    }

    // Pick up any unvisited bites whose parent is visited or None.
    loop {
        let mut progress = false;
        for bi in 0..total_bites {
            let parent_visited = ctx.parent[bi].is_none_or(|p| visited[p]);
            if !visited[bi] && parent_visited {
                dfs(
                    ctx,
                    bi,
                    &mut visited,
                    &mut result,
                    &mut arc_passes,
                    &mut bite_arcs,
                    &mut visit_order,
                );
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }

    // Remaining unvisited: emit in index order.
    for (bi, &v) in visited.iter().enumerate().take(total_bites) {
        if !v {
            visit_order.push(bi);
            let (pass_idx, local) = global_to_local(bi, ctx.bite_offsets);
            for &arc_idx in &ctx.bite_arcs_per_pass[pass_idx][local] {
                result.push(ctx.all_arcs[arc_idx].clone());
                arc_passes.push(ctx.all_arc_pass[arc_idx]);
                bite_arcs[bi].push(result.len() - 1);
            }
        }
    }

    (result, arc_passes, bite_arcs, visit_order)
}

/// Recursive DFS for bite ordering.
///
/// Emits arcs for the current bite, then visits children
/// in distance-from-entry order.
#[allow(clippy::needless_range_loop)]
#[prof]
fn dfs(
    ctx: &BiteOrderCtx,
    bite: usize,
    visited: &mut [bool],
    result: &mut Vec<Vec<Point>>,
    arc_passes: &mut Vec<usize>,
    bite_arcs: &mut [Vec<usize>],
    visit_order: &mut Vec<usize>,
) {
    if visited[bite] {
        return;
    }
    // Tree constraint: parent must be visited first.
    if let Some(p) = ctx.parent[bite] {
        if !visited[p] {
            return;
        }
    }
    visited[bite] = true;
    visit_order.push(bite);

    // Emit this bite's arcs.
    let (pass_idx, local) = global_to_local(bite, ctx.bite_offsets);
    for &arc_idx in &ctx.bite_arcs_per_pass[pass_idx][local] {
        result.push(ctx.all_arcs[arc_idx].clone());
        arc_passes.push(ctx.all_arc_pass[arc_idx]);
        bite_arcs[bite].push(result.len() - 1);
    }

    // Visit children (bites whose parent is this one).
    // Process in distance-from-entry order for travel efficiency.
    let mut children: Vec<(usize, f64)> = Vec::new();
    for ci in bite + 1..ctx.parent.len() {
        if ctx.parent[ci] == Some(bite) && !visited[ci] {
            let (cp, cl) = global_to_local(ci, ctx.bite_offsets);
            let c =
                get_polygon_vertex_centroid(&ctx.bite_polys_per_pass[cp][cl]);
            let d = (c - ctx.entry).length_squared();
            children.push((ci, d));
        }
    }
    children.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    for (ci, _) in children {
        dfs(ctx, ci, visited, result, arc_passes, bite_arcs, visit_order);
    }
}

/// Split arcs at V-junctions and compute segment outward normals.
#[prof]
fn split_arcs_at_v_junctions(
    arcs: &[Vec<Point>],
    entry: Point,
) -> (Vec<Vec<Point>>, Vec<Point>, Vec<Vec<usize>>) {
    let mut segments: Vec<Vec<Point>> = Vec::new();
    let mut segment_directions: Vec<Point> = Vec::new();
    let mut arc_segments: Vec<Vec<usize>> = Vec::with_capacity(arcs.len());

    for arc in arcs {
        let comps = split_polyline_at_v_junctions(arc, V_JUNCTION_THRESHOLD);
        let mut idxs = Vec::with_capacity(comps.len());
        for seg in &comps {
            idxs.push(segments.len());
            // Compute outward normal at the segment midpoint.
            let dir = if seg.len() >= 3 {
                let mid = seg.len() / 2;
                let d0 = seg[mid] - seg[mid.saturating_sub(1)];
                let d1 = seg[(mid + 1).min(seg.len() - 1)] - seg[mid];
                let tangent = ((d0 + d1) * 0.5).normalize_or_zero();
                let n1 = Point::new(-tangent.y, tangent.x);
                let n2 = Point::new(tangent.y, -tangent.x);
                let midpoint = seg[mid];
                let to_mid = midpoint - entry;
                if n1.dot(to_mid) > n2.dot(to_mid) {
                    if n1.length_squared() > 0.5 {
                        n1
                    } else {
                        Point::ZERO
                    }
                } else if n2.length_squared() > 0.5 {
                    n2
                } else {
                    Point::ZERO
                }
            } else {
                Point::ZERO
            };
            segments.push(seg.clone());
            segment_directions.push(dir);
        }
        arc_segments.push(idxs);
    }

    (segments, segment_directions, arc_segments)
}

/// Run the peeling clearing strategy and return an [`Ops`].
///
/// Generates, splits, and orders cutting arcs via
/// [`split_ordered_wavefronts`], then fillets and links them into Ops.
#[allow(clippy::too_many_arguments)]
#[prof]
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
    cut_state: &State,
    travel_state: &State,
) -> Ops {
    let (valid_tool_area, _valid_total_area) =
        compute_inset_region(pocket_boundary, tool_radius, islands);

    let holes: Vec<Vec<Point>> = islands.iter().map(|h| h.to_vec()).collect();
    let mat = MedialAxis::compute(
        pocket_boundary,
        &holes,
        tool_radius,
        step_over * 0.5,
    )
    .ok();

    let centre = cleared
        .fragments()
        .iter()
        .max_by(|a, b| {
            let aa = get_polygon_area(a);
            let ab = get_polygon_area(b);
            ab.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(get_polygon_centroid)
        .unwrap_or_else(|| get_polygon_centroid(pocket_boundary));

    let cut_arcs = split_ordered_wavefronts(
        cleared,
        step_over,
        &valid_tool_area,
        0.01,
        centre,
    );

    let result = finish_peeling(
        &cut_arcs,
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
    prof_report();
    result
}

// ── Internal helpers ───────────────────────────────────────────

/// Filter, fillet, and link cutting arcs into Ops.
#[allow(clippy::too_many_arguments)]
#[prof]
fn finish_peeling(
    graph: &WavefrontGraph,
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
    let cut_arcs = &graph.arcs;
    if cut_arcs.is_empty() {
        return Ops::new();
    }

    let min_span = step_over;

    let filleted: Vec<Vec<Point>> = cut_arcs
        .iter()
        .enumerate()
        .filter_map(|(ai, arc)| {
            if arc.len() < 3 {
                return None;
            }
            let b = get_polyline_bounds(arc);
            let span = (b.max.x - b.min.x).max(b.max.y - b.min.y);
            if span < min_span {
                return None;
            }
            // Use pre-computed segment directions to determine fillet
            // side.  The fillet arcs opposite to the wave direction
            // (i.e. inward toward the cleared area).  The arc is kept
            // whole so no fillet is inserted at merged wave junctions.
            let (start_side, end_side) = if ai < graph.arc_segments.len() {
                let seg_idxs = &graph.arc_segments[ai];
                if !seg_idxs.is_empty() {
                    fillet_sides_from_directions(graph, seg_idxs)
                } else {
                    (1.0, 1.0)
                }
            } else {
                (1.0, 1.0)
            };

            let fa = descending_radius_fillet(
                arc,
                pocket_boundary,
                islands,
                tool_radius,
                wall_margin,
                start_side,
                end_side,
            );
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
        Some(cleared.fragments()),
        cut_z,
        safe_z,
        tool_radius,
        travel_smoothing,
        true,
        cut_state,
        travel_state,
    )
}

/// Compute fillet enter/exit sides from pre-computed segment directions.
///
/// The outer flag (at the segment midpoint) is one of the two normals
/// to the local tangent.  Comparing it with the left normal at the
/// same midpoint tells us whether outer is LEFT or RIGHT of travel
/// direction — a property that is invariant along the entire segment.
/// The fillet arcs opposite of the outer flag, i.e. toward the inner
/// (cleared) side.  The arc is kept whole so no fillet is inserted at
/// merged wave junctions.
#[prof]
fn fillet_sides_from_directions(
    graph: &WavefrontGraph,
    seg_idxs: &[usize],
) -> (f64, f64) {
    fn side_from_flag(seg: &[Point], flag: Point) -> f64 {
        let mid = seg.len() / 2;
        let d0 = seg[mid] - seg[mid.saturating_sub(1)];
        let d1 = seg[(mid + 1).min(seg.len() - 1)] - seg[mid];
        let t_mid = ((d0 + d1) * 0.5).normalize_or_zero();
        // Left normal at the midpoint.  If the flag aligns with it
        // then outer is LEFT (CW) → inner is RIGHT → side = -1.
        // Otherwise outer is RIGHT (CCW) → inner is LEFT → side = +1.
        let left_normal = Point::new(-t_mid.y, t_mid.x);
        if left_normal.dot(flag) > 0.0 {
            -1.0
        } else {
            1.0
        }
    }

    let start_side = side_from_flag(
        &graph.segments[seg_idxs[0]],
        graph.segment_directions[seg_idxs[0]],
    );
    let end_side = side_from_flag(
        &graph.segments[*seg_idxs.last().unwrap()],
        graph.segment_directions[*seg_idxs.last().unwrap()],
    );

    (start_side, end_side)
}

/// Build motion segments from arcs using NN ordering, MAT routing, and
/// smoothing — the same algorithm as the former geo `link_filleted_arcs`.
#[allow(clippy::too_many_arguments)]
#[prof]
fn build_link_segments(
    arcs: &[Vec<Point>],
    uncleared: &[Polygon],
    mat: Option<&MedialAxis>,
    cleared: Option<&[Polygon]>,
    cut_z: f64,
    safe_z: f64,
    safe_margin: f64,
    smoothing_amount: i32,
    preserve_order: bool,
) -> Vec<MotionSegment> {
    // Trim MAT to cleared area so travel routing only uses
    // already-machined territory.
    let mat_trimmed: Option<MedialAxis> =
        mat.and_then(|m| cleared.map(|c| m.trim_to_polygons(c)));
    let mat_ref: Option<&MedialAxis> = mat_trimmed.as_ref().or(mat);
    let mut segments: Vec<MotionSegment> = Vec::new();

    // Pre-compute obstacle bounds — these are reused across many
    // collision checks inside the smoothing / corner-cutting pipeline.
    let obstacle_bounds: Vec<Rect> = compute_polygon_bounds(uncleared);

    let order: Vec<usize> = if preserve_order || arcs.is_empty() {
        (0..arcs.len()).collect()
    } else {
        order_nearest_neighbor(arcs)
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

            let direct_seg = [last, first];
            let blocked = does_path_sweep_intersect_polygon(
                &direct_seg,
                safe_margin,
                uncleared,
                &obstacle_bounds,
            );

            let link: Vec<Point> = if blocked {
                let mat_link = mat_ref
                    .and_then(|ma| ma.path_between(last, first))
                    .unwrap_or_else(|| vec![last, first]);
                if mat_link.len() < 2 {
                    vec![last, first]
                } else {
                    build_smoothed_path(
                        last,
                        first,
                        &mat_link,
                        uncleared,
                        &obstacle_bounds,
                        safe_margin,
                        smoothing_amount,
                    )
                }
            } else {
                vec![last, first]
            };

            // Tangent extensions for G1 continuity at junctions.
            let mut link = link;
            let next_head: Vec<Point> = arc.to_vec();
            if !segments.is_empty() {
                let prev_cut = &segments.last().unwrap().points;
                let prev_tail: Vec<Point> =
                    prev_cut.iter().map(|p| Point::new(p.x, p.y)).collect();
                blend_tangent(&mut link, &prev_tail, &next_head, safe_margin);
            } else {
                blend_tangent(&mut link, &[], &next_head, safe_margin);
            }

            // Round any remaining sharp corners (including the angles
            // at the tangent extension points).
            link = chaikin_corner_cut(
                &link,
                uncleared,
                &obstacle_bounds,
                safe_margin,
                6,
            );

            // Safety net: if any post-processing introduced a
            // collision, fall back to the verified-safe path.
            if does_path_sweep_intersect_polygon(
                &link,
                safe_margin,
                uncleared,
                &obstacle_bounds,
            ) {
                if !blocked {
                    link = vec![last, first];
                } else {
                    let raw = mat_ref
                        .and_then(|ma| ma.path_between(last, first))
                        .unwrap_or_else(|| vec![last, first]);
                    link = build_smoothed_path(
                        last,
                        first,
                        &raw,
                        uncleared,
                        &obstacle_bounds,
                        safe_margin,
                        smoothing_amount,
                    );
                }
            }

            let travel_pts: Vec<Point3D> = link
                .iter()
                .map(|p| Point3D::new(p.x, p.y, safe_z))
                .collect();
            segments.push(MotionSegment {
                role: MotionRole::Travel,
                points: travel_pts,
            });

            let cut_pts: Vec<Point3D> =
                arc.iter().map(|p| Point3D::new(p.x, p.y, cut_z)).collect();
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
#[prof]
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
