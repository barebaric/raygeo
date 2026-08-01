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

        steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(HelixSpec {
                center: region.entry_pt,
                start_radius: helix_r,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                pitch: opts.plunge_pitch,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            }),
            region_boundary: None,
        });

        let spiral_max_r = (region.r_max - opts.tool_radius - opts.safe_margin)
            .max(helix_r + 0.01);
        let radial_dist = spiral_max_r - helix_r;
        if radial_dist > 0.0 && opts.step_over > 0.0 {
            let revolutions = radial_dist / opts.step_over;
            steps.push(PlanStep {
                face_id: face_id.to_string(),
                spec: Arc::new(SpiralSpec {
                    center: region.entry_pt,
                    z: opts.target_z,
                    start_radius: helix_r,
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
        45.0,
    ) {
        steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(ToroidalClearSpec {
                carrier: vec![start, end],
                start: Point3D::new(start.x, start.y, opts.safe_z),
                target_z: opts.target_z,
                tool_radius: opts.tool_radius,
                step_over: opts.step_over,
                max_ramp_angle_deg: 45.0,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            }),
            region_boundary: None,
        });
    } else {
        let (start, end) = ramp_segment_in_region(
            region.entry_pt,
            &region.polygon,
            opts.tool_radius,
        );
        steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(RampSpec {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: 45.0,
                style: crate::geo::algo::ramp::RampStyle::ZigZag,
                lateral_amplitude: opts.tool_radius * 0.8,
            }),
            region_boundary: None,
        });
    }

    Ok(steps)
}

/// Pick the longest axis-aligned line segment through `entry_pt` whose
/// tool disc stays entirely inside `region`. Tries both X and Y sweeps
/// against the **eroded** region (region eroded by `tool_radius` so the
/// tool disc centred anywhere on the line fits inside the original
/// region). Picks the longest inside sub-segment. Falls back to the
/// AABB-spanning line if erosion yields nothing.
fn ramp_segment_in_region(
    entry_pt: crate::geo::types::Point,
    region: &Polygon,
    tool_radius: f64,
) -> (crate::geo::types::Point, crate::geo::types::Point) {
    use crate::geo::shape::polygon::{offset_polygon, JoinStyle};

    let eroded = offset_polygon(region, -tool_radius, JoinStyle::Miter);
    let valid: &[Polygon] = if eroded.is_empty() {
        std::slice::from_ref(region)
    } else {
        &eroded
    };

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
                *p1, *p2, valid,
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
