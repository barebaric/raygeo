use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::algo::engagement::Engagement;
use crate::geo::shape::polygon::{
    get_polygon_heading_at, get_polygons_closest_point,
};
use crate::geo::shape::polygon3d::walk_along_polygon_3d;
use crate::ops::assembly::result::AssemblyResult;
use crate::ops::container::Ops;
use crate::ops::cut::CutDirection;
use crate::ops::cut::ToolPose;
use crate::ops::state::State;
use crate::types::{Point, Point3D, Polygon};

/// Find polygon vertices that lie strictly between `from` and `to` along
/// the forward (CCW) direction of `poly3d`.  Returns them in walk order.
/// This ensures the engine emits explicit LINE_TO commands at corners so
/// the toolpath has sharp miter joins instead of cutting across.
fn intermediate_vertices(
    poly3d: &[Point3D],
    from: &Point3D,
    to: &Point3D,
) -> Vec<Point3D> {
    let n = poly3d.len();
    if n < 3 {
        return Vec::new();
    }
    let find_seg = |p: &Point3D| -> Option<(usize, f64)> {
        for i in 0..n {
            let a = poly3d[i];
            let b = poly3d[(i + 1) % n];
            let ab = b - a;
            let len_sq = ab.length_squared();
            if len_sq < 1e-12 {
                continue;
            }
            let ap = *p - a;
            let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
            let closest = a + ab * t;
            if closest.distance_squared(*p) < 1e-10 {
                return Some((i, t));
            }
        }
        None
    };
    let (mut from_seg, mut from_t) = match find_seg(from) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let (to_seg, to_t) = match find_seg(to) {
        Some(v) => v,
        None => return Vec::new(),
    };

    // If `from` is at the far end of its segment (t ≈ 1), it's exactly at
    // vertex (from_seg+1) — advance to the next segment with t=0 to avoid
    // duplicating that vertex as an intermediate.
    if from_t > 1.0 - 1e-9 {
        from_seg = (from_seg + 1) % n;
        from_t = 0.0;
    }

    let mut verts = Vec::new();

    if from_seg == to_seg {
        if to_t > from_t + 1e-9 {
            // Same segment, forward — no vertices crossed.
            return verts;
        }
        // Wrapped around: cross ALL vertices from (from_seg+1)%n back to from_seg.
        let mut idx = (from_seg + 1) % n;
        for _ in 0..n {
            verts.push(poly3d[idx]);
            if idx == from_seg {
                break;
            }
            idx = (idx + 1) % n;
        }
        return verts;
    }

    // Different segments: walk forward from (from_seg+1)%n to to_seg.
    let mut idx = (from_seg + 1) % n;
    for _ in 0..n {
        if idx == to_seg {
            break;
        }
        verts.push(poly3d[idx]);
        idx = (idx + 1) % n;
    }
    // The vertex at to_seg is the start of the segment containing `to`.
    // It's only worth pushing if `to` is past it (to_t > 0).  When the loop
    // breaks immediately, from_seg+1 == to_seg, and poly3d[to_seg] would
    // duplicate `from`; the t advancement above rules that out.
    if to_t > 1e-9 {
        verts.push(poly3d[to_seg]);
    }
    verts
}

#[allow(dead_code)]
pub(crate) struct ProfileCommon {
    pub step_length: f64,
    pub target_z: f64,
    pub safe_z: f64,
    pub tolerance: f64,
    pub tool_radius: f64,
    pub cut_direction: CutDirection,
    pub expansion_batch_size: usize,
    pub cancel_check: Option<fn() -> bool>,
    pub engagement_area_threshold: f64,
    pub engagement_angle_threshold: f64,
    pub stock_to_leave: f64,
}

