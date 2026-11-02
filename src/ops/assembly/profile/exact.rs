//! Exact boundaries for the profile pass.
//!
//! [`super::engine::run_profile`] walks one boundary representation: a
//! closed loop of line-and-arc pieces. This module supplies the
//! curve-true variant of that boundary — offset by
//! [`crate::geo::algo::offset_exact`] instead of a polygon offsetter —
//! and runs the pass over it, so arcs are emitted as arcs and walls land
//! on the exact offset. [`run_exact_loops`] reports `None` when the exact
//! offsetter produces nothing machinable, and the caller falls back to
//! the polygon path.

use std::f64::consts::FRAC_PI_2;

use crate::error::RaygeoResult;
use crate::geo::algo::analysis::{
    get_closed_spans, get_path_winding_order_from_array,
};
use crate::geo::algo::offset_exact::offset_exact;
use crate::geo::types::{Command, Point, Point3D, WindingOrder};
use crate::ops::assembly::profile::engine::{run_profile, ProfileCommon};
use crate::ops::assembly::profile::trace_helpers as th;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::ToolPose;

/// How close consecutive piece ends must be to count as connected.
const JOINT_TOL: f64 = 1e-6;

/// A closed loop of line-and-arc pieces with cached lengths, addressed by
/// `(piece index, parameter)` pairs.
pub(crate) struct ExactLoop {
    spans: Vec<(Point3D, Command)>,
    lengths: Vec<f64>,
    perimeter: f64,
}

impl ExactLoop {
    pub(crate) fn from_spans(
        spans: Vec<(Point3D, Command)>,
    ) -> Option<ExactLoop> {
        if spans.len() < 2 {
            return None;
        }
        let lengths: Vec<f64> =
            spans.iter().map(|(s, c)| c.length(*s)).collect();
        let perimeter: f64 = lengths.iter().sum();
        if perimeter.is_nan() || perimeter <= 1e-9 {
            return None;
        }
        Some(ExactLoop {
            spans,
            lengths,
            perimeter,
        })
    }

    /// The exact loops of `geometry` (lines and arcs kept, other curves as
    /// line chains within `tolerance`), each with its winding order.
    pub(crate) fn from_geometry(
        geometry: &crate::geo::geometry::Geometry,
        tolerance: f64,
    ) -> Vec<(ExactLoop, WindingOrder)> {
        let mut out = Vec::new();
        for spans in get_closed_spans(geometry, tolerance) {
            // The subpath readers treat the first command as their MOVE.
            let mut data: Vec<Command> = Vec::with_capacity(spans.len() + 1);
            data.push(Command::Move { end: spans[0].0 });
            data.extend(spans.iter().map(|(_, c)| c.clone()));
            let winding = get_path_winding_order_from_array(&data, 0)
                .unwrap_or(WindingOrder::CCW);
            if let Some(lp) = ExactLoop::from_spans(spans) {
                out.push((lp, winding));
            }
        }
        out
    }

    pub(crate) fn perimeter(&self) -> f64 {
        self.perimeter
    }

    pub(crate) fn start(&self) -> Point3D {
        self.spans[0].0
    }

    /// Tracing heading at the loop start: the tangent turned up 90°, like
    /// the polygon engine's outward-normal + 90°.
    pub(crate) fn heading_at_start(&self, forward: bool) -> f64 {
        let tangent = self.spans[0]
            .1
            .tangent_at(self.spans[0].0, 0.0)
            .unwrap_or(Point::new(1.0, 0.0));
        let mut heading = tangent.y.atan2(tangent.x) + FRAC_PI_2;
        if !forward {
            heading += std::f64::consts::PI;
        }
        heading
    }

    pub(crate) fn point_at(&self, idx: usize, t: f64) -> Point3D {
        let (start, command) = &self.spans[idx];
        command.point_at(*start, t).unwrap_or(*start)
    }

    /// `(center, radius, clockwise)` of an arc piece.
    fn arc_parts(&self, idx: usize) -> Option<(Point, f64, bool)> {
        let (start, command) = &self.spans[idx];
        let Command::Arc {
            center_offset,
            normal,
            ..
        } = command
        else {
            return None;
        };
        let c = Point::new(start.x, start.y)
            + Point::new(center_offset.x, center_offset.y);
        Some((
            c,
            Point::new(center_offset.x, center_offset.y).length(),
            normal.z < 0.0,
        ))
    }

