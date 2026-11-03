//! Slot-style erosion of residual uncut clumps.
//!
//! The adaptive wavefront's convergence gate insets the remaining
//! material by the step scale before intersecting it with the
//! tool-centre envelope, so sub-step-scale clumps — the ~1 mm-wide
//! slivers a high step-over strands between passes — read as converged
//! and are never machined. This pass runs after the clearing steps: it
//! finds the uncut material inside the tool-centre envelope (a
//! phantom-scale inset drops the sweep-approximation film), slots each
//! clump along a spine carrier, and records the cut footprint in the
//! cleared-area model. When nothing actionable remains it emits no ops
//! at all.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::clipping::clip_line_segment_with_polygons_2d;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_convex_hull, get_polygon_signed_area,
    get_polygons_group_difference, get_polygons_group_intersection,
    get_polyline_swept_polygon, offset_polygon, JoinStyle,
};
use crate::geo::types::{Point, Point3D, Polygon};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::trace_utils as tu;
use crate::ops::assembly::tracelet::write_polyline;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::ToolPose;

/// Inset that drops sweep-approximation film: successive capsule sweeps
/// of the same boundary overlap to within ~40 µm scallops, so anything
/// thinner than this band is numerical residue, not material. The
/// detection deliberately does NOT inset by the step scale — that is
/// exactly the blindness this pass exists to repair.
const PHANTOM_SCALE: f64 = 0.05;

/// Residual pieces smaller than this are film, not material.
const CLUMP_MIN_AREA: f64 = 0.05;

/// Upper bound on the spine carriers fitted per clump.
const MAX_CARRIERS: usize = 12;

/// Spec for the erosion assembler.
#[derive(Clone, Debug)]
pub struct ErosionSpec {
    pub tool_radius: f64,
    pub target_z: f64,
    /// Travel height for the retract between carriers.
    pub safe_z: f64,
    /// Leave the pocket alone when the actionable residual totals less
    /// than this area (mm²) — the same tolerance the clearing pass
    /// converges against.
    pub area_tolerance: f64,
}

