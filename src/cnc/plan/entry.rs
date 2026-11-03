//! CNC entry strategy orchestration.
//!
//! [`plan_entry`] takes a single [`Region`] (wide sub-region with an
//! entry point and max inscribed radius) and produces the appropriate
//! entry [`PlanStep`]s.  Region detection is the caller's
//! responsibility — use [`find_regions`](crate::ops::feature::region::find_regions)
//! to obtain regions.
//!
//! Execution is the job of the pipeline — see
//! [`create_intent`](crate::cnc::execution::intent::create_intent).

use std::sync::Arc;

use prof_macros::prof;

use crate::cnc::plan::plan::PlanStep;
use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::shape::line::longest_line_through_point;
use crate::geo::shape::polygon::get_polygon_bounds;
use crate::geo::types::{Point3D, Polygon};
use crate::ops::assembly::helix::HelixSpec;
use crate::ops::assembly::ramp::RampSpec;
use crate::ops::assembly::spiral::SpiralSpec;
use crate::ops::assembly::toroid::ToroidalClearSpec;
use crate::ops::feature::ramp::find_ramp_carrier;
use crate::ops::feature::region::Region;

pub struct EntryWorkplanOptions {
    pub islands: Vec<Polygon>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub safe_z: f64,
    pub target_z: f64,
    pub plunge_pitch: f64,
    pub safe_margin: f64,
    pub angular_step: f64,
    /// Extra clearance the entries must keep from the islands. The region
    /// polygon arrives eroded by the wall margin (walls carry it), but the
    /// islands are raw — the ramp fallback buffers them by
    /// `tool_radius + wall_margin` so no entry cut sneaks past the margin.
    pub wall_margin: f64,
    /// Steepest descent the entry may make, in degrees. `<= 0` keeps the
    /// planner's historical behaviour: the helix descends `plunge_pitch`
    /// per revolution and the ramp entries cap at 45°. A positive angle
    /// derives the helix pitch from it and caps both ramp entries at it,
    /// so a caller can ask for gentle lead-ins regardless of which
    /// strategy a region's shape selects.
    pub max_entry_angle_deg: f64,
}

/// Plan entry steps for a single wide [`Region`].
///
/// Strategy is chosen based on `region.r_max`:
/// - **Helix + spiral** when `r_max >= 2 × tool_diameter` (room for a
///   helix that clears on descent then spirals out).
/// - **Toroidal ramp** when [`find_ramp_carrier`] finds a usable
///   carrier line inside the region polygon.
/// - **Zigzag ramp** as a last-resort fallback.
///
/// Each step targets the default face (`""`). The caller may change
/// `face_id` on the returned steps for multi-face parts.
#[prof]
pub fn plan_entry(
    region: &Region,
    opts: &EntryWorkplanOptions,
    face_id: &str,
) -> RaygeoResult<Vec<PlanStep>> {
    let mut steps = Vec::new();
    let tool_diameter = 2.0 * opts.tool_radius;

    if region.r_max >= 2.0 * tool_diameter {
        let helix_r = (opts.tool_radius * 0.8).min(region.r_max * 0.5);
        // A positive entry angle turns the requested slope into the
        // pitch: one revolution drops pitch = tan(angle) · 2πr.
        let pitch = if opts.max_entry_angle_deg > 0.0 {
            opts.max_entry_angle_deg.to_radians().tan()
                * std::f64::consts::TAU
                * helix_r
        } else {
            opts.plunge_pitch
        };

        steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(HelixSpec {
                center: region.entry_pt,
                start_radius: helix_r,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                pitch,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            }),
            region_boundary: None,
        });

        let spiral_max_r = (region.r_max - opts.tool_radius - opts.safe_margin)
            .max(helix_r + 0.01);
        if spiral_max_r > 0.0 && opts.step_over > 0.0 {
            let revolutions = spiral_max_r / opts.step_over;
            // The spiral starts at the center, not at the helix radius:
            // the helix machines the floor only along the final arc of
            // its last revolution (the descent above the floor leaves a
            // film over the rest of the seed disk), so a spiral that
            // starts at the helix radius would strand the crescent
            // between the seed disk and its first outward turn. From the
            // center out, the spiral's swept band covers the whole seed
            // disk the helix claims, and the cleared-area model stays
            // honest.
            steps.push(PlanStep {
                face_id: face_id.to_string(),
                spec: Arc::new(SpiralSpec {
                    center: region.entry_pt,
                    z: opts.target_z,
                    start_radius: 0.0,
                    end_radius: spiral_max_r,
                    revolutions,
                    direction: HelixDirection::Cw,
                    angular_step: opts.angular_step,
                    start_angle: 0.0,
                }),
                region_boundary: None,
            });
        }
    } else if let Some((start, end)) = find_ramp_carrier(
        &region.polygon,
        &opts.islands,
        opts.tool_radius,
        entry_angle_cap(opts),
        // The trochoid about the carrier swings `lateral_swing` to each
        // side; keep the swing inside the tool-centre region.
        crate::geo::algo::trochoid::trochoid_lateral_swing(
            opts.tool_radius * 2.0,
            opts.step_over / (opts.tool_radius * 2.0),
            30.0,
            (opts.step_over * 0.3).max(0.05),
        ),
    ) {
        steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(ToroidalClearSpec {
                carrier: vec![start, end],
                start: Point3D::new(start.x, start.y, opts.safe_z),
                target_z: opts.target_z,
                tool_radius: opts.tool_radius,
                step_over: opts.step_over,
                max_ramp_angle_deg: entry_angle_cap(opts),
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            }),
            region_boundary: None,
        });
    } else {
        let ((start, end), amplitude) = ramp_entry(
            region,
            &opts.islands,
            opts.tool_radius,
            opts.tool_radius + opts.wall_margin,
        );
        steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(RampSpec {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: entry_angle_cap(opts),
                style: crate::geo::algo::ramp::RampStyle::ZigZag,
                lateral_amplitude: amplitude,
                tool_radius: opts.tool_radius,
            }),
            region_boundary: None,
        });
    }

    Ok(steps)
}

