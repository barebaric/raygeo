//! Drilling plan builder.
//!
//! [`plan_drilling`] takes a [`Part`] and options and produces a plan
//! that drills holes: one cycle per hole, all sharing the same
//! parameters. Holes either come from the caller as explicit centers
//! or are detected automatically: circular island contours of the
//! face (via [`find_holes`]) count as holes. Holes needing different
//! depths or parameters need a separate plan.
//!
//! Travel between holes is left to the executor: the steps carry only
//! the per-hole cycles, and the sequential link retracts to `safe_z`,
//! traverses and approaches the next hole.

use std::sync::Arc;

use prof_macros::prof;

use crate::cnc::plan::plan::{Plan, PlanStep};
use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::types::Point;
use crate::ops::assembly::drill::{DrillSpec, PeckMode, PeckParams};
use crate::ops::feature::hole::{find_holes, Hole};
use crate::ops::part::Part;
use crate::ops::CoolantMode;

/// Which hole-making cycle to plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrillCycleKind {
    /// Single plunge to depth.
    Plain,
    /// Shallow spot/marking cycle (same moves as plain).
    Spot,
    /// Peck cycle with full retract per peck (G83 semantics).
    Peck,
    /// Chip-break peck cycle (G73 semantics).
    ChipBreak,
}

/// Hole ordering for the plan's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoleOrder {
    /// Order of detection / as passed.
    DetectionOrder,
    /// Greedy nearest-neighbour chain starting at the first hole,
    /// minimising travel.
    NearestNeighbor,
}

/// Options for [`plan_drilling`].
pub struct DrillingWorkplanOptions {
    pub cycle: DrillCycleKind,
    /// Drill (and hole) diameter.
    pub diameter: f64,
    /// Drill point angle in degrees, for tip compensation.
    pub tip_angle: f64,
    /// Surface Z the holes start at.
    pub z_surface: f64,
    /// Full-diameter depth of the holes.
    pub target_z: f64,
    /// Rapid plane just above the surface.
    pub rapid_z: f64,
    pub safe_z: f64,
    /// Axial depth per peck (Peck / ChipBreak only).
    pub peck_increment: f64,
    /// Chip-break retract distance (ChipBreak only).
    pub chip_break_retract: f64,
    /// Dwell at the bottom of every plunge (ms).
    pub dwell_ms: f64,
    /// Cut feed (mm/min); `None` leaves the plan's cut state unset.
    pub feed_rate: Option<f64>,
    /// Vertical plunge feed (mm/min); `None` falls back to the cut feed.
    pub plunge_feed: Option<f64>,
    pub spindle_rpm: Option<u32>,
    pub coolant: Option<CoolantMode>,
    /// Explicit hole centers. When `None`, holes are detected as
    /// circular island contours of the face. Islands smaller than the
    /// drill diameter (plus 0.01 mm) are skipped in detection mode —
    /// the drill must fit.
    pub holes: Option<Vec<Point>>,
    /// Max distance (mm) between an island contour sample and the
    /// fitted circle for it to count as a hole.
    pub hole_tolerance: f64,
    pub hole_order: HoleOrder,
}