pub(crate) fn run_profile(
    cleared: &mut crate::ops::cut::ClearedArea,
    offset_poly: &Polygon,
    common: &ProfileCommon,
    cut_state: &State,
    target_polygon_idx: u32,
    recorder: &mut super::trace::TraceRecorder,
) -> RaygeoResult<AssemblyResult> {
    if offset_poly.len() < 3 {
        return Err(RaygeoError::DegenerateGeometry(
            "offset polygon too small".into(),
        ));
    }

    let poly3d: Vec<Point3D> = offset_poly
        .iter()
        .map(|p| Point3D::new(p.x, p.y, common.target_z))
        .collect();

    let n = poly3d.len();
    let mut perimeter = 0.0;
    for i in 0..n {
        let next = (i + 1) % n;
        perimeter += (poly3d[next] - poly3d[i]).length();
    }

    // Compute heading from polygon tangent at start vertex
    let heading = {
        let normal_angle = get_polygon_heading_at(offset_poly, offset_poly[0]);
        // Tangent direction = outward normal + 90°
        normal_angle + std::f64::consts::FRAC_PI_2
    };

    let mut ops = Ops::new();
    let start_pos = poly3d[0];

    ops.move_to(start_pos.x, start_pos.y, common.safe_z, None);
    ops.move_to(start_pos.x, start_pos.y, common.target_z, None);
    ops.apply_state(cut_state);
    recorder.record_polygon_start(
        start_pos,
        heading,
        ops.len() as u32,
        target_polygon_idx,
        perimeter,
    );

    // Effective area threshold
    let full_crescent =
        std::f64::consts::PI * common.tool_radius * common.tool_radius
            - 2.0
                * common.tool_radius
                * common.tool_radius
                * ((common.step_length / (2.0 * common.tool_radius))
                    .clamp(-1.0, 1.0)
                    .acos())
            + (common.step_length * 0.5)
                * (4.0 * common.tool_radius * common.tool_radius
                    - common.step_length * common.step_length)
                    .max(0.0)
                    .sqrt();
    let area_threshold = if common.engagement_area_threshold > 0.0 {
        common.engagement_area_threshold
    } else {
        full_crescent * 0.85
    };
    let angle_threshold = common.engagement_angle_threshold;

    let original_feed_rate = cut_state.feed_rate;
    let mut current_feed_rate = cut_state.feed_rate;
    let mut feed_reduced = false;

    let mut current_3d = start_pos;
    let mut dist_traveled = 0.0;
    let max_steps = (perimeter / common.step_length + 10.0) as usize;
    #[allow(unused_assignments)]
    let mut last_eng: Engagement = unsafe { std::mem::zeroed() };

    for _ in 0..max_steps {
        // Cancellation check
        if let Some(check) = common.cancel_check {
            if check() {
                ops.move_to(current_3d.x, current_3d.y, common.safe_z, None);
                let end_pose = ToolPose {
                    pos: current_3d,
                    heading,
                };
                return Ok(AssemblyResult {
                    ops,
                    cleared_polygons: vec![],
                    start: ToolPose {
                        pos: start_pos,
                        heading,
                    },
                    end: end_pose,
                });
            }
        }

        // Restore feed after reduction
        if feed_reduced {
            if let Some(fr) = original_feed_rate {
                ops.set_feed_rate(fr);
                recorder.record_feed_change(
                    current_3d,
                    heading,
                    ops.len() as u32,
                    current_feed_rate.unwrap_or(0),
                    fr,
                );
                current_feed_rate = original_feed_rate;
            }
            feed_reduced = false;
        }

        let mut next = walk_along_polygon_3d(
            &poly3d,
            &current_3d,
            true,
            common.step_length,
        );

        // ── Engagement check + adaptive step/feed ──
        let mut effective_step = common.step_length;
        let mut reductions = 0u32;
        let mut travel_skipped = false;
        // The engagement check is only meaningful for multi-pass roughing
        // where earlier passes leave material that the check can measure
        // against. On a first pass (or finish pass), the tool is fully
        // buried by design and the check would spuriously fire on every
        // step. Gate it on stock_to_leave > 0 (rough pass) AND the cleared
        // area already containing material from a prior pass.
        let check_engagement =
            common.stock_to_leave > 0.0 && !cleared.is_empty();
        if check_engagement {
            loop {
                let eng = cleared.get_point_engagement(
                    Point::new(next.x, next.y),
                    common.tool_radius,
                );
                if eng.area > area_threshold || eng.angle > angle_threshold {
                    reductions += 1;
                    if reductions >= 3 {
                        // Travel-skip: lift, move, re-plunge, restore feed.
                        last_eng = eng;
                        ops.move_to(next.x, next.y, common.safe_z, None);
                        ops.move_to(next.x, next.y, common.target_z, None);
                        if let Some(fr) = original_feed_rate {
                            ops.set_feed_rate(fr);
                            current_feed_rate = original_feed_rate;
                        }
                        effective_step = common.step_length;
                        travel_skipped = true;
                        break;
                    }
                    effective_step *= 0.5;
                    next = walk_along_polygon_3d(
                        &poly3d,
                        &current_3d,
                        true,
                        effective_step,
                    );
                    if let Some(fr) = current_feed_rate {
                        let reduced = (fr as f64 * 0.5).max(1.0) as i32;
                        ops.set_feed_rate(reduced);
                        recorder.record_feed_change(
                            next,
                            heading,
                            ops.len() as u32,
                            fr,
                            reduced,
                        );
                        current_feed_rate = Some(reduced);
                        feed_reduced = true;
                    }
                    continue;
                }
                last_eng = eng;
                break;
            }
        }

        // ── Drift correction ──
        let mut wall_distance = 0.0;
        if let Some((_idx, _t, closest, dist_sq)) = get_polygons_closest_point(
            std::slice::from_ref(offset_poly),
            Point::new(next.x, next.y),
        ) {
            wall_distance = dist_sq.sqrt();
            let dx = next.x - closest.x;
            let dy = next.y - closest.y;
            let dist = wall_distance;
            if dist > common.tolerance {
                let nudge = 0.5;
                let corrected = Point::new(
                    closest.x + dx * (1.0 - nudge),
                    closest.y + dy * (1.0 - nudge),
                );
                next = Point3D::new(corrected.x, corrected.y, common.target_z);
            }
        }
        dist_traveled += effective_step;

        // Expand the cleared area for this step BEFORE the next engagement
        // check so the swept path is reflected in the cleared fragments.
        cleared.begin_batch();
        cleared.expand_batched(
            crate::types::Point::new(current_3d.x, current_3d.y),
            crate::types::Point::new(next.x, next.y),
            common.tool_radius,
        );
        cleared.commit_batch_local();
        cleared.compact_if_needed(common.tolerance);

        if dist_traveled >= perimeter - common.step_length * 0.5 {
            recorder.record_cut(
                start_pos,
                heading,
                current_3d,
                ops.len() as u32,
                target_polygon_idx,
                perimeter,
                dist_traveled,
                current_feed_rate.unwrap_or(0),
                effective_step,
                reductions,
                last_eng.angle,
                last_eng.area,
                last_eng.chord_depth,
                wall_distance,
            );
            if !travel_skipped {
                let verts =
                    intermediate_vertices(&poly3d, &current_3d, &start_pos);
                for v in &verts {
                    ops.line_to(v.x, v.y, common.target_z, None);
                }
                ops.line_to(start_pos.x, start_pos.y, common.target_z, None);
            }
            break;
        }
        recorder.record_cut(
            next,
            heading,
            current_3d,
            ops.len() as u32,
            target_polygon_idx,
            perimeter,
            dist_traveled,
            current_feed_rate.unwrap_or(0),
            effective_step,
            reductions,
            last_eng.angle,
            last_eng.area,
            last_eng.chord_depth,
            wall_distance,
        );
        if !travel_skipped {
            let verts = intermediate_vertices(&poly3d, &current_3d, &next);
            for v in &verts {
                ops.line_to(v.x, v.y, common.target_z, None);
            }
            ops.line_to(next.x, next.y, common.target_z, None);
        }
        current_3d = next;
    }

    recorder.record_polygon_end(
        start_pos,
        heading,
        ops.len() as u32,
        target_polygon_idx,
    );

    ops.move_to(start_pos.x, start_pos.y, common.safe_z, None);

    let end_pose = ToolPose {
        pos: start_pos,
        heading,
    };
    Ok(AssemblyResult {
        ops,
        cleared_polygons: vec![],
        start: end_pose,
        end: end_pose,
    })
}
