//! Optimize: Travel distance optimization for Ops sequences.
//!
//! Implements nearest-neighbor reordering (KDTree) and 2-opt refinement
//! for both workpiece-level and segment-level path optimization.

use std::collections::HashSet;

use rstar::{PointDistance, RTree, RTreeObject, AABB};

use super::link::{find_pass_entry, find_pass_exit};
use crate::geo::types::Point3D;
use crate::ops::callbacks::Callbacks;
use crate::ops::container::Ops;
use crate::ops::enums::{CommandCategory, CommandType};
use crate::ops::state::State;
use crate::ops::transform::{Phase, TransformCtx, Transformer};
use crate::ops::types::{MoveCmd, OpCategory};

const TWO_OPT_SEGMENT_THRESHOLD: usize = 1000;
const TWO_OPT_COMMAND_LIMIT: usize = 10000;
const TWO_OPT_MAX_ITER: usize = 10;

/// Minimum direction change (degrees) for a vertex to count as a
/// corner for the cut planner's "prefer corners" mode. A square's
/// corners turn by 90°, a polyline circle approximation turns by far
/// less per vertex, so this separates real corners from smooth curves.
const CORNER_ANGLE_DEG: f64 = 25.0;

/// Tolerance (mm) used to recognise a closed subpath: its last moving
/// command ends where its MoveTo started.
const CLOSED_PATH_EPS: f64 = 1e-6;

/// Parameters for the [`optimize_travel`] transformer.
#[derive(Clone, Debug, PartialEq)]
pub struct OptimizeSpec {
    /// Whether flipping subpaths is allowed.
    pub allow_flip: bool,
    /// Keep the first workpiece in place.
    pub preserve_first: bool,
    /// Workpiece UIDs whose order to preserve.
    pub preserve_order: Vec<String>,
    /// When set, acceleration-aware scanline merging runs as the
    /// first step of optimization: parallel cut lines on the same
    /// scan row — including rows spanning multiple workpieces — are
    /// bridged at zero power when the machine's motion profile makes
    /// the merged sweep faster. `None` disables merging.
    pub merge_scanlines: Option<super::merge_scanlines::MergeScanlinesSpec>,
    /// When set, a closed path picked as the next segment may be
    /// entered at any of its vertices: the segment is rotated so its
    /// start (and end) sits at the vertex nearest the current head
    /// position. Open paths keep their current behaviour and closed
    /// paths with lead-ins, overcut or tabs (no longer closed) are
    /// never rotated.
    pub best_start_point: bool,
    /// Only meaningful together with `best_start_point`: restrict the
    /// candidate start vertices of a closed path to its corners
    /// (vertices where the path direction changes by more than
    /// [`CORNER_ANGLE_DEG`]). Paths without corners, like circles,
    /// fall back to any vertex. Defaults to false.
    pub prefer_corners: bool,
}

impl Transformer for OptimizeSpec {
    fn phase(&self) -> Phase {
        Phase::GeometryRefinement
    }

    fn apply(&self, ctx: &mut TransformCtx<'_>) {
        if let Some(spec) = &self.merge_scanlines {
            super::merge_scanlines::merge_scanlines(
                ctx.ops,
                spec,
                ctx.callbacks,
            );
        }
        optimize_travel(
            ctx.ops,
            self.allow_flip,
            self.preserve_first,
            self.preserve_order.clone(),
            ctx.callbacks,
            self.best_start_point,
            self.prefer_corners,
        );
    }

    fn name(&self) -> &str {
        "optimize"
    }

    fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name().hash(&mut h);
        self.allow_flip.hash(&mut h);
        self.preserve_first.hash(&mut h);
        self.preserve_order.hash(&mut h);
        self.best_start_point.hash(&mut h);
        self.prefer_corners.hash(&mut h);
        self.merge_scanlines
            .as_ref()
            .map(|m| m.cache_key())
            .unwrap_or(0)
            .hash(&mut h);
        h.finish()
    }
}

#[derive(Clone)]
struct WorkpieceMeta {
    uid: String,
    ops: Ops,
    entry_point: Point3D,
    exit_point: Point3D,
    can_flip: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point2D([f64; 2]);

impl rstar::Point for Point2D {
    type Scalar = f64;
    const DIMENSIONS: usize = 2;

    fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
        Point2D([generator(0), generator(1)])
    }

    fn nth(&self, index: usize) -> Self::Scalar {
        self.0[index]
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        &mut self.0[index]
    }
}

#[derive(Clone, PartialEq)]
struct SegmentPoint {
    point: Point2D,
    segment_idx: usize,
    /// True when this point is the segment's exit (used to flip open
    /// segments on entry).
    is_exit: bool,
    /// When set, the segment is a closed path and this is the command
    /// index of the vertex it should be entered at (cut planner).
    vertex: Option<usize>,
}

impl RTreeObject for SegmentPoint {
    type Envelope = AABB<Point2D>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.point)
    }
}

impl PointDistance for SegmentPoint {
    fn distance_2(&self, other: &Point2D) -> <Point2D as rstar::Point>::Scalar {
        let dx = self.point.0[0] - other.0[0];
        let dy = self.point.0[1] - other.0[1];
        dx * dx + dy * dy
    }
}

fn dist_xy(p1: Point3D, p2: Point3D) -> f64 {
    let dx = p1.x - p2.x;
    let dy = p1.y - p2.y;
    dx.hypot(dy)
}