    /// Walks `dist` from `(idx, t)` — forward for a positive distance, backward
    /// for a negative one — and returns the landing `(index, parameter, point)`.
    pub(crate) fn advance(
        &self,
        idx: usize,
        t: f64,
        dist: f64,
    ) -> (usize, f64, Point3D) {
        let n = self.spans.len();
        let mut remaining = dist;
        let mut idx = idx;
        let mut t = t;
        // Walk within pieces until the distance is spent; guard against a
        // full perimeter of zero-length pieces.
        for _ in 0..n + 1 {
            let len = self.lengths[idx];
            if len <= 1e-12 {
                idx = if remaining >= 0.0 {
                    (idx + 1) % n
                } else {
                    (idx + n - 1) % n
                };
                t = if remaining >= 0.0 { 0.0 } else { 1.0 };
                continue;
            }
            let dt = remaining / len;
            let new_t = t + dt;
            if (0.0..=1.0).contains(&new_t) {
                return (idx, new_t, self.point_at(idx, new_t));
            }
            // Move to the neighbouring piece and continue with the rest.
            if remaining >= 0.0 {
                remaining -= (1.0 - t) * len;
                idx = (idx + 1) % n;
                t = 0.0;
            } else {
                // Walking backward: the piece consumes t * len of the walk,
                // so the remainder moves toward zero.
                remaining += t * len;
                idx = (idx + n - 1) % n;
                t = 1.0;
            }
        }
        (idx, t, self.point_at(idx, t))
    }

    /// Distance from `p` to the nearest point of the loop.
    pub(crate) fn wall_distance(&self, p: Point) -> f64 {
        self.spans
            .iter()
            .map(|(start, command)| {
                command
                    .closest_point_to(*start, p.x, p.y)
                    .map_or(f64::INFINITY, |(_, _, d)| d.sqrt())
            })
            .fold(f64::INFINITY, f64::min)
    }

    /// Emits the cut moves from `(from_idx, from_t)` to `(to_idx, to_t)`,
    /// walking forward or backward. Arc pieces come out as arcs; piece
    /// boundaries come out as corner moves where the position actually
    /// jumps.
    pub(crate) fn emit_between(
        &self,
        trace: &mut Tracelet,
        from: (usize, f64),
        to: (usize, f64),
        forward: bool,
        z: f64,
    ) {
        let n = self.spans.len();
        let (mut si, mut st) = from;
        let (ti, tt) = to;
        // Same piece, plain partial run.
        if si == ti && ((forward && tt >= st) || (!forward && tt <= st)) {
            self.emit_partial(trace, si, st, tt, forward, z);
            return;
        }
        // First partial run, up to the piece boundary we leave through.
        self.emit_partial(
            trace,
            si,
            st,
            if forward { 1.0 } else { 0.0 },
            forward,
            z,
        );
        for _ in 0..n {
            si = if forward {
                (si + 1) % n
            } else {
                (si + n - 1) % n
            };
            st = if forward { 0.0 } else { 1.0 };
            if si == ti {
                self.emit_partial(trace, si, st, tt, forward, z);
                return;
            }
            self.emit_full(trace, si, forward, z);
        }
    }

    /// Emits the part of piece `idx` between parameters `t0` and `t1`.
    /// Emits the part of piece `idx` between parameters `t0` and `t1`, with
    /// every move lifted to the pass height `z`.
    fn emit_partial(
        &self,
        trace: &mut Tracelet,
        idx: usize,
        t0: f64,
        t1: f64,
        forward: bool,
        z: f64,
    ) {
        let (_, command) = &self.spans[idx];
        let (from, to) = (self.point_at(idx, t0), self.point_at(idx, t1));
        if (Point::new(from.x, from.y)).distance(Point::new(to.x, to.y))
            <= JOINT_TOL
        {
            return;
        }
        match command {
            Command::Line { .. } => trace.line_to(to.x, to.y, z, None),
            Command::Arc { .. } => {
                let Some((c, _, cw)) = self.arc_parts(idx) else {
                    return;
                };
                // Walking a piece backwards flips its rotation.
                let clockwise = if forward { cw } else { !cw };
                trace.arc_to(
                    to.x,
                    to.y,
                    c.x - from.x,
                    c.y - from.y,
                    clockwise,
                    z,
                    None,
                );
            }
            _ => {}
        }
    }

