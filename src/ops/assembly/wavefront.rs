//! Inside-out adaptive wavefront expansion.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::offset::grow_geometry;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::topology::{
    split_inner_and_outer_contours, split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::{
    get_circle_polygon, get_polygon_area, get_polygon_centroid,
    get_polygon_signed_area, get_polygons_closest_point,
    is_point_inside_polygon, resample_polygon,
};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::container::Ops;
use crate::ops::part::{FaceState, Part};
use crate::ops::state::State;
use crate::ops::types::ToolPose;
use crate::types::{Point, Point3D, Polygon};

const MAX_WAVEFRONT_ITERATIONS: usize = 1000;

/// Spec for the inside-out adaptive wavefront assembler.
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontSpec {
    pub step_over: f64,
    pub z: f64,
    pub area_tolerance: f64,
    pub precision: f64,
}

impl Assembler for AdaptiveWavefrontSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "wavefront: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = adaptive_wavefronts(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "wavefront: done");
        Ok(meta)
    }

    fn name(&self) -> &'static str {
        "wavefront"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Inside-out adaptive wavefronts.
///
/// Finds the largest inscribed circle in the stock region, seeds the
/// cleared area with concentric rings spaced `step_over` apart, and
/// then iteratively expands the frontier outward by `step_over`,
/// clipping to the boundary, until the pocket is fully cleared.
///
/// Each ring fragment is emitted as `MoveTo` (first point) + `LineTo`
/// (rest), all at height `z`, with `cut_state` applied.
#[prof]
pub fn adaptive_wavefronts(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &AdaptiveWavefrontSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let mut state_applied = false;

    let mut first_point: Option<Point> = None;
    let mut last_point: Option<Point> = None;

    // Pre-compute the envelope once and cache it on the ClearedArea.
    // The envelope (tool-centre valid area) depends only on the stock
    // region, which is constant throughout the loop.
    // This avoids recomputing compute_inset_region in every bites()
    // and actionable_remaining() call.
    let envelope = face.cleared.envelope(&face.stock_region, 0.0);
    face.cleared.set_envelope_cache(envelope);

    // Seed: find the largest inscribed circle and emit concentric rings.
    let (center, r_max) = find_largest_circle(
        &face.stock_region.boundary,
        &face.stock_region.islands,
        0.1,
    )
    .unwrap_or_else(|| {
        (get_polygon_centroid(&face.stock_region.boundary), 0.0)
    });
    let seed_r = (0.01_f64).max(r_max * 0.02);
    let spiral_max_r = r_max.max(seed_r);
    let mut seed_polys: Vec<Polygon> = Vec::new();
    if opts.step_over > 0.0 {
        let mut r = seed_r;
        while r <= spiral_max_r {
            seed_polys.push(get_circle_polygon(center, r, 64));
            r += opts.step_over;
        }
        if seed_polys.is_empty() {
            seed_polys.push(get_circle_polygon(center, spiral_max_r, 64));
        }
    } else {
        seed_polys.push(get_circle_polygon(center, seed_r, 64));
    }
    face.cleared = crate::ops::part::cleared_area::ClearedArea::with_fragments(
        &seed_polys,
    );

    // Emit seed rings through the exact same code path as wavefront
    // rings: one `move_to` + `line_to` per ring, sharing the same
    // `state_applied` flag.
    for frag in &seed_polys {
        let frag_area = get_polygon_area(frag);
        if frag_area < opts.area_tolerance {
            continue;
        }
        let points: Vec<Point> = if opts.precision > 0.0 {
            resample_polygon(frag, opts.precision)
        } else {
            frag.clone()
        };
        if points.len() < 3 {
            continue;
        }
        if !state_applied {
            trace.apply_state(cut_state);
            state_applied = true;
        }
        if first_point.is_none() {
            first_point = Some(points[0]);
        }
        last_point = Some(points[points.len() - 1]);
        let ring_start = points[0];
        trace.move_to(ring_start.x, ring_start.y, opts.z, None);
        for p in &points[1..] {
            trace.line_to(p.x, p.y, opts.z, None);
        }
        trace.line_to(ring_start.x, ring_start.y, opts.z, None);
    }

    let simplify_tol = if opts.precision > 0.0 {
        opts.precision
    } else {
        0.01
    };

    // Snapshot the frontier before the loop.  Each iteration filters out
    // band-polygon points that lie close to the previous frontier (the
    // inner edge of the annular band — already cut) and splits the
    // remaining outer-edge points into separate runs so that gaps become
    // travel moves rather than cut lines.
    let mut prev_frontier: Vec<Polygon> =
        face.cleared.frontier(&face.stock_region, simplify_tol);

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let bounded = face.cleared.bites(
            &face.stock_region,
            opts.step_over,
            0.0,
            simplify_tol,
        );
        if bounded.is_empty() {
            break;
        }

        let new_ring = face.cleared.cut_fast(&bounded);
        if new_ring.is_empty() {
            continue;
        }

        let threshold_sq = (opts.step_over * 0.5).powi(2);

        for frag in &new_ring {
            // Skip CW-wound holes (Clipper2 difference output: CCW
            // outer rings are new material, CW holes are the
            // already-cleared boundary).
            if get_polygon_signed_area(frag) <= 0.0 {
                continue;
            }
            // Skip fragments that are too small to be meaningful.
            let frag_area = get_polygon_area(frag);
            if frag_area < opts.area_tolerance {
                continue;
            }
            let points: Vec<Point> = if opts.precision > 0.0 {
                resample_polygon(frag, opts.precision)
            } else {
                frag.clone()
            };
            if points.len() < 3 {
                continue;
            }

            // Keep only points far enough from the previous frontier
            // boundary.  Points close to the previous frontier are on
            // the inner edge of the band (already cut).  Using
            // point-to-boundary distance (not point-to-vertex) handles
            // cases where `simplify_polyline` produces different vertex
            // positions along the same curve.
            let keep: Vec<bool> = points
                .iter()
                .map(|p| {
                    let min_dist_sq =
                        match get_polygons_closest_point(&prev_frontier, *p) {
                            Some((_, _, _, d2)) => d2,
                            None => f64::MAX,
                        };
                    min_dist_sq > threshold_sq
                })
                .collect();

            // Split into contiguous runs so that gaps (filtered-out
            // points) become travel moves rather than cut lines.
            let n = points.len();
            let mut runs: Vec<Vec<Point>> = Vec::new();
            let mut current: Vec<Point> = Vec::new();
            for (i, &p) in points.iter().enumerate() {
                if keep[i] {
                    current.push(p);
                } else if !current.is_empty() {
                    runs.push(std::mem::take(&mut current));
                }
            }
            if !current.is_empty() {
                runs.push(current);
            }

            // Merge first and last runs if the polygon seam wraps.
            if runs.len() >= 2 && keep[0] && keep[n - 1] {
                let last = runs.pop().unwrap();
                runs[0].splice(0..0, last);
            }

            let all_kept = keep.iter().all(|&k| k);
            for (ri, run) in runs.iter().enumerate() {
                if run.len() < 2 {
                    continue;
                }
                if !state_applied {
                    trace.apply_state(cut_state);
                    state_applied = true;
                }
                if first_point.is_none() {
                    first_point = Some(run[0]);
                }
                last_point = Some(run[run.len() - 1]);
                let ring_start = run[0];
                trace.move_to(ring_start.x, ring_start.y, opts.z, None);
                for p in &run[1..] {
                    trace.line_to(p.x, p.y, opts.z, None);
                }
                // Close only if no points were filtered (inner edge
                // not present).  Filtered runs are open arcs where a
                // closing line would cut across the gap — the D shape.
                if all_kept && ri == 0 {
                    trace.line_to(ring_start.x, ring_start.y, opts.z, None);
                }
            }
        }

        prev_frontier = face.cleared.frontier(&face.stock_region, simplify_tol);

        let ring_area: f64 =
            new_ring.iter().map(|p| get_polygon_signed_area(p)).sum();
        if ring_area < opts.area_tolerance
            || face.cleared.actionable_remaining(&face.stock_region, 0.0)
                < opts.area_tolerance
        {
            break;
        }
    }

    let start_pos = first_point.unwrap_or(Point::ZERO);
    let end_pos = last_point.unwrap_or(Point::ZERO);

    Ok(AssemblyMeta {
        start: ToolPose {
            pos: Point3D::new(start_pos.x, start_pos.y, opts.z),
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::new(end_pos.x, end_pos.y, opts.z),
            heading: 0.0,
        },
    })
}