fn can_flip(ops: &Ops) -> bool {
    for i in 0..ops.len() {
        if ops.is_cutting(i) {
            return true;
        }
    }
    false
}

fn split_by_workpiece_markers(ops: &Ops) -> Vec<(String, Ops)> {
    let mut blocks: Vec<(String, Ops)> = Vec::new();
    let mut current_uid: Option<String> = None;
    let mut current_block = Ops::new();

    for i in 0..ops.len() {
        let ct = ops.command_type(i);
        if ct == CommandType::WorkpieceStart {
            current_uid = Some(ops.workpiece_uid(i).to_string());
            current_block = Ops::new();
        } else if ct == CommandType::WorkpieceEnd {
            if let Some(uid) = current_uid.take() {
                blocks.push((uid, current_block.clone()));
            }
            current_block = Ops::new();
        } else if ops.category(i) == CommandCategory::Moving
            && current_uid.is_some()
        {
            current_block.transfer_command_from(ops, i);
        }
    }

    if let Some(uid) = current_uid {
        if !current_block.is_empty() {
            blocks.push((uid, current_block));
        }
    }

    blocks
}

fn extract_workpiece_meta(uid: &str, ops: &Ops) -> Option<WorkpieceMeta> {
    if ops.is_empty() {
        return None;
    }

    let entry_point = find_pass_entry(ops)?;
    let exit_point = find_pass_exit(ops)?;

    Some(WorkpieceMeta {
        uid: uid.to_string(),
        ops: ops.clone(),
        entry_point,
        exit_point,
        can_flip: can_flip(ops),
    })
}

fn kdtree_order_workpieces(metas: &mut [WorkpieceMeta]) -> Vec<WorkpieceMeta> {
    let n = metas.len();
    if n < 2 {
        return metas.to_vec();
    }

    let mut entry_points: Vec<Point2D> = Vec::with_capacity(n);
    let mut exit_points: Vec<Point2D> = Vec::with_capacity(n);
    let mut points: Vec<SegmentPoint> = Vec::with_capacity(n * 2);
    for (i, meta) in metas.iter().enumerate() {
        let entry = Point2D([meta.entry_point.x, meta.entry_point.y]);
        let exit = Point2D([meta.exit_point.x, meta.exit_point.y]);
        entry_points.push(entry);
        exit_points.push(exit);
        points.push(SegmentPoint {
            point: entry,
            segment_idx: i,
            is_exit: false,
            vertex: None,
        });
        points.push(SegmentPoint {
            point: exit,
            segment_idx: i,
            is_exit: true,
            vertex: None,
        });
    }

    let mut tree = RTree::bulk_load(points);
    let mut ordered: Vec<WorkpieceMeta> = Vec::with_capacity(n);

    ordered.push(metas[0].clone());
    let mut current_pos =
        Point2D([metas[0].exit_point.x, metas[0].exit_point.y]);

    tree.remove(&SegmentPoint {
        point: entry_points[0],
        segment_idx: 0,
        is_exit: false,
        vertex: None,
    });
    tree.remove(&SegmentPoint {
        point: exit_points[0],
        segment_idx: 0,
        is_exit: true,
        vertex: None,
    });

    while ordered.len() < n {
        let sp = match tree.nearest_neighbor(current_pos) {
            Some(sp) => sp,
            None => break,
        };

        let seg_idx = sp.segment_idx;
        let mut next_meta = metas[seg_idx].clone();
        if next_meta.can_flip && sp.is_exit {
            next_meta = WorkpieceMeta {
                uid: next_meta.uid.clone(),
                ops: next_meta.ops.flip_ops(),
                entry_point: next_meta.exit_point,
                exit_point: next_meta.entry_point,
                can_flip: next_meta.can_flip,
            };
            metas[seg_idx] = next_meta.clone();
        }

        ordered.push(next_meta.clone());
        current_pos = Point2D([next_meta.exit_point.x, next_meta.exit_point.y]);

        tree.remove(&SegmentPoint {
            point: entry_points[seg_idx],
            segment_idx: seg_idx,
            is_exit: false,
            vertex: None,
        });
        tree.remove(&SegmentPoint {
            point: exit_points[seg_idx],
            segment_idx: seg_idx,
            is_exit: true,
            vertex: None,
        });
    }

    ordered
}

fn two_opt_workpieces(
    ordered: &mut [WorkpieceMeta],
    callbacks: &dyn Callbacks,
) {
    let n = ordered.len();
    if n < 3 {
        return;
    }

    let mut iter_count = 0;
    let mut improved = true;

    while improved && iter_count < TWO_OPT_MAX_ITER {
        if callbacks.is_cancelled() {
            return;
        }
        improved = false;
        for i in 0..n - 2 {
            for j in i + 2..n {
                let a_exit = ordered[i].exit_point;
                let b_entry = ordered[i + 1].entry_point;
                let e_exit = ordered[j].exit_point;

                let (curr_cost, new_cost) = if j < n - 1 {
                    let f_entry = ordered[j + 1].entry_point;
                    let curr =
                        dist_xy(a_exit, b_entry) + dist_xy(e_exit, f_entry);
                    let new_ =
                        dist_xy(a_exit, e_exit) + dist_xy(b_entry, f_entry);
                    (curr, new_)
                } else {
                    (dist_xy(a_exit, b_entry), dist_xy(a_exit, e_exit))
                };

                if new_cost < curr_cost {
                    let mut sub = ordered[i + 1..=j].to_vec();
                    for item in &mut sub {
                        if item.can_flip {
                            *item = WorkpieceMeta {
                                uid: item.uid.clone(),
                                ops: item.ops.flip_ops(),
                                entry_point: item.exit_point,
                                exit_point: item.entry_point,
                                can_flip: item.can_flip,
                            };
                        }
                    }
                    sub.reverse();
                    for k in (i + 1)..=j {
                        ordered[k] = sub[k - (i + 1)].clone();
                    }
                    improved = true;
                }
            }
        }
        iter_count += 1;
    }
}