/// The descent cap the ramp entries honour: the caller's entry angle
/// when one was requested, the planner's historical 45° otherwise.
fn entry_angle_cap(opts: &EntryWorkplanOptions) -> f64 {
    if opts.max_entry_angle_deg > 0.0 {
        opts.max_entry_angle_deg
    } else {
        45.0
    }
}

/// A zigzag ramp entry that stays inside the region: the carrier is the
/// longest segment through the entry point in the tool-centre region, and
/// the zigzag's lateral amplitude is clamped to the segment's clearance
/// inside that region (clamping to a straight ramp when the region is too
/// narrow to swing at all).
///
/// `island_clearance` is the distance the carrier and its swing keep from
/// the islands — the tool radius plus the wall margin, since the islands
/// arrive raw while the region polygon already carries the margin.
#[allow(clippy::too_many_arguments)]
fn ramp_entry(
    region: &Region,
    islands: &[Polygon],
    tool_radius: f64,
    island_clearance: f64,
) -> ((crate::geo::types::Point, crate::geo::types::Point), f64) {
    use crate::geo::shape::polygon::get_signed_boundary_distance;

    let (start, end) = ramp_segment_in_region(
        region.entry_pt,
        &region.polygon,
        islands,
        tool_radius,
        island_clearance,
    );
    let amplitude = tool_radius * 0.8;
    // How far the segment stays inside the tool-centre region: the
    // zigzag swings ±amplitude about it. Interior samples only — the
    // clipped segment's endpoints sit on the boundary where the
    // clearance is 0, which would clamp the amplitude to a straight
    // ramp even when the whole interior has room to swing.
    let valid = tool_centre_region(
        &region.polygon,
        islands,
        tool_radius,
        island_clearance,
    );
    let mut d_min = f64::INFINITY;
    for k in 1..32 {
        let t = k as f64 / 32.0;
        let p = start.lerp(end, t);
        let d = -get_signed_boundary_distance(p, &valid);
        d_min = d_min.min(d);
    }
    let amplitude = amplitude.min((d_min - 0.05).max(0.0));
    ((start, end), amplitude)
}

/// The area the tool centre may travel: `region` eroded by `tool_radius`
/// minus the islands buffered by `island_clearance`. Without the buffered
/// islands a carrier through the region's shell would machine straight
/// through an island the shell does not carry.
fn tool_centre_region(
    region: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    island_clearance: f64,
) -> Vec<Polygon> {
    use crate::geo::shape::polygon::{
        get_polygons_group_difference, offset_polygon, JoinStyle,
    };

    let eroded = offset_polygon(region, -tool_radius, JoinStyle::Miter);
    let mut valid = if eroded.is_empty() {
        vec![region.clone()]
    } else {
        eroded
    };
    if !islands.is_empty() {
        let buffered: Vec<Polygon> = islands
            .iter()
            .flat_map(|isl| {
                offset_polygon(isl, island_clearance, JoinStyle::Round)
            })
            .filter(|ring| ring.len() >= 3)
            .collect();
        if !buffered.is_empty() {
            valid = get_polygons_group_difference(&valid, &buffered);
        }
    }
    valid
}

/// Pick the longest axis-aligned line segment through `entry_pt` whose
/// tool disc stays entirely inside `region` and clear of the islands.
/// Tries both X and Y sweeps against the **tool-centre region** (region
/// eroded by `tool_radius`, islands buffered by `island_clearance`).
/// Picks the longest inside sub-segment. Falls back to the AABB-spanning
/// line if erosion yields nothing.
fn ramp_segment_in_region(
    entry_pt: crate::geo::types::Point,
    region: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    island_clearance: f64,
) -> (crate::geo::types::Point, crate::geo::types::Point) {
    let valid =
        tool_centre_region(region, islands, tool_radius, island_clearance);

    let bbox = crate::geo::shape::polygon::get_polygon_bounds(region);

    let candidates = [
        (
            crate::geo::types::Point::new(bbox.min.x, entry_pt.y),
            crate::geo::types::Point::new(bbox.max.x, entry_pt.y),
        ),
        (
            crate::geo::types::Point::new(entry_pt.x, bbox.min.y),
            crate::geo::types::Point::new(entry_pt.x, bbox.max.y),
        ),
    ];

    let mut best: Option<(crate::geo::types::Point, crate::geo::types::Point)> =
        None;
    let mut best_len = 0.0_f64;

    for (p1, p2) in &candidates {
        let clipped =
            crate::geo::algo::clipping::clip_line_segment_with_polygons_2d(
                *p1, *p2, &valid,
            );
        for (a, b) in clipped {
            let len = (b.x - a.x).hypot(b.y - a.y);
            if len > best_len {
                best_len = len;
                best = Some((a, b));
            }
        }
    }

    best.unwrap_or_else(|| {
        let bbox = get_polygon_bounds(region);
        longest_line_through_point(entry_pt, bbox)
    })
}
