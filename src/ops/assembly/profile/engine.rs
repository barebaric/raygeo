use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::algo::engagement::Engagement;
use crate::geo::types::{Command, Point, Point3D, Polygon};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::CutDirection;
use crate::ops::types::ToolPose;
use crate::trace_types::MoveKind;

use super::exact::ExactLoop;
use super::trace_helpers as th;

/// A polygon as a closed loop of exact line pieces (lifted to `z`), so
/// polygon offsets walk through the same engine as curve-true boundaries.
pub(crate) fn loop_from_polygon(
    offset_poly: &Polygon,
    z: f64,
) -> Option<ExactLoop> {
    if offset_poly.len() < 3 {
        return None;
    }
    let mut spans: Vec<(Point3D, Command)> =
        Vec::with_capacity(offset_poly.len());
    let mut prev = Point3D::new(offset_poly[0].x, offset_poly[0].y, z);
    for point in offset_poly.iter().skip(1).chain(offset_poly.iter().take(1)) {
        let end = Point3D::new(point.x, point.y, z);
        if prev.distance(end) > 1e-9 {
            spans.push((prev, Command::Line { end }));
        }
        prev = end;
    }
    ExactLoop::from_spans(spans)
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
    pub feed_reduction_factor: f64,
    pub stock_to_leave: f64,
}