fn group_paths_power_agnostic(ops: &Ops) -> Vec<Ops> {
    let mut segments: Vec<Ops> = Vec::new();
    if ops.is_empty() {
        return segments;
    }

    let mut i = 0;
    while i < ops.len() {
        if !ops.is_travel(i) {
            i += 1;
            continue;
        }
        let mut current_segment = Ops::new();
        current_segment.transfer_command_from(ops, i);
        i += 1;
        while i < ops.len() && !ops.is_travel(i) {
            current_segment.transfer_command_from(ops, i);
            i += 1;
        }
        segments.push(current_segment);
    }
    segments
}

fn split_scanline(move_idx: usize, scan_idx: usize, ops: &Ops) -> Vec<Ops> {
    let pv = ops.scanline_data(scan_idx);
    if pv.is_empty() || pv.iter().all(|&b| b == 0) {
        return Vec::new();
    }

    let mut result = Ops::new();
    result.cmds_mut().reserve(2);
    result.transfer_command_from(ops, move_idx);
    result.transfer_command_from(ops, scan_idx);
    vec![result]
}

fn group_mixed_continuity(ops: &Ops) -> Vec<Ops> {
    let mut segments: Vec<Ops> = Vec::new();
    if ops.is_empty() {
        return segments;
    }

    let mut i = 0;
    while i < ops.len() {
        if !ops.is_travel(i) {
            i += 1;
            continue;
        }

        if i + 1 < ops.len() && ops.is_scanline(i + 1) {
            let sub_segments = split_scanline(i, i + 1, ops);
            segments.extend(sub_segments);
            i += 2;
        } else {
            let mut current_segment = Ops::new();
            current_segment.transfer_command_from(ops, i);
            i += 1;
            while i < ops.len() && !ops.is_travel(i) {
                if ops.is_scanline(i) {
                    break;
                }
                current_segment.transfer_command_from(ops, i);
                i += 1;
            }
            segments.push(current_segment);
        }
    }
    segments
}

fn is_closed_segment(seg: &Ops) -> bool {
    if seg.len() < 2 {
        return false;
    }
    let start = seg.endpoint(0);
    let mut last = start;
    for i in (0..seg.len()).rev() {
        if seg.category(i) == CommandCategory::Moving {
            last = seg.endpoint(i);
            break;
        }
    }
    dist_xy(start, last) < CLOSED_PATH_EPS
}

