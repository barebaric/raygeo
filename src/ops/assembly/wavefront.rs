//! Inside-out adaptive wavefront expansion.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::fitting::linearize_data;
use crate::geo::algo::offset::grow_geometry;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::topology::{
    split_inner_and_outer_contours, split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::{
    clean_polygon, get_circle_polygon, get_polygon_area, get_polygon_centroid,
    is_point_inside_polygon, resample_polygon,
};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;
use crate::ops::container::Ops;
use crate::ops::part::Part;
use crate::ops::state::State;
use crate::ops::types::ToolPose;
use crate::types::{Point, Point3D, Polygon};

const MAX_WAVEFRONT_ITERATIONS: usize = 1000;

/// Options for [`adaptive_wavefronts`].
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontOptions {
    pub tool_radius: f64,
    pub step_over: f64,
    pub z: f64,
    pub area_tolerance: f64,
    pub precision: f64,
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
    part: &mut Part,
    trace: &mut Tracelet,
    opts: &AdaptiveWavefrontOptions,
    cut_state: &State,
    seed: &[Polygon],
) -> RaygeoResult<AssemblyMeta> {
    let mut state_applied = false;

    let mut first_point: Option<Point> = None;
    let mut last_point: Option<Point> = None;

    // Pre-compute the envelope once and cache it on the ClearedArea.
    // The envelope (tool-centre valid area) depends only on the stock
    // region and tool radius, which are constant throughout the loop.
    // This avoids recomputing compute_inset_region in every bites()
    // and actionable_remaining() call.
    let envelope = part.cleared.envelope(&part.stock_region, opts.tool_radius);
    part.cleared.set_envelope_cache(envelope);

    // Emit pre-seeded rings through the exact same code path as
    // wavefront rings: one `move_to` + `line_to` per ring, sharing
    // the same `state_applied` flag.  No section markers, no separate
    // state application — structurally identical to wavefront output.
    for frag in seed {
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

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let bounded = part.cleared.bites(
            &part.stock_region,
            opts.step_over,
            opts.tool_radius,
            if opts.precision > 0.0 {
                opts.precision
            } else {
                0.01
            },
        );
        if bounded.is_empty() {
            break;
        }

        let new_ring = part.cleared.cut_fast(&bounded);
        if new_ring.is_empty() {
            continue;
        }

        for frag in &new_ring {
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
            // Close the fragment ring by moving back to the first point.
            trace.line_to(ring_start.x, ring_start.y, opts.z, None);
        }

        let ring_area: f64 = new_ring.iter().map(get_polygon_area).sum();
        if ring_area < opts.area_tolerance
            || part
                .cleared
                .actionable_remaining(&part.stock_region, opts.tool_radius)
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
    part: &Part,
    tool_radius: f64,
    step_over: f64,
    offset_mm: f64,
    area_tolerance: f64,
    precision: f64,
    cut_feed_rate: i32,
    cut_power: f64,
) -> RaygeoResult<(Ops, AssemblyMeta)> {
    let src_geo = part.geometry.as_ref().ok_or_else(|| {
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
                let poly = geometry_to_polygon(&geo, 0.01);
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
        // Find the largest inscribed circle to seed the cleared area.
        let (center, r_max) = find_largest_circle(boundary, islands, 0.1)
            .unwrap_or_else(|| (get_polygon_centroid(boundary), 0.0));

        // Seed with concentric rings spaced `step_over` apart so the
        // wavefront expands smoothly from the pocket centre.  The
        // outermost ring is the frontier; the first wavefront bite
        // expands it by exactly `step_over`, giving an equidistant,
        // seamless transition into the wavefront phase.
        //
        // The rings cover the inscribed disk up to the largest radius
        // where the tool centre can safely travel, leaving the
        // wavefront to fill the remaining (typically irregular) rim.
        let seed_r = tool_radius.min(r_max * 0.8).max(0.01);
        let spiral_max_r = (r_max - tool_radius).max(seed_r);

        let mut seed_polys: Vec<Polygon> = Vec::new();
        if step_over > 0.0 {
            let mut r = seed_r;
            while r <= spiral_max_r {
                seed_polys.push(get_circle_polygon(center, r, 64));
                r += step_over;
            }
            // Always include the outermost ring so the frontier sits
            // at the maximum seed radius.
            if seed_polys.is_empty() {
                seed_polys.push(get_circle_polygon(center, spiral_max_r, 64));
            }
        } else {
            seed_polys.push(get_circle_polygon(center, seed_r, 64));
        }

        let mut pocket_part = Part::from_polygons_initial(
            boundary,
            islands,
            &seed_polys,
            (0.0, 0.0),
        );

        // Expand with adaptive wavefronts
        let wf_opts = AdaptiveWavefrontOptions {
            tool_radius,
            step_over,
            z,
            area_tolerance,
            precision,
        };
        let meta = adaptive_wavefronts(
            &mut pocket_part,
            &mut trace,
            &wf_opts,
            &cut_state,
            &seed_polys,
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

/// Convert a single Geometry contour to a 2D polygon.
fn geometry_to_polygon(geo: &Geometry, tolerance: f64) -> Polygon {
    let mut linearized = geo.copy();
    if !linearized.data.is_empty() {
        linearized.data = linearize_data(&linearized.data, tolerance);
    }
    let segs = linearized.segments();
    for seg in &segs {
        if seg.len() < 3 {
            continue;
        }
        let poly: Polygon = seg.iter().map(|p| Point::new(p.x, p.y)).collect();
        if let Some(cleaned) = clean_polygon(&poly, 0.01 * tolerance) {
            return cleaned;
        } else if poly.len() >= 3 {
            return poly;
        }
    }
    Polygon::new()
}