/// Builds a drilling plan for one face of `part`.
#[prof]
pub fn plan_drilling(
    part: &Part,
    face_id: &str,
    opts: &DrillingWorkplanOptions,
) -> RaygeoResult<Plan> {
    if opts.diameter <= 0.0 {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "diameter must be above 0, got {}",
            opts.diameter
        )));
    }
    if matches!(opts.cycle, DrillCycleKind::Peck | DrillCycleKind::ChipBreak)
        && opts.peck_increment <= 0.0
    {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "peck_increment must be above 0, got {}",
            opts.peck_increment
        )));
    }
    let face = part.face(face_id).ok_or_else(|| {
        RaygeoError::DegenerateGeometry(format!(
            "the part has no face {face_id:?}"
        ))
    })?;
    let (boundary, islands) = face.extract_boundary();
    let holes = resolve_holes(
        opts.holes.as_deref(),
        &islands,
        opts.hole_tolerance,
        opts.diameter,
    )?;
    let holes = match opts.hole_order {
        HoleOrder::DetectionOrder => holes,
        HoleOrder::NearestNeighbor => order_holes_nearest_neighbor(holes),
    };

    let mut plan =
        Plan::new(boundary.unwrap_or_default(), islands, opts.safe_z);
    for hole in &holes {
        plan.steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(make_drill_spec(opts, hole.center)),
            region_boundary: None,
        });
    }
    plan.cut_state.feed_rate = opts.feed_rate.map(|f| f.round() as i32);
    plan.cut_state.plunge_feed = opts.plunge_feed.map(|f| f.round() as i32);
    plan.cut_state.spindle_rpm = opts.spindle_rpm;
    plan.cut_state.coolant = opts.coolant;
    Ok(plan)
}

/// Build the per-hole assembler spec for the configured cycle.
fn make_drill_spec(opts: &DrillingWorkplanOptions, center: Point) -> DrillSpec {
    let peck = match opts.cycle {
        DrillCycleKind::Peck => Some(PeckParams {
            mode: PeckMode::FullRetract,
            increment: opts.peck_increment,
            retract_amount: 0.0,
        }),
        DrillCycleKind::ChipBreak => Some(PeckParams {
            mode: PeckMode::ChipBreak,
            increment: opts.peck_increment,
            retract_amount: opts.chip_break_retract,
        }),
        _ => None,
    };
    DrillSpec {
        center,
        diameter: opts.diameter,
        tip_angle: opts.tip_angle,
        z_surface: opts.z_surface,
        target_z: opts.target_z,
        rapid_z: opts.rapid_z,
        safe_z: opts.safe_z,
        peck,
        dwell_ms: opts.dwell_ms,
        spot: opts.cycle == DrillCycleKind::Spot,
        tip_compensate: true,
    }
}

/// Get the holes to machine: explicit centers (diameter taken from
/// the drill) or detected circular islands (filtered to those the
/// drill fits).
///
/// Shared with the thread-milling planner, which passes its own
/// `min_diameter`.
pub(crate) fn resolve_holes(
    explicit: Option<&[Point]>,
    islands: &[crate::geo::types::Polygon],
    tolerance: f64,
    min_diameter: f64,
) -> RaygeoResult<Vec<Hole>> {
    if let Some(centers) = explicit {
        if centers.is_empty() {
            return Err(RaygeoError::DegenerateGeometry(
                "the explicit hole list is empty".into(),
            ));
        }
        return Ok(centers
            .iter()
            .map(|&center| Hole {
                center,
                diameter: min_diameter,
            })
            .collect());
    }
    let detected = find_holes(islands, tolerance);
    let fitting: Vec<Hole> = detected
        .into_iter()
        .filter(|hole| hole.diameter >= min_diameter - 0.01)
        .collect();
    if fitting.is_empty() {
        return Err(RaygeoError::DegenerateGeometry(
            "no holes found: the face has no circular islands (fitting \
             the drill diameter)"
                .into(),
        ));
    }
    Ok(fitting)
}

/// Greedy nearest-neighbour chain starting at the first hole.
pub(crate) fn order_holes_nearest_neighbor(mut holes: Vec<Hole>) -> Vec<Hole> {
    if holes.is_empty() {
        return holes;
    }
    let mut ordered = Vec::with_capacity(holes.len());
    let mut current = holes.remove(0);
    ordered.push(current);
    while !holes.is_empty() {
        let (best, _) = holes
            .iter()
            .enumerate()
            .map(|(i, h)| (i, current.center.distance(h.center)))
            .min_by(|a, b| {
                a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("holes is non-empty");
        current = holes.remove(best);
        ordered.push(current);
    }
    ordered
}