/// Direction change in degrees at vertex *b* between *a* and *c*.
fn corner_angle_deg(a: Point3D, b: Point3D, c: Point3D) -> f64 {
    let v1x = b.x - a.x;
    let v1y = b.y - a.y;
    let v2x = c.x - b.x;
    let v2y = c.y - b.y;
    let l1 = v1x.hypot(v1y);
    let l2 = v2x.hypot(v2y);
    if l1 < 1e-12 || l2 < 1e-12 {
        return 0.0;
    }
    let dot = (v1x * v2x + v1y * v2y) / (l1 * l2);
    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

/// Command indices of the vertices of a closed subpath, in cyclic
/// order. A subpath is `[MoveTo, LineTo, …]` (optionally closing back
/// to its start), so every moving command ends at a vertex. With
/// `prefer_corners` only vertices whose direction changes by more than
/// [`CORNER_ANGLE_DEG`] are kept; paths without corners (circles) keep
/// every vertex. Returns `None` for open paths or degenerate loops.
fn closed_segment_vertex_indices(
    seg: &Ops,
    prefer_corners: bool,
) -> Option<Vec<usize>> {
    if !is_closed_segment(seg) {
        return None;
    }
    let moving: Vec<usize> = (0..seg.len())
        .filter(|&i| seg.category(i) == CommandCategory::Moving)
        .collect();
    if moving.len() < 3 {
        return None; // a loop needs at least three vertices
    }
    if !prefer_corners {
        return Some(moving);
    }
    let pts: Vec<Point3D> = moving.iter().map(|&i| seg.endpoint(i)).collect();
    let n = pts.len();
    let corners: Vec<usize> = (0..n)
        .filter(|&v| {
            corner_angle_deg(pts[(v + n - 1) % n], pts[v], pts[(v + 1) % n])
                > CORNER_ANGLE_DEG
        })
        .map(|v| moving[v])
        .collect();
    if corners.is_empty() {
        return Some(moving);
    }
    Some(corners)
}

/// Rotate a closed subpath so it starts (MoveTo) at the vertex ended by
/// the command *vertex_cmd*, keeping the command order cyclically
/// intact. The path stays closed: the final command still ends at the
/// new start. State commands ahead of the path are kept in front of it.
fn rotate_closed_segment(seg: &Ops, vertex_cmd: usize) -> Ops {
    let mut out = Ops::new();
    let moving: Vec<usize> = (0..seg.len())
        .filter(|&i| seg.category(i) == CommandCategory::Moving)
        .collect();
    if moving.len() < 2 || vertex_cmd == moving[0] {
        return seg.copy();
    }
    for i in 0..moving[0] {
        out.transfer_command_from(seg, i);
    }
    let start = seg.endpoint(vertex_cmd);
    out.move_to(start.x, start.y, start.z, None);
    // The line commands (everything after the initial MoveTo) are
    // replayed cyclically starting just after the chosen vertex, so the
    // loop wraps around and ends back at the new start.
    let lines = &moving[1..];
    let n_lines = lines.len();
    let pos = lines.iter().position(|&i| i == vertex_cmd).unwrap();
    for offset in 1..=n_lines {
        out.transfer_command_from(seg, lines[(pos + offset) % n_lines]);
    }
    for i in (moving[moving.len() - 1] + 1)..seg.len() {
        out.transfer_command_from(seg, i);
    }
    out
}

fn kdtree_order_segments(
    segments: &mut [Ops],
    allow_flip: bool,
    best_start_point: bool,
    prefer_corners: bool,
) -> Vec<Ops> {
    let n = segments.len();
    if n < 2 {
        return segments.to_vec();
    }

    let mut points_by_segment: Vec<Vec<SegmentPoint>> = Vec::with_capacity(n);
    let mut points: Vec<SegmentPoint> = Vec::with_capacity(n * 2);
    for (i, seg) in segments.iter().enumerate() {
        let start = seg.endpoint(0);
        let end = seg.endpoint(seg.len() - 1);
        let start_pt = Point2D([start.x, start.y]);
        let end_pt = Point2D([end.x, end.y]);

        let vertices = if best_start_point {
            closed_segment_vertex_indices(seg, prefer_corners)
        } else {
            None
        };
        let seg_points = if let Some(cmds) = vertices {
            cmds.iter()
                .map(|&cmd| {
                    let p = seg.endpoint(cmd);
                    SegmentPoint {
                        point: Point2D([p.x, p.y]),
                        segment_idx: i,
                        is_exit: false,
                        vertex: Some(cmd),
                    }
                })
                .collect()
        } else {
            vec![
                SegmentPoint {
                    point: start_pt,
                    segment_idx: i,
                    is_exit: false,
                    vertex: None,
                },
                SegmentPoint {
                    point: end_pt,
                    segment_idx: i,
                    is_exit: true,
                    vertex: None,
                },
            ]
        };
        points_by_segment.push(seg_points.clone());
        points.extend(seg_points);
    }

    let mut tree = RTree::bulk_load(points);
    let mut ordered: Vec<Ops> = Vec::with_capacity(n);

    let first_seg = &segments[0];
    ordered.push(first_seg.clone());
    let last = first_seg.endpoint(first_seg.len() - 1);
    let mut current_pos = Point2D([last.x, last.y]);

    remove_segment_points(&mut tree, &points_by_segment[0]);

    while ordered.len() < n {
        let sp = match tree.nearest_neighbor(current_pos) {
            Some(sp) => sp,
            None => break,
        };

        let seg_idx = sp.segment_idx;
        let next_seg = match sp.vertex {
            Some(cmd) => rotate_closed_segment(&segments[seg_idx], cmd),
            None if sp.is_exit && allow_flip => segments[seg_idx].flip_ops(),
            None => segments[seg_idx].clone(),
        };

        let last = next_seg.endpoint(next_seg.len() - 1);
        current_pos = Point2D([last.x, last.y]);
        ordered.push(next_seg);

        remove_segment_points(&mut tree, &points_by_segment[seg_idx]);
    }

    ordered
}

fn remove_segment_points(
    tree: &mut RTree<SegmentPoint>,
    points: &[SegmentPoint],
) {
    for p in points {
        tree.remove(p);
    }
}

fn two_opt(ordered: &mut [Ops], allow_flip: bool, callbacks: &dyn Callbacks) {
    let n = ordered.len();
    if n < 3 {
        return;
    }

    let mut iter_count = 0;
    let mut improved = true;

    while improved && iter_count < TWO_OPT_MAX_ITER {
        if callbacks.is_cancelled() {
            return;
        }
        improved = false;
        for i in 0..n - 2 {
            for j in i + 2..n {
                let a_end = ordered[i].endpoint(ordered[i].len() - 1);
                let b_start = ordered[i + 1].endpoint(0);
                let e_end = ordered[j].endpoint(ordered[j].len() - 1);

                let (curr_cost, new_cost) = if j < n - 1 {
                    let f_start = ordered[j + 1].endpoint(0);
                    let curr =
                        dist_xy(a_end, b_start) + dist_xy(e_end, f_start);
                    let new_ =
                        dist_xy(a_end, e_end) + dist_xy(b_start, f_start);
                    (curr, new_)
                } else {
                    (dist_xy(a_end, b_start), dist_xy(a_end, e_end))
                };

                if new_cost < curr_cost {
                    let mut sub = ordered[i + 1..=j].to_vec();
                    if allow_flip {
                        for seg in &mut sub {
                            *seg = seg.flip_ops();
                        }
                    }
                    sub.reverse();
                    for k in (i + 1)..=j {
                        ordered[k] = sub[k - (i + 1)].clone();
                    }
                    improved = true;
                }
            }
        }
        iter_count += 1;
    }
}

#[derive(Clone, Debug)]
enum OptJob {
    Passthrough {
        original_index: usize,
        segment: Ops,
    },
    KdtreeOnly {
        original_index: usize,
        sub_segments: Vec<Ops>,
    },
    TwoOpt {
        original_index: usize,
        sub_segments: Vec<Ops>,
    },
}

/// True when *ops* consists solely of strict ``(MoveTo, ScanLine)``
/// pairs — the raster assembler's segmented-mode output, where every
/// travel is immediately followed by one scanline run and there are no
/// other cutting commands.
fn is_scanline_only(ops: &Ops) -> bool {
    if ops.is_empty() {
        return false;
    }
    let mut expect_travel = true;
    for i in 0..ops.len() {
        if expect_travel {
            if !ops.is_travel(i) {
                return false;
            }
            expect_travel = false;
        } else if !ops.is_scanline(i) {
            return false;
        } else {
            expect_travel = true;
        }
    }
    !expect_travel
}

fn prepare_optimization_jobs(long_segments: &[(usize, &Ops)]) -> Vec<OptJob> {
    let mut jobs = Vec::with_capacity(long_segments.len());
    let mut two_opt_candidates: Vec<(usize, Vec<Ops>, usize)> =
        Vec::with_capacity(long_segments.len());

    for (original_index, long_segment) in long_segments {
        if long_segment.is_empty() || long_segment.is_marker(0) {
            jobs.push(OptJob::Passthrough {
                original_index: *original_index,
                segment: (*long_segment).clone(),
            });
            continue;
        }

        let contains_scanline =
            (0..long_segment.len()).any(|j| long_segment.is_scanline(j));
        let sub_segments = if contains_scanline {
            group_mixed_continuity(long_segment)
        } else {
            group_paths_power_agnostic(long_segment)
        };

        let num_sub_segments = sub_segments.len();

        if num_sub_segments <= 1 {
            jobs.push(OptJob::Passthrough {
                original_index: *original_index,
                segment: (*long_segment).clone(),
            });
            continue;
        }

        if num_sub_segments > TWO_OPT_SEGMENT_THRESHOLD {
            jobs.push(OptJob::KdtreeOnly {
                original_index: *original_index,
                sub_segments,
            });
        } else {
            let command_count: usize =
                sub_segments.iter().map(|s| s.len()).sum();
            two_opt_candidates.push((
                *original_index,
                sub_segments,
                command_count,
            ));
        }
    }

    two_opt_candidates.sort_by_key(|c| c.2);

    let mut bucketed_command_count = 0;
    for (original_index, sub_segments, command_count) in two_opt_candidates {
        if bucketed_command_count + command_count <= TWO_OPT_COMMAND_LIMIT {
            jobs.push(OptJob::TwoOpt {
                original_index,
                sub_segments,
            });
            bucketed_command_count += command_count;
        } else {
            jobs.push(OptJob::KdtreeOnly {
                original_index,
                sub_segments,
            });
        }
    }

    jobs
}

/// One ``(MoveTo, ScanLine)`` run of a scanline-only segment, addressed
/// by command indices into the source ops. `flip` marks runs whose
/// direction was reversed by the optimizer.
#[derive(Clone, Copy)]
struct ScanRun {
    move_idx: usize,
    scan_idx: usize,
    flip: bool,
}

/// True when the ScanLine command at *idx* carries any non-zero power.
fn run_is_nonzero(ops: &Ops, idx: usize) -> bool {
    match &ops.commands[idx].category {
        OpCategory::Moving {
            cmd: MoveCmd::ScanLine { power_values },
            ..
        } => !power_values.is_empty() && power_values.iter().any(|&b| b != 0),
        _ => false,
    }
}

/// Effective start point of a run (flip-aware).
fn run_start(long_segment: &Ops, run: &ScanRun) -> Point3D {
    if run.flip {
        long_segment.endpoint(run.scan_idx)
    } else {
        long_segment.endpoint(run.move_idx)
    }
}

/// Effective end point of a run (flip-aware).
fn run_end(long_segment: &Ops, run: &ScanRun) -> Point3D {
    if run.flip {
        long_segment.endpoint(run.move_idx)
    } else {
        long_segment.endpoint(run.scan_idx)
    }
}

/// Optimize a scanline-only long segment in place as index ranges.
///
/// The raster assembler's segmented mode emits one ``(MoveTo,
/// ScanLine)`` pair per zero-power-delimited run, already in
/// near-optimal order. Reordering hundreds of thousands of runs through
/// the classic segment path would materialize an ``Ops`` object per
/// run, so the runs are kept as index ranges into *long_segment* and
/// reordered with the same KD-tree nearest-neighbor walk (plus 2-opt
/// refinement for small run counts).
///
/// Returns ``None`` when the segment has at most one non-empty run —
/// the caller then keeps the segment as-is, matching the classic
/// passthrough behavior.
fn optimize_scanline_runs(
    long_segment: &Ops,
    allow_flip: bool,
    callbacks: &dyn Callbacks,
) -> Option<Vec<ScanRun>> {
    let mut runs: Vec<ScanRun> = Vec::with_capacity(long_segment.len() / 2);
    let mut i = 0;
    while i + 1 < long_segment.len() {
        if long_segment.is_travel(i) && long_segment.is_scanline(i + 1) {
            if run_is_nonzero(long_segment, i + 1) {
                runs.push(ScanRun {
                    move_idx: i,
                    scan_idx: i + 1,
                    flip: false,
                });
            }
            i += 2;
        } else {
            return None;
        }
    }

    if runs.len() <= 1 {
        return None;
    }

    let mut ordered = kdtree_order_runs(&runs, long_segment, allow_flip);
    if ordered.len() <= TWO_OPT_SEGMENT_THRESHOLD {
        two_opt_runs(&mut ordered, long_segment, allow_flip, callbacks);
    }
    Some(ordered)
}

fn kdtree_order_runs(
    runs: &[ScanRun],
    long_segment: &Ops,
    allow_flip: bool,
) -> Vec<ScanRun> {
    let n = runs.len();
    let mut entry_points: Vec<Point2D> = Vec::with_capacity(n);
    let mut exit_points: Vec<Point2D> = Vec::with_capacity(n);
    let mut points: Vec<SegmentPoint> = Vec::with_capacity(n * 2);
    for (i, run) in runs.iter().enumerate() {
        let start = long_segment.endpoint(run.move_idx);
        let end = long_segment.endpoint(run.scan_idx);
        let start_pt = Point2D([start.x, start.y]);
        let end_pt = Point2D([end.x, end.y]);
        entry_points.push(start_pt);
        exit_points.push(end_pt);
        points.push(SegmentPoint {
            point: start_pt,
            segment_idx: i,
            is_exit: false,
            vertex: None,
        });
        points.push(SegmentPoint {
            point: end_pt,
            segment_idx: i,
            is_exit: true,
            vertex: None,
        });
    }

    let mut tree = RTree::bulk_load(points);
    let mut ordered: Vec<ScanRun> = Vec::with_capacity(n);

    ordered.push(runs[0]);
    let last = long_segment.endpoint(runs[0].scan_idx);
    let mut current_pos = Point2D([last.x, last.y]);

    tree.remove(&SegmentPoint {
        point: entry_points[0],
        segment_idx: 0,
        is_exit: false,
        vertex: None,
    });
    tree.remove(&SegmentPoint {
        point: exit_points[0],
        segment_idx: 0,
        is_exit: true,
        vertex: None,
    });

    while ordered.len() < n {
        let sp = match tree.nearest_neighbor(current_pos) {
            Some(sp) => sp,
            None => break,
        };

        let seg_idx = sp.segment_idx;
        let mut next_run = runs[seg_idx];
        if sp.is_exit && allow_flip {
            next_run.flip = true;
        }

        let last = run_end(long_segment, &next_run);
        current_pos = Point2D([last.x, last.y]);
        ordered.push(next_run);

        tree.remove(&SegmentPoint {
            point: entry_points[seg_idx],
            segment_idx: seg_idx,
            is_exit: false,
            vertex: None,
        });
        tree.remove(&SegmentPoint {
            point: exit_points[seg_idx],
            segment_idx: seg_idx,
            is_exit: true,
            vertex: None,
        });
    }

    ordered
}

fn two_opt_runs(
    ordered: &mut [ScanRun],
    long_segment: &Ops,
    allow_flip: bool,
    callbacks: &dyn Callbacks,
) {
    let n = ordered.len();
    if n < 3 {
        return;
    }

    let mut iter_count = 0;
    let mut improved = true;

    while improved && iter_count < TWO_OPT_MAX_ITER {
        if callbacks.is_cancelled() {
            return;
        }
        improved = false;
        for i in 0..n - 2 {
            for j in i + 2..n {
                let a_end = run_end(long_segment, &ordered[i]);
                let b_start = run_start(long_segment, &ordered[i + 1]);
                let e_end = run_end(long_segment, &ordered[j]);

                let (curr_cost, new_cost) = if j < n - 1 {
                    let f_start = run_start(long_segment, &ordered[j + 1]);
                    let curr =
                        dist_xy(a_end, b_start) + dist_xy(e_end, f_start);
                    let new_ =
                        dist_xy(a_end, e_end) + dist_xy(b_start, f_start);
                    (curr, new_)
                } else {
                    (dist_xy(a_end, b_start), dist_xy(a_end, e_end))
                };

                if new_cost < curr_cost {
                    if allow_flip {
                        for run in ordered.iter_mut().take(j + 1).skip(i + 1) {
                            run.flip = !run.flip;
                        }
                    }
                    ordered[i + 1..=j].reverse();
                    improved = true;
                }
            }
        }
        iter_count += 1;
    }
}

fn emit_scanline_runs(
    out: &mut Ops,
    long_segment: &Ops,
    runs: &[ScanRun],
    prev: &mut State,
) {
    for run in runs {
        if run.flip {
            let scan_end = long_segment.endpoint(run.scan_idx);
            let move_end = long_segment.endpoint(run.move_idx);
            if let Some(state) = long_segment.state(run.move_idx) {
                *prev = sync_state_commands(out, state, prev);
            }
            out.move_to(scan_end.x, scan_end.y, scan_end.z, None);
            if let Some(state) = long_segment.state(run.scan_idx) {
                *prev = sync_state_commands(out, state, prev);
            }
            let power: Vec<u8> = long_segment
                .scanline_data(run.scan_idx)
                .into_iter()
                .rev()
                .collect();
            let extra =
                long_segment.extra_axes(run.scan_idx).map(|ea| ea.to_vec());
            out.scan_to(move_end.x, move_end.y, move_end.z, power, extra);
        } else {
            for j in [run.move_idx, run.scan_idx] {
                if let Some(state) = long_segment.state(j) {
                    *prev = sync_state_commands(out, state, prev);
                }
                out.transfer_command_from(long_segment, j);
            }
        }
    }
}

fn sync_state_commands(ops: &mut Ops, state: &State, prev: &State) -> State {
    let mut prev = prev.clone();
    if (state.power - prev.power).abs() > f64::EPSILON {
        ops.set_power(state.power);
        prev.power = state.power;
    }
    if let Some(cs) = state.feed_rate {
        if prev.feed_rate != Some(cs) {
            ops.set_feed_rate(cs);
            prev.feed_rate = Some(cs);
        }
    }
    if let Some(ts) = state.rapid_rate {
        if prev.rapid_rate != Some(ts) {
            ops.set_rapid_rate(ts);
            prev.rapid_rate = Some(ts);
        }
    }
    if state.coolant != prev.coolant {
        if let Some(mode) = state.coolant {
            ops.set_coolant(mode);
        }
        prev.coolant = state.coolant;
    }
    if state.air_assist != prev.air_assist {
        if let Some(mode) = state.air_assist {
            ops.set_air_assist(mode);
        }
        prev.air_assist = state.air_assist;
    }
    if state.head_coolant != prev.head_coolant {
        if let Some(mode) = state.head_coolant {
            ops.set_head_coolant(mode);
        }
        prev.head_coolant = state.head_coolant;
    }
    if state.power_mode != prev.power_mode {
        if let Some(mode) = state.power_mode {
            ops.set_power_mode(mode);
        }
        prev.power_mode = state.power_mode;
    }
    if let Some(ref uid) = state.active_head_uid {
        if prev.active_head_uid.as_deref() != Some(uid.as_str()) {
            ops.set_head(uid);
            prev.active_head_uid = Some(uid.clone());
        }
    }
    prev
}

/// Optimize travel distance in an Ops sequence.
///
/// Performs two levels of optimization:
/// 1. Workpiece-level: Reorders and flips workpieces to minimize
///    inter-workpiece travel (when multiple workpieces are present).
/// 2. Segment-level: Reorders path segments within each workpiece
///    using KDTree nearest-neighbor and optionally 2-opt refinement.
///
/// - `ops`: The Ops sequence to optimize (modified in place).
/// - `allow_flip`: Whether to allow flipping subpaths.
/// - `preserve_first`: Whether to keep the first workpiece in place.
/// - `preserve_order`: List of workpiece UIDs whose order must be
///   preserved.
/// - `callbacks`: Callback bundle for progress reports and
///   cancellation polling.
pub fn optimize_travel(
    ops: &mut Ops,
    allow_flip: bool,
    preserve_first: bool,
    preserve_order: Vec<String>,
    callbacks: &dyn Callbacks,
    best_start_point: bool,
    prefer_corners: bool,
) {
    ops.preload_state();

    let blocks = split_by_workpiece_markers(ops);
    if blocks.len() >= 2 {
        optimize_workpiece_order(
            ops,
            &blocks,
            allow_flip,
            preserve_first,
            &preserve_order,
            callbacks,
        );
        return;
    }

    optimize_segments(
        ops,
        allow_flip,
        best_start_point,
        prefer_corners,
        callbacks,
    );
}

fn report_progress(callbacks: &dyn Callbacks, progress: f64, message: &str) {
    callbacks.report_progress(progress, message);
}

fn optimize_workpiece_order(
    ops: &mut Ops,
    blocks: &[(String, Ops)],
    allow_flip: bool,
    preserve_first: bool,
    preserve_order: &[String],
    callbacks: &dyn Callbacks,
) {
    report_progress(callbacks, 0.0, "Analyzing workpieces...");

    let mut metas: Vec<WorkpieceMeta> = Vec::new();
    for (uid, block_ops) in blocks {
        if let Some(mut meta) = extract_workpiece_meta(uid, block_ops) {
            if !allow_flip {
                meta.can_flip = false;
            }
            metas.push(meta);
        }
    }

    if metas.len() < 2 {
        return;
    }

    let preserved_set: HashSet<String> =
        preserve_order.iter().cloned().collect();
    let mut preserved_indices: HashSet<usize> = HashSet::new();
    let mut reorderable_metas: Vec<WorkpieceMeta> = Vec::new();

    for (i, meta) in metas.iter().enumerate() {
        if preserved_set.contains(&meta.uid) || (preserve_first && i == 0) {
            preserved_indices.insert(i);
        } else {
            reorderable_metas.push(meta.clone());
        }
    }

    if reorderable_metas.is_empty() {
        return;
    }

    report_progress(callbacks, 0.1, "Optimizing workpiece order...");

    let ordered_metas = kdtree_order_workpieces(&mut reorderable_metas);

    let mut ordered_metas = ordered_metas;
    two_opt_workpieces(&mut ordered_metas, callbacks);

    report_progress(callbacks, 0.9, "Reassembling optimized workpieces...");

    if !preserved_indices.is_empty() {
        let mut final_metas: Vec<WorkpieceMeta> = Vec::new();
        let mut reorder_idx = 0;
        for (i, meta) in metas.iter().enumerate() {
            if preserved_indices.contains(&i) {
                final_metas.push(meta.clone());
            } else if reorder_idx < ordered_metas.len() {
                final_metas.push(ordered_metas[reorder_idx].clone());
                reorder_idx += 1;
            }
        }
        reassemble_workpieces(ops, &final_metas);
    } else {
        reassemble_workpieces(ops, &ordered_metas);
    }

    report_progress(callbacks, 1.0, "Workpiece optimization complete");
}

fn reassemble_workpieces(ops: &mut Ops, ordered_metas: &[WorkpieceMeta]) {
    ops.preload_state();
    ops.clear();

    let mut prev = State::default();
    for meta in ordered_metas {
        ops.workpiece_start(&meta.uid);
        for j in 0..meta.ops.len() {
            if let Some(state) = meta.ops.state(j) {
                prev = sync_state_commands(ops, state, &prev);
            }
            ops.transfer_command_from(&meta.ops, j);
        }
        ops.workpiece_end(&meta.uid);
    }
}

fn optimize_segments(
    ops: &mut Ops,
    allow_flip: bool,
    best_start_point: bool,
    prefer_corners: bool,
    callbacks: &dyn Callbacks,
) {
    report_progress(callbacks, 0.0, "Preprocessing for optimization...");

    let nons = ops.without_state();

    let long_segments = nons.group_by_auxiliary_state();

    report_progress(
        callbacks,
        0.05,
        "Analyzing and bucketing path segments...",
    );

    // Scanline-only long segments (the raster assembler's segmented
    // output): every run is already a zero-power-delimited (MoveTo,
    // ScanLine) pair, so they are optimized in place as index ranges
    // instead of materializing an Ops object per run (a dense engraving
    // has hundreds of thousands).  Segments with at most one non-empty
    // run fall back to the classic passthrough path.
    let mut scanline_units: std::collections::HashMap<usize, Vec<ScanRun>> =
        std::collections::HashMap::new();
    let mut processed_results: std::collections::HashMap<usize, Vec<Ops>> =
        std::collections::HashMap::new();
    let mut opt_inputs: Vec<(usize, &Ops)> =
        Vec::with_capacity(long_segments.len());
    for (i, seg) in long_segments.iter().enumerate() {
        let contains_scanline = (0..seg.len()).any(|j| seg.is_scanline(j));
        if contains_scanline && is_scanline_only(seg) {
            match optimize_scanline_runs(seg, allow_flip, callbacks) {
                Some(runs) => {
                    scanline_units.insert(i, runs);
                }
                None => {
                    processed_results.insert(i, vec![seg.clone()]);
                }
            }
        } else {
            opt_inputs.push((i, seg));
        }
    }

    let jobs = prepare_optimization_jobs(&opt_inputs);

    let total_workload: usize = jobs
        .iter()
        .map(|j| match j {
            OptJob::Passthrough { .. } => 1,
            OptJob::KdtreeOnly { sub_segments, .. } => sub_segments.len(),
            OptJob::TwoOpt { sub_segments, .. } => sub_segments.len(),
        })
        .max()
        .unwrap_or(1);

    let mut cumulative_workload: usize = 0;

    for (i, job) in jobs.iter().enumerate() {
        if callbacks.is_cancelled() {
            break;
        }

        let progress =
            0.05 + 0.85 * (cumulative_workload as f64 / total_workload as f64);

        match job {
            OptJob::Passthrough {
                original_index,
                segment,
            } => {
                processed_results
                    .insert(*original_index, vec![segment.clone()]);
            }
            OptJob::KdtreeOnly {
                original_index,
                sub_segments,
            }
            | OptJob::TwoOpt {
                original_index,
                sub_segments,
            } => {
                report_progress(
                    callbacks,
                    progress,
                    &format!("Optimizing segment {}/{}...", i + 1, jobs.len()),
                );

                let mut sub_segments = sub_segments.clone();
                let ordered = kdtree_order_segments(
                    &mut sub_segments,
                    allow_flip,
                    best_start_point,
                    prefer_corners,
                );

                let final_segments = if matches!(job, OptJob::TwoOpt { .. }) {
                    let mut segs = ordered;
                    two_opt(&mut segs, allow_flip, callbacks);
                    segs
                } else {
                    ordered
                };

                processed_results.insert(*original_index, final_segments);
            }
        }

        cumulative_workload += match job {
            OptJob::Passthrough { .. } => 1,
            OptJob::KdtreeOnly { sub_segments, .. } => sub_segments.len(),
            OptJob::TwoOpt { sub_segments, .. } => sub_segments.len(),
        };
    }

    report_progress(callbacks, 0.9, "Reassembling optimized paths...");

    ops.clear();
    let mut prev = State::default();
    for (i, seg) in long_segments.iter().enumerate() {
        if let Some(runs) = scanline_units.get(&i) {
            emit_scanline_runs(ops, seg, runs, &mut prev);
            continue;
        }
        let Some(segments) = processed_results.get(&i) else {
            continue;
        };
        for segment_ops in segments {
            if segment_ops.is_empty() {
                continue;
            }
            if segment_ops.is_marker(0) {
                ops.transfer_command_from(segment_ops, 0);
                continue;
            }
            for j in 0..segment_ops.len() {
                if let Some(state) = segment_ops.state(j) {
                    prev = sync_state_commands(ops, state, &prev);
                }
                ops.transfer_command_from(segment_ops, j);
            }
        }
    }

    report_progress(callbacks, 1.0, "Optimization complete");
}