// ── Multi-pocket wrapper ───────────────────────────────────────────

/// Multi-pocket adaptive wavefronts.
///
/// Extracts all pockets from `part.geometry`, optionally offsets the
/// boundary inward, seeds each pocket with concentric rings spaced
/// `step_over` apart, and runs wavefront expansion inside each.
/// Returns the combined `Ops`.
#[allow(clippy::too_many_arguments)]
#[prof]
pub fn adaptive_wavefronts_multi_pocket(
    face: &FaceState,
    step_over: f64,
    offset_mm: f64,
    area_tolerance: f64,
    precision: f64,
    cut_feed_rate: i32,
    cut_power: f64,
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let src_geo = face.geometry.as_ref().ok_or_else(|| {
        crate::RaygeoError::ContourError(
            "adaptive_wavefronts_multi_pocket requires a part with geometry"
                .to_string(),
        )
    })?;

    // 1. Apply optional inward offset
    let geo = if offset_mm > 0.0 {
        grow_geometry(src_geo, -offset_mm)
    } else {
        src_geo.copy()
    };

    // 2. Split into contours, keep only closed ones
    let contours = split_into_contours(&geo);
    let closed: Vec<Geometry> =
        contours.into_iter().filter(|c| c.is_closed(1e-6)).collect();
    if closed.is_empty() {
        return Err(crate::RaygeoError::ContourError(
            "No closed contours found in workpiece geometry".to_string(),
        ));
    }

    let closed_refs: Vec<&Geometry> = closed.iter().collect();

    // 3. Split inner / outer
    let (inner_idx, outer_idx) = split_inner_and_outer_contours(&closed_refs);
    if outer_idx.is_empty() {
        return Err(crate::RaygeoError::ContourError(
            "No outer boundary contour found in workpiece geometry".to_string(),
        ));
    }

    // 4. Convert to polygons for pocket association
    let to_poly = |idx: &[usize]| -> Vec<(Geometry, Polygon)> {
        idx.iter()
            .map(|&i| {
                let geo = closed[i].copy();
                let poly = geo
                    .to_polygons(0.01)
                    .into_iter()
                    .next()
                    .unwrap_or_default();
                (geo, poly)
            })
            .collect()
    };
    let outer_pairs = to_poly(&outer_idx);
    let inner_pairs = to_poly(&inner_idx);

    // 5. Associate pockets
    let mut used_inner = vec![false; inner_pairs.len()];
    let mut pockets: Vec<(Polygon, Vec<Polygon>)> = Vec::new();

    for (_, outer_poly) in &outer_pairs {
        let mut islands: Vec<Polygon> = Vec::new();
        for (j, (_, inner_poly)) in inner_pairs.iter().enumerate() {
            if used_inner[j] || inner_poly.len() < 3 {
                continue;
            }
            let cx = get_polygon_centroid(inner_poly);
            if is_point_inside_polygon(cx, outer_poly) {
                islands.push(inner_poly.clone());
                used_inner[j] = true;
            }
        }
        pockets.push((outer_poly.clone(), islands));
    }

    // 6. Process each pocket
    let z = 0.0;
    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };
    let mut combined_ops = Ops::new();
    let mut trace = Tracelet::new();
    let mut first_point: Option<Point3D> = None;
    let mut last_point: Option<Point3D> = None;

    for (boundary, islands) in &pockets {
        let mut pocket_part =
            Part::from_polygons(boundary, islands, (0.0, 0.0));

        let wf_opts = AdaptiveWavefrontSpec {
            step_over,
            z,
            area_tolerance,
            precision,
        };
        let meta = adaptive_wavefronts(
            pocket_part.face_mut(""),
            &mut trace,
            &wf_opts,
            &cut_state,
        )?;

        if first_point.is_none() {
            first_point = Some(meta.start.pos);
        }
        last_point = Some(meta.end.pos);
    }

    combined_ops.extend(trace.ops());

    let start_pos = first_point.unwrap_or(Point3D::ZERO);
    let end_pos = last_point.unwrap_or(Point3D::ZERO);

    Ok((
        combined_ops,
        AssemblyMeta {
            start: ToolPose {
                pos: start_pos,
                heading: 0.0,
            },
            end: ToolPose {
                pos: end_pos,
                heading: 0.0,
            },
        },
    ))
}