/// Walks the offset boundary once at the target depth.
///
/// The boundary is an [`ExactLoop`] — a closed loop of line-and-arc pieces.
/// Polygon offsets enter as all-line loops, curve-true offsets carry arcs,
/// and the walk emits what the pieces are: chords along line pieces, arc
/// moves along arc pieces, with explicit corner moves where pieces meet at
/// an angle. CCW (`common.cut_direction`) walks the loop as stored —
/// interior on the left, the climb side for a clockwise spindle — and CW
/// walks it backwards.
///
/// The walk is parameterized on the boundary itself, so positions lie on it
/// by construction and no drift correction is needed; the wall distance is
/// measured only for the tracing meta.
pub(crate) fn run_profile(
    face: &mut FaceState,
    trace: &mut Tracelet,
    loop_: &ExactLoop,
    common: &ProfileCommon,
    cut_state: &State,
    target_polygon_idx: u32,
) -> RaygeoResult<AssemblyMeta> {
    let forward = common.cut_direction == CutDirection::Ccw;
    let perimeter = loop_.perimeter();
    if perimeter.is_nan() || perimeter <= 1e-9 {
        return Err(RaygeoError::DegenerateGeometry(
            "offset boundary too small".into(),
        ));
    }
    let start_pos = loop_.start();
    let heading = loop_.heading_at_start(forward);

    trace.move_to(start_pos.x, start_pos.y, common.safe_z, None);
    // Descend at the plunge feed (falling back to the cut feed) instead of
    // rapiding into full depth.
    trace.apply_state(cut_state);
    let plunge_feed = cut_state.plunge_feed.or(cut_state.feed_rate);
    if plunge_feed != cut_state.feed_rate {
        trace.set_feed_rate(plunge_feed.unwrap_or(0));
    }
    trace.line_to(start_pos.x, start_pos.y, common.target_z, None);
    if plunge_feed != cut_state.feed_rate {
        if let Some(fr) = cut_state.feed_rate {
            trace.set_feed_rate(fr);
        }
    }

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

    let mut current = start_pos;
    let (mut piece_idx, mut piece_t) = (0usize, 0.0f64);
    let mut dist_traveled = 0.0;
    let max_steps = (perimeter / common.step_length + 10.0) as usize;
    #[allow(unused_assignments)]
    #[allow(unused_assignments)]
    let mut _last_eng: Engagement = unsafe { std::mem::zeroed() };
    let mut first_step = true;
    // Travel-skips in a row; a long run means the skip heuristic has
    // deadlocked (the loop starts in open material), so it is switched
    // off for the rest of the loop.
    let mut consecutive_skips = 0u32;
    let mut engagement_disabled = false;

    trace.init(
        th::tool_snapshot(start_pos, heading, start_pos),
        Some(th::init_meta(target_polygon_idx)),
    );

    for _ in 0..max_steps {
        // Cancellation check
        if let Some(check) = common.cancel_check {
            if check() {
                trace.move_to(current.x, current.y, common.safe_z, None);
                trace.exit(th::tool_snapshot(current, heading, current), None);
                let end_pose = ToolPose {
                    pos: current,
                    heading,
                };
                return Ok(AssemblyMeta {
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
                trace.set_feed_rate(fr);
                trace.move_event(
                    MoveKind::Travel,
                    th::tool_snapshot(current, heading, current),
                    Some(th::feed_change_meta(
                        current_feed_rate.unwrap_or(0),
                        fr,
                    )),
                );
                current_feed_rate = original_feed_rate;
            }
            feed_reduced = false;
        }

        let step = if forward {
            common.step_length
        } else {
            -common.step_length
        };
        let (mut next_idx, mut next_t, _) =
            loop_.advance(piece_idx, piece_t, step);
        let mut next = loop_.point_at(next_idx, next_t);

        // ── Engagement check + adaptive step/feed ──
        let mut effective_step = common.step_length;
        let mut reductions = 0u32;
        let mut travel_skipped = false;
        // The engagement check is only meaningful for multi-pass roughing
        // where earlier passes leave material that the check can measure
        // against. On a first pass (or finish pass), the tool is fully
        // buried by design and the check would spuriously fire on every
        // step. Gate it on stock_to_leave > 0 (rough pass) AND the face.cleared
        // area already containing material from a prior pass.
        let check_engagement = !engagement_disabled
            && common.stock_to_leave > 0.0
            && !face.cleared.is_empty();
        if check_engagement {
            loop {
                let eng = face.cleared.get_point_engagement(
                    Point::new(next.x, next.y),
                    common.tool_radius,
                );
                if eng.area > area_threshold || eng.angle > angle_threshold {
                    reductions += 1;
                    if reductions >= 3 && consecutive_skips < 3 {
                        // Travel-skip: lift, move, re-plunge, restore feed.
                        _last_eng = eng;
                        trace.move_to(next.x, next.y, common.safe_z, None);
                        trace.move_to(next.x, next.y, common.target_z, None);
                        if let Some(fr) = original_feed_rate {
                            trace.set_feed_rate(fr);
                            current_feed_rate = original_feed_rate;
                        }
                        effective_step = common.step_length;
                        travel_skipped = true;
                        consecutive_skips += 1;
                        break;
                    }
                    if reductions >= 3 {
                        // Skipping deadlocks here — every step reads as a
                        // fresh full bite (the loop starts in open
                        // material), and the skipped stretches anchor the
                        // frontier so it stays that way. Cut the rest of
                        // the loop as planned, at the full feed.
                        engagement_disabled = true;
                        if let Some(fr) = original_feed_rate {
                            trace.set_feed_rate(fr);
                            current_feed_rate = original_feed_rate;
                        }
                        feed_reduced = false;
                        effective_step = common.step_length;
                        let step = if forward {
                            effective_step
                        } else {
                            -effective_step
                        };
                        (next_idx, next_t, _) =
                            loop_.advance(piece_idx, piece_t, step);
                        next = loop_.point_at(next_idx, next_t);
                        _last_eng = eng;
                        break;
                    }
                    effective_step *= 0.5;
                    let step = if forward {
                        effective_step
                    } else {
                        -effective_step
                    };
                    (next_idx, next_t, _) =
                        loop_.advance(piece_idx, piece_t, step);
                    next = loop_.point_at(next_idx, next_t);
                    if let Some(fr) = current_feed_rate {
                        let reduced = (fr as f64 * common.feed_reduction_factor)
                            .max(1.0)
                            as i32;
                        trace.set_feed_rate(reduced);
                        trace.move_event(
                            MoveKind::Travel,
                            th::tool_snapshot(next, heading, next),
                            Some(th::feed_change_meta(fr, reduced)),
                        );
                        current_feed_rate = Some(reduced);
                        feed_reduced = true;
                    }
                    continue;
                }
                _last_eng = eng;
                break;
            }
        }

        let wall_distance = loop_.wall_distance(Point::new(next.x, next.y));
        dist_traveled += effective_step;

        // Expand the face.cleared area for this step BEFORE the next engagement
        // check so the swept path is reflected in the face.cleared fragments.
        face.cleared.begin_batch();
        face.cleared.expand_batched(
            Point::new(current.x, current.y),
            Point::new(next.x, next.y),
            common.tool_radius,
        );
        face.cleared.commit_batch_local();
        face.cleared
            .compact_if_needed(&face.stock_region, common.tolerance);

        if dist_traveled >= perimeter - common.step_length * 0.5 {
            if !travel_skipped {
                // Close the pass exactly back at the loop start.
                loop_.emit_between(
                    trace,
                    (piece_idx, piece_t),
                    (0, 0.0),
                    forward,
                    common.target_z,
                );
                trace.cut(
                    th::tool_snapshot(start_pos, heading, current),
                    Some(th::cut_meta(
                        target_polygon_idx,
                        dist_traveled,
                        perimeter,
                        wall_distance,
                        current_feed_rate.unwrap_or(0),
                        effective_step,
                        reductions,
                        first_step,
                    )),
                );
            }
            break;
        }
        if !travel_skipped {
            loop_.emit_between(
                trace,
                (piece_idx, piece_t),
                (next_idx, next_t),
                forward,
                common.target_z,
            );
            trace.cut(
                th::tool_snapshot(next, heading, current),
                Some(th::cut_meta(
                    target_polygon_idx,
                    dist_traveled,
                    perimeter,
                    wall_distance,
                    current_feed_rate.unwrap_or(0),
                    effective_step,
                    reductions,
                    first_step,
                )),
            );
            first_step = false;
            consecutive_skips = 0;
        }
        current = next;
        piece_idx = next_idx;
        piece_t = next_t;
    }

    trace.move_to(start_pos.x, start_pos.y, common.safe_z, None);
    trace.exit(th::tool_snapshot(start_pos, heading, start_pos), None);

    let end_pose = ToolPose {
        pos: start_pos,
        heading,
    };
    Ok(AssemblyMeta {
        start: end_pose,
        end: end_pose,
    })
}