impl Assembler for ErosionSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "erosion: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_erosion(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "erosion: done");
        Ok(meta)
    }

    fn name(&self) -> &str {
        "erosion"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Slot the uncut clumps inside the tool-centre envelope.
///
/// Detects the residual the clearing pass left, fits a spine carrier
/// per clump, machines a slot along it at `target_z`, and records the
/// swept footprint in the cleared-area model. Emits no ops when the
/// residual totals less than `area_tolerance`.
#[prof]
pub fn generate_erosion(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &ErosionSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let zero = ToolPose {
        pos: Point3D::ZERO,
        heading: 0.0,
    };
    let idle = || AssemblyMeta {
        start: zero,
        end: zero,
    };
    if face.cleared.is_empty() {
        return Ok(idle());
    }
    let envelope = face.cleared.envelope(&face.stock_region, opts.tool_radius);
    if envelope.is_empty() {
        return Ok(idle());
    }

    // Uncut material inside the tool-centre envelope — the wall band
    // between the envelope and the walls belongs to the finishing
    // profile — minus the sweep-approximation film. The inset flips
    // sign per winding so hole rings shrink their material too.
    let inside = get_polygons_group_intersection(
        &face.cleared.remaining(&face.stock_region),
        &envelope,
    );
    let mut clumps: Vec<Polygon> = inside
        .iter()
        .flat_map(|p| offset_polygon(p, inset_delta(p), JoinStyle::Round))
        .filter(|p| p.len() >= 3 && get_polygon_area(p) >= CLUMP_MIN_AREA)
        .collect();
    clumps.sort_by(|a, b| {
        crate::utils::sort_f64(get_polygon_area(b), get_polygon_area(a))
    });
    let residual: f64 = clumps.iter().map(get_polygon_area).sum();
    if residual < opts.area_tolerance {
        return Ok(idle());
    }

    let z = opts.target_z;
    let carriers: Vec<Vec<Point>> = clumps
        .iter()
        .flat_map(|c| spine_carriers(c, &envelope, opts.tool_radius))
        .collect();

    let mut bands: Vec<Polygon> = Vec::new();
    let mut start_pose: Option<ToolPose> = None;
    let mut end_pose = zero;
    for (i, carrier) in carriers.iter().enumerate() {
        // Plunge in cleared territory near the carrier start, like the
        // slot pass does, so no carrier begins as a full-depth plunge
        // into the clump it is about to erode.
        let mut full_carrier = carrier.clone();
        if let Some(&first) = carrier.first() {
            if let Some(plunge) = face.cleared.find_plunge_point(
                &face.stock_region,
                first,
                opts.tool_radius,
                opts.tool_radius * 3.0,
            ) {
                full_carrier.insert(0, plunge);
            }
        }

        // Between carriers: retract, travel, and hand the descent to
        // the polyline's own first line_to (the pipeline links into the
        // first carrier from above).
        if i > 0 {
            let prev = &carriers[i - 1];
            let from = prev[prev.len() - 1];
            trace.move_to(from.x, from.y, opts.safe_z, None);
            let p0 = full_carrier[0];
            trace.move_to(p0.x, p0.y, opts.safe_z, None);
        }

        // Forward pass, then backward: the backward leg doubles the
        // coverage of a spine that is slightly off the clump's centre.
        let n = full_carrier.len();
        let mut path: Vec<Point3D> = Vec::with_capacity(n * 2);
        for p in &full_carrier {
            path.push(Point3D::new(p.x, p.y, z));
        }
        for p in full_carrier.iter().rev().skip(1) {
            path.push(Point3D::new(p.x, p.y, z));
        }

        if start_pose.is_none() {
            start_pose = Some(ToolPose {
                pos: path[0],
                heading: tu::path_heading(&path, 0),
            });
        }
        end_pose = ToolPose {
            pos: path[path.len() - 1],
            heading: tu::path_heading(&path, path.len() - 1),
        };

        write_polyline(trace, &path, i == 0, Some(cut_state));
        bands.extend(get_polyline_swept_polygon(
            &full_carrier,
            opts.tool_radius,
        ));
    }

    // Record the eroded footprint so downstream steps see an honest
    // cleared area.
    face.cleared.cut(&bands);

    Ok(AssemblyMeta {
        start: start_pose.unwrap_or(zero),
        end: end_pose,
    })
}

/// The film-removal inset for one ring of a difference output: inward
/// for CCW outers, outward for CW holes — either way the material side
/// shrinks by [`PHANTOM_SCALE`].
fn inset_delta(ring: &Polygon) -> f64 {
    if get_polygon_signed_area(ring) < 0.0 {
        PHANTOM_SCALE
    } else {
        -PHANTOM_SCALE
    }
}

/// Fit slot carriers that cover `clump`: the longest line through the
/// clump's hull, clipped to the tool-centre envelope, then repeated on
/// whatever the tool-width sweep around the carriers still leaves
/// uncovered (bounded, so a pathological clump degrades into a mini
/// raster instead of looping forever).
fn spine_carriers(
    clump: &Polygon,
    envelope: &[Polygon],
    tool_radius: f64,
) -> Vec<Vec<Point>> {
    let mut carriers: Vec<Vec<Point>> = Vec::new();
    let mut uncovered: Vec<Polygon> = vec![clump.clone()];
    for _ in 0..MAX_CARRIERS {
        let piece = uncovered.iter().filter(|p| p.len() >= 3).max_by(|a, b| {
            crate::utils::sort_f64(get_polygon_area(a), get_polygon_area(b))
        });
        let piece = match piece {
            Some(p) if get_polygon_area(p) >= CLUMP_MIN_AREA => p,
            _ => break,
        };

        let (a, b) = farthest_pair(&get_polygon_convex_hull(piece));
        let clipped = clip_line_segment_with_polygons_2d(a, b, envelope);
        let mut extended = false;
        for (s, e) in clipped {
            if s.distance(e) <= 1e-9 {
                continue;
            }
            carriers.push(vec![s, e]);
            extended = true;
        }
        if !extended {
            break;
        }

        let band = get_polyline_swept_polygon(
            carriers[carriers.len() - 1].as_slice(),
            tool_radius,
        );
        uncovered = get_polygons_group_difference(&uncovered, &band);
        let left: f64 = uncovered.iter().map(get_polygon_area).sum();
        if left < CLUMP_MIN_AREA {
            break;
        }
    }
    carriers
}

/// The two farthest-apart points on a convex polygon.
fn farthest_pair(hull: &Polygon) -> (Point, Point) {
    let mut best = (hull[0], hull[hull.len() - 1]);
    let mut best_d = -1.0_f64;
    for i in 0..hull.len() {
        for j in (i + 1)..hull.len() {
            let d = hull[i].distance(hull[j]);
            if d > best_d {
                best_d = d;
                best = (hull[i], hull[j]);
            }
        }
    }
    best
}
