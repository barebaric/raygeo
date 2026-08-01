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

const TWO_OPT_SEGMENT_THRESHOLD: usize = 1000;
const TWO_OPT_COMMAND_LIMIT: usize = 10000;
const TWO_OPT_MAX_ITER: usize = 10;

/// Parameters for the [`optimize_travel`] transformer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptimizeSpec {
    /// Whether flipping subpaths is allowed.
    pub allow_flip: bool,
    /// Keep the first workpiece in place.
    pub preserve_first: bool,
    /// Workpiece UIDs whose order to preserve.
    pub preserve_order: Vec<String>,
}

impl Transformer for OptimizeSpec {
    fn phase(&self) -> Phase {
        Phase::GeometryRefinement
    }

    fn apply(&self, ctx: &mut TransformCtx<'_>) {
        optimize_travel(
            ctx.ops,
            self.allow_flip,
            self.preserve_first,
            self.preserve_order.clone(),
            ctx.callbacks,
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
    is_exit: bool,
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
        });
        points.push(SegmentPoint {
            point: exit,
            segment_idx: i,
            is_exit: true,
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
    });
    tree.remove(&SegmentPoint {
        point: exit_points[0],
        segment_idx: 0,
        is_exit: true,
    });

    while ordered.len() < n {
        let sp = match tree.nearest_neighbor(&current_pos) {
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
        });
        tree.remove(&SegmentPoint {
            point: exit_points[seg_idx],
            segment_idx: seg_idx,
            is_exit: true,
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

fn kdtree_order_segments(segments: &mut [Ops], allow_flip: bool) -> Vec<Ops> {
    let n = segments.len();
    if n < 2 {
        return segments.to_vec();
    }

    let mut entry_points: Vec<Point2D> = Vec::with_capacity(n);
    let mut exit_points: Vec<Point2D> = Vec::with_capacity(n);
    let mut points: Vec<SegmentPoint> = Vec::with_capacity(n * 2);
    for (i, seg) in segments.iter().enumerate() {
        let start = seg.endpoint(0);
        let end = seg.endpoint(seg.len() - 1);
        let start_pt = Point2D([start.x, start.y]);
        let end_pt = Point2D([end.x, end.y]);
        entry_points.push(start_pt);
        exit_points.push(end_pt);
        points.push(SegmentPoint {
            point: start_pt,
            segment_idx: i,
            is_exit: false,
        });
        points.push(SegmentPoint {
            point: end_pt,
            segment_idx: i,
            is_exit: true,
        });
    }

    let mut tree = RTree::bulk_load(points);
    let mut ordered: Vec<Ops> = Vec::with_capacity(n);

    let first_seg = &segments[0];
    ordered.push(first_seg.clone());
    let last = first_seg.endpoint(first_seg.len() - 1);
    let mut current_pos = Point2D([last.x, last.y]);

    tree.remove(&SegmentPoint {
        point: entry_points[0],
        segment_idx: 0,
        is_exit: false,
    });
    tree.remove(&SegmentPoint {
        point: exit_points[0],
        segment_idx: 0,
        is_exit: true,
    });

    while ordered.len() < n {
        let sp = match tree.nearest_neighbor(&current_pos) {
            Some(sp) => sp,
            None => break,
        };

        let seg_idx = sp.segment_idx;
        let next_seg = if sp.is_exit && allow_flip {
            segments[seg_idx].flip_ops()
        } else {
            segments[seg_idx].clone()
        };

        let last = next_seg.endpoint(next_seg.len() - 1);
        current_pos = Point2D([last.x, last.y]);
        ordered.push(next_seg);

        tree.remove(&SegmentPoint {
            point: entry_points[seg_idx],
            segment_idx: seg_idx,
            is_exit: false,
        });
        tree.remove(&SegmentPoint {
            point: exit_points[seg_idx],
            segment_idx: seg_idx,
            is_exit: true,
        });
    }

    ordered
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

fn prepare_optimization_jobs(long_segments: &[Ops]) -> Vec<OptJob> {
    let mut jobs = Vec::new();
    let mut two_opt_candidates: Vec<(usize, Vec<Ops>, usize)> = Vec::new();

    for (i, long_segment) in long_segments.iter().enumerate() {
        if long_segment.is_empty() || long_segment.is_marker(0) {
            jobs.push(OptJob::Passthrough {
                original_index: i,
                segment: long_segment.clone(),
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
                original_index: i,
                segment: long_segment.clone(),
            });
            continue;
        }

        if num_sub_segments > TWO_OPT_SEGMENT_THRESHOLD {
            jobs.push(OptJob::KdtreeOnly {
                original_index: i,
                sub_segments,
            });
        } else {
            let command_count: usize =
                sub_segments.iter().map(|s| s.len()).sum();
            two_opt_candidates.push((i, sub_segments, command_count));
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

    optimize_segments(ops, allow_flip, callbacks);
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

    let jobs = prepare_optimization_jobs(&long_segments);

    let total_workload: usize = jobs
        .iter()
        .map(|j| match j {
            OptJob::Passthrough { .. } => 1,
            OptJob::KdtreeOnly { sub_segments, .. } => sub_segments.len(),
            OptJob::TwoOpt { sub_segments, .. } => sub_segments.len(),
        })
        .max()
        .unwrap_or(1);

    let mut processed_results: std::collections::HashMap<usize, Vec<Ops>> =
        std::collections::HashMap::new();
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
                let ordered =
                    kdtree_order_segments(&mut sub_segments, allow_flip);

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

    let mut flat_result: Vec<Ops> = Vec::new();
    for i in 0..long_segments.len() {
        if let Some(segments) = processed_results.get(&i) {
            flat_result.extend(segments.iter().cloned());
        }
    }

    ops.clear();
    let mut prev = State::default();
    for segment_ops in &flat_result {
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

    report_progress(callbacks, 1.0, "Optimization complete");
}