    /// Emits a whole piece in the walking direction.
    fn emit_full(
        &self,
        trace: &mut Tracelet,
        idx: usize,
        forward: bool,
        z: f64,
    ) {
        if forward {
            self.emit_partial(trace, idx, 0.0, 1.0, true, z);
        } else {
            self.emit_partial(trace, idx, 1.0, 0.0, false, z);
        }
    }

    /// The number of pieces in the loop.
    pub(crate) fn piece_count(&self) -> usize {
        self.spans.len()
    }

    /// The loop's pieces with their start points, in walk order.
    pub(crate) fn spans(&self) -> &[(Point3D, Command)] {
        &self.spans
    }

    /// The exact length of piece `idx`.
    pub(crate) fn piece_length(&self, idx: usize) -> f64 {
        self.lengths[idx]
    }

    /// Emits a whole piece in the walking direction, ending at height
    /// `z_end` — an arc piece becomes a helical arc move. The start
    /// height is wherever the previous move left the tool.
    pub(crate) fn emit_piece_to_z(
        &self,
        trace: &mut Tracelet,
        idx: usize,
        forward: bool,
        z_end: f64,
    ) {
        let (_, command) = &self.spans[idx];
        let (from, to) = if forward {
            (self.point_at(idx, 0.0), self.point_at(idx, 1.0))
        } else {
            (self.point_at(idx, 1.0), self.point_at(idx, 0.0))
        };
        if (Point::new(from.x, from.y)).distance(Point::new(to.x, to.y))
            <= JOINT_TOL
        {
            return;
        }
        match command {
            Command::Line { .. } => trace.line_to(to.x, to.y, z_end, None),
            Command::Arc { .. } => {
                let Some((c, _, cw)) = self.arc_parts(idx) else {
                    return;
                };
                // Walking a piece backwards flips its rotation.
                let clockwise = if forward { cw } else { !cw };
                trace.arc_to(
                    to.x,
                    to.y,
                    c.x - from.x,
                    c.y - from.y,
                    clockwise,
                    z_end,
                    None,
                );
            }
            _ => {}
        }
    }

    /// Emits `laps` full rounds of the loop while the height sinks from
    /// `z_from` to `z_to`, ending back at the loop start at `z_to`. The
    /// walk direction matches the caller's, so the ramp blends into the
    /// pass that follows.
    pub(crate) fn emit_ramp(
        &self,
        trace: &mut Tracelet,
        laps: usize,
        z_from: f64,
        z_to: f64,
        forward: bool,
    ) {
        let n = self.piece_count();
        let total = laps as f64 * self.perimeter;
        if total.is_nan() || total <= 0.0 {
            return;
        }
        let order: Vec<usize> = if forward {
            (0..n).collect()
        } else {
            (0..n).rev().collect()
        };
        let drop = z_from - z_to;
        let mut done = 0.0f64;
        for _ in 0..laps {
            for &idx in &order {
                let len = self.piece_length(idx);
                done += len;
                let z_end = if done >= total - 1e-9 {
                    z_to
                } else {
                    z_from - drop * (done / total)
                };
                self.emit_piece_to_z(trace, idx, forward, z_end);
            }
        }
    }
}

/// The classified exact offset loops: counter-clockwise material loops
/// (largest first) and clockwise island loops.
pub(crate) type LoopClassification =
    (Vec<(ExactLoop, f64)>, Vec<(ExactLoop, f64)>);

/// Classifies the exact offset loops of `geometry`: counter-clockwise
/// material loops (largest first) and, unless `keep_outer_only`, clockwise
/// island loops. `Ok(None)` means the exact offsetter produced nothing
/// machinable and the caller should fall back to the polygon path.
pub(crate) fn classify_loops(
    geometry: &crate::geo::geometry::Geometry,
    offset: f64,
    tolerance: f64,
    keep_outer_only: bool,
) -> Option<LoopClassification> {
    let loops_geo = offset_exact(geometry, offset, tolerance);
    if loops_geo.is_empty() {
        return None;
    }
    let tolerance = tolerance.max(1e-6);
    let mut loops: Vec<(ExactLoop, f64)> = Vec::new();
    let mut islands: Vec<(ExactLoop, f64)> = Vec::new();
    for geo in &loops_geo {
        for (lp, winding) in ExactLoop::from_geometry(geo, tolerance) {
            let area = lp.perimeter();
            if winding == WindingOrder::CCW {
                loops.push((lp, area));
            } else if !keep_outer_only {
                islands.push((lp, area));
            }
        }
    }
    if loops.is_empty() && islands.is_empty() {
        return None;
    }
    // Outer pieces largest first; islands follow nearest to where the pass
    // before them ended, like the polygon engine walks its grown islands.
    loops.sort_by(|a, b| b.1.total_cmp(&a.1));
    Some((loops, islands))
}

/// Builds the exact tool-center loops for one side and runs them, outer
/// pieces first. `offset` is the region grow for the outer profile and the
/// shrink for the inner profile. `Ok(None)` means the exact offsetter
/// produced nothing machinable and the caller should fall back to the
/// polygon path.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_exact_loops(
    face: &mut FaceState,
    trace: &mut Tracelet,
    geometry: &crate::geo::geometry::Geometry,
    offset: f64,
    opts: &super::ProfileSpec,
    cut_state: &State,
    keep_outer_only: bool,
) -> RaygeoResult<Option<AssemblyMeta>> {
    // Like the polygon path, the pass covers one region — the largest.
    // Smaller counter-clockwise loops are separate material regions the
    // pass does not cover (declared islands that fall outside the pocket
    // normalize to this shape), and are dropped.
    let Some((mut loops, islands)) =
        classify_loops(geometry, offset, opts.tolerance, keep_outer_only)
    else {
        return Ok(None);
    };
    loops.truncate(1);
    let tolerance = opts.tolerance.max(1e-6);

    let common = ProfileCommon {
        step_length: opts.step_length,
        target_z: opts.target_z,
        safe_z: opts.safe_z,
        tolerance: opts.tolerance,
        tool_radius: opts.tool_radius,
        cut_direction: opts.cut_direction,
        expansion_batch_size: opts.expansion_batch_size,
        cancel_check: opts.cancel_check,
        engagement_area_threshold: opts.engagement_area_threshold,
        engagement_angle_threshold: opts.engagement_angle_threshold,
        feed_reduction_factor: opts.feed_reduction_factor,
        stock_to_leave: opts.stock_to_leave,
    };

    // Tracing attributes, as flattened polygons like the polygon engine's.
    let polys: Vec<crate::geo::types::Polygon> = loops
        .iter()
        .chain(islands.iter())
        .flat_map(|(lp, _)| {
            let mut g = crate::geo::geometry::Geometry::new();
            let mut first = true;
            for (start, command) in &lp.spans {
                if first {
                    g.move_to(start.x, start.y, start.z);
                    first = false;
                }
                match command {
                    Command::Move { end } => {
                        g.move_to(end.x, end.y, end.z);
                    }
                    Command::Line { end } => {
                        g.line_to(end.x, end.y, end.z);
                    }
                    Command::Arc {
                        end,
                        center_offset,
                        normal,
                    } => {
                        g.arc_to(
                            end.x,
                            end.y,
                            center_offset.x,
                            center_offset.y,
                            normal.z < 0.0,
                            end.z,
                        );
                    }
                    Command::Bezier {
                        end,
                        control1,
                        control2,
                    } => {
                        g.bezier_to(*control1, *control2, *end);
                    }
                }
            }
            g.to_polygons(tolerance)
        })
        .collect();
    let walk_order: Vec<u32> = (0..polys.len() as u32).collect();
    trace.set_attrs(th::build_attrs(&polys, &walk_order));

    let mut start_pose: Option<ToolPose> = None;
    let mut end_pose = ToolPose {
        pos: Point3D::ZERO,
        heading: 0.0,
    };
    let mut idx = 0u32;
    for (lp, _) in &loops {
        let meta = run_profile(face, trace, lp, &common, cut_state, idx)?;
        if start_pose.is_none() {
            start_pose = Some(meta.start);
        }
        end_pose = meta.end;
        idx += 1;
    }
    if !islands.is_empty() {
        let mut remaining = islands;
        while !remaining.is_empty() {
            let last = end_pose.pos;
            let nearest = remaining
                .iter()
                .enumerate()
                .min_by(|(_, (a, _)), (_, (b, _))| {
                    let da = (a.start() - last).length_squared();
                    let db = (b.start() - last).length_squared();
                    crate::utils::sort_f64(da, db)
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            let (lp, _) = remaining.remove(nearest);
            let meta = run_profile(face, trace, &lp, &common, cut_state, idx)?;
            if start_pose.is_none() {
                start_pose = Some(meta.start);
            }
            end_pose = meta.end;
            idx += 1;
        }
    }
    match start_pose {
        Some(start) => Ok(Some(AssemblyMeta {
            start,
            end: end_pose,
        })),
        None => Ok(None),
    }
}
