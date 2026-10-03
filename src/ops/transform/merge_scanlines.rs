//! Merge collinear cut lines into acceleration-aware scanlines.
//!
//! Detects parallel cut lines that lie on the same scan row — the same
//! direction, the same perpendicular offset — and bridges the gaps
//! between them at zero power when the machine's acceleration profile
//! makes the merged sweep faster than decelerating, traveling across
//! the gap, and re-accelerating.
//!
//! This is what makes engraving arrays efficient (#495): instead of
//! planning each array element as an independent unit with full
//! stop/start cycles between them, rows that span multiple elements
//! become single long sweeps.
//!
//! The merge decision never enumerates gap subsets and never compares
//! all pairs of lines: candidates are bucketed into rows by hashing,
//! and each row is merged in one left-to-right sweep whose per-gap
//! cost comparison is O(1). Total cost is O(n log n) for n candidate
//! lines.

use std::collections::HashMap;
use std::sync::Arc;

use crate::geo::types::Point3D;
use crate::ops::callbacks::Callbacks;
use crate::ops::container::time::move_duration;
use crate::ops::container::Ops;
use crate::ops::enums::{RasterMode, SectionType};
use crate::ops::state::State;
use crate::ops::transform::{Phase, TransformCtx, Transformer};
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory, StateCmd};

/// Width of an angle bucket in radians. Two lines whose canonical
/// directions differ by less than this can share a row.
const ANGLE_BUCKET_RAD: f64 = 0.1_f64.to_radians();

/// Quantization step for Z when bucketing rows (100 nm). Lines only
/// share a row when their Z matches to well below any physical
/// tolerance; multi-pass Z levels stay separate.
const Z_QUANTUM: f64 = 1e-4;

/// Relative tolerance for matching the sample density (samples per
/// millimetre) of two scanlines before they may be bridged.
const DENSITY_TOLERANCE: f64 = 0.005;

/// Smallest perpendicular-offset difference treated as a real row
/// spacing (0.1 µm). Anything below is float noise between lines
/// generated at the same offset.
const PERP_NOISE_FLOOR: f64 = 1e-4;

/// Parameters for the [`merge_scanlines`] transformer.
#[derive(Clone, Debug, PartialEq)]
pub struct MergeScanlinesSpec {
    /// Machine acceleration in mm/s². Values <= 0 disable the cost
    /// model (gaps are then only limited by `max_gap_mm`).
    pub acceleration: f64,
    /// Fallback cut speed in mm/min for lines that carry no feed
    /// rate in their state.
    pub cut_speed: f64,
    /// Fallback rapid speed in mm/min.
    pub rapid_speed: f64,
    /// Manual ceiling on bridged gap length in mm. 0 = unlimited.
    pub max_gap_mm: f64,
    /// Maximum perpendicular distance for two parallel lines to be
    /// considered part of the same row, in mm.
    pub tolerance: f64,
}

impl Transformer for MergeScanlinesSpec {
    fn phase(&self) -> Phase {
        Phase::GeometryRefinement
    }

    fn apply(&self, ctx: &mut TransformCtx<'_>) {
        merge_scanlines(ctx.ops, self, ctx.callbacks);
    }

    fn name(&self) -> &str {
        "merge_scanlines"
    }

    fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name().hash(&mut h);
        self.acceleration.to_bits().hash(&mut h);
        self.cut_speed.to_bits().hash(&mut h);
        self.rapid_speed.to_bits().hash(&mut h);
        self.max_gap_mm.to_bits().hash(&mut h);
        self.tolerance.to_bits().hash(&mut h);
        h.finish()
    }
}

/// A straight cut line eligible for merging: a `MoveTo` followed by
/// exactly one straight cutting command (`ScanLine` or `LineTo`),
/// with only state commands in between.
struct Candidate {
    /// Low/high endpoints along the canonical row direction. A
    /// serpentine raster traverses each line in either direction,
    /// so `start`/`end` alone do not identify a row end.
    lo: Point3D,
    hi: Point3D,
    is_scanline: bool,
    power_values: Arc<Vec<u8>>,
    state: State,
    /// Projection of the *content* (non-zero-power) extent onto the
    /// canonical direction. Overscan pads the line ends with zero
    /// power; rows are keyed, sorted, and bridged on the content so
    /// padded neighbours neither overlap nor shrink the gap.
    proj_lo: f64,
    proj_hi: f64,
    /// Zero-power pad samples at the low/high content ends (in
    /// canonical orientation), kept so merged lines preserve the
    /// outermost overscan pads.
    pad_lo: usize,
    pad_hi: usize,
    /// Whether the original traversal ran along the canonical
    /// direction (lo -> hi).
    aligned: bool,
    /// Signed perpendicular offset of the row from the origin.
    perp: f64,
    angle_bucket: i64,
}

impl Candidate {
    fn feed_rate(&self, fallback: f64) -> f64 {
        self.state.feed_rate.map(f64::from).unwrap_or(fallback)
    }

    fn rapid_rate(&self, fallback: f64) -> f64 {
        self.state.rapid_rate.map(f64::from).unwrap_or(fallback)
    }

    fn content_len(&self) -> f64 {
        self.proj_hi - self.proj_lo
    }

    /// Samples covering the content extent (canonical orientation).
    fn content_samples(&self) -> std::ops::Range<usize> {
        self.pad_lo..self.power_values.len() - self.pad_hi
    }

    /// Power sample at canonical position *j*.
    fn sample(&self, j: usize) -> u8 {
        let n = self.power_values.len();
        if self.aligned {
            self.power_values[j]
        } else {
            self.power_values[n - 1 - j]
        }
    }

    fn density(&self) -> f64 {
        let len = self.content_len();
        if len <= 0.0 {
            0.0
        } else {
            self.content_samples().len() as f64 / len
        }
    }
}

/// Section signature of a unit: adjacent regions with the same
/// signature are processed as one unit so scanlines can merge across
/// the per-workpiece section wrappers of an array.
#[derive(Clone, Copy, PartialEq, Eq)]
struct UnitSignature {
    section_type: Option<SectionType>,
    raster_mode: Option<RasterMode>,
}

/// A contiguous command range to process as one unit.
struct Unit {
    start: usize,
    end: usize,
    signature: UnitSignature,
    candidates: Vec<Candidate>,
}

/// A run of row segments bridged into one sweep. `proj_lo`/`proj_hi`
/// bound the run along the row direction, bridges included.
struct Run {
    segments: Vec<usize>,
    proj_lo: f64,
    proj_hi: f64,
}

pub fn merge_scanlines(
    ops: &mut Ops,
    spec: &MergeScanlinesSpec,
    callbacks: &dyn Callbacks,
) {
    if ops.is_empty() || spec.tolerance <= 0.0 {
        return;
    }

    ops.preload_state();

    let units = collect_units(ops);
    if units.is_empty() {
        return;
    }

    let total = units.len();
    let mut new_ops = Ops::new();
    let mut pos = 0usize;
    let mut cancelled = false;

    for (i, unit) in units.iter().enumerate() {
        if callbacks.is_cancelled() {
            cancelled = true;
            break;
        }
        callbacks.report_progress(i as f64 / total as f64, "merge_scanlines");

        // Pass through everything before the unit verbatim.
        for j in pos..unit.start {
            new_ops.transfer_command_from(ops, j);
        }
        pos = unit.end;

        if try_emit_unit(ops, unit, spec, &mut new_ops) {
            // Restructured; nothing else to do for this unit.
            continue;
        }
        // No merging paid off — pass the unit through unchanged.
        for j in unit.start..unit.end {
            new_ops.transfer_command_from(ops, j);
        }
    }

    if cancelled {
        return;
    }
    for j in pos..ops.len() {
        new_ops.transfer_command_from(ops, j);
    }
    ops.replace_with(&new_ops);
}

// ---------------------------------------------------------------------------
// Unit collection
// ---------------------------------------------------------------------------

/// Split the ops into processing units.
///
/// A unit is a maximal run of commands whose moving paths are all
/// straight `MoveTo` + single-cut lines, bounded by markers that must
/// not be restructured (job/layer/state-block markers, dwells, custom
/// commands) or by section markers with a different signature.
/// Per-workpiece markers and repeated section wrappers with the same
/// signature are interior to a unit — that is what lets an array's
/// scanlines merge into shared rows.
fn collect_units(ops: &Ops) -> Vec<Unit> {
    let n = ops.len();
    let mut units: Vec<Unit> = Vec::new();
    let mut i = 0usize;

    while i < n {
        let sig = current_signature(ops, i);
        let (end, candidates, pure) = scan_unit(ops, i, sig);
        if pure && candidates.len() >= 2 {
            // Absorb leading workpiece markers so the unit's first
            // marker is available as the start bookend.
            let mut start = i;
            while start > 0
                && matches!(
                    &ops.commands[start - 1].category,
                    OpCategory::Marker(MarkerCmd::WorkpieceStart(_))
                )
            {
                start -= 1;
            }
            units.push(Unit {
                start,
                end,
                signature: sig,
                candidates,
            });
        }
        i = end.max(i + 1);
    }

    units
}

/// Section signature in effect at command `i`. When `i` is itself a
/// section start, the signature it opens (so the marker is consumed
/// as part of the unit it opens rather than passed through
/// separately).
fn current_signature(ops: &Ops, i: usize) -> UnitSignature {
    if let OpCategory::Marker(MarkerCmd::OpsSectionStart {
        section_type,
        raster_mode,
        ..
    }) = &ops.commands[i].category
    {
        return UnitSignature {
            section_type: Some(*section_type),
            raster_mode: *raster_mode,
        };
    }
    for j in (0..i).rev() {
        match &ops.commands[j].category {
            OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                raster_mode,
                ..
            }) => {
                return UnitSignature {
                    section_type: Some(*section_type),
                    raster_mode: *raster_mode,
                };
            }
            OpCategory::Marker(MarkerCmd::OpsSectionEnd { .. }) => {
                return UnitSignature {
                    section_type: None,
                    raster_mode: None,
                };
            }
            _ => {}
        }
    }
    UnitSignature {
        section_type: None,
        raster_mode: None,
    }
}

/// Scan a prospective unit starting at `i`; returns its end index,
/// its candidates, and whether it is "pure" (every moving path is a
/// mergeable single straight line).
fn scan_unit(
    ops: &Ops,
    start: usize,
    signature: UnitSignature,
) -> (usize, Vec<Candidate>, bool) {
    let n = ops.len();
    let mut candidates = Vec::new();
    let mut pure = true;

    // Path scanning state: the MoveTo index and the moving commands
    // seen since it.
    let mut move_idx: Option<usize> = None;
    let mut moving: Vec<usize> = Vec::new();

    macro_rules! flush {
        () => {
            if let Some(mi) = move_idx.take() {
                if moving.len() == 1 {
                    if let Some(c) = make_candidate(ops, mi, moving[0]) {
                        candidates.push(c);
                    }
                } else if !moving.is_empty() {
                    // Polylines and curves are not mergeable lines.
                    pure = false;
                }
            }
            moving.clear();
        };
    }

    let mut i = start;
    while i < n {
        match &ops.commands[i].category {
            OpCategory::Marker(marker) => match marker {
                // Per-workpiece markers only end the current path;
                // the unit continues across them.
                MarkerCmd::WorkpieceStart(_) | MarkerCmd::WorkpieceEnd(_) => {
                    flush!();
                }
                MarkerCmd::OpsSectionStart {
                    section_type,
                    raster_mode,
                    ..
                } => {
                    let sig = UnitSignature {
                        section_type: Some(*section_type),
                        raster_mode: *raster_mode,
                    };
                    if sig != signature {
                        flush!();
                        return (i, candidates, pure);
                    }
                    flush!();
                }
                MarkerCmd::OpsSectionEnd {
                    section_type,
                    raster_mode,
                    ..
                } => {
                    let sig = UnitSignature {
                        section_type: Some(*section_type),
                        raster_mode: *raster_mode,
                    };
                    if sig != signature {
                        flush!();
                        return (i, candidates, pure);
                    }
                    flush!();
                }
                // Job/layer/state-block markers are hard barriers.
                _ => {
                    flush!();
                    return (i, candidates, pure);
                }
            },
            OpCategory::State(state) => match state {
                StateCmd::Dwell(_) | StateCmd::Custom(_) => {
                    flush!();
                    return (i, candidates, pure);
                }
                // Other state commands are interior; paths continue.
                _ => {}
            },
            OpCategory::Moving { cmd, .. } => match cmd {
                MoveCmd::MoveTo => {
                    flush!();
                    move_idx = Some(i);
                }
                MoveCmd::LineTo | MoveCmd::ScanLine { .. } => {
                    if move_idx.is_none() {
                        pure = false;
                    } else {
                        moving.push(i);
                    }
                }
                // Arcs and beziers make the path unmergeable.
                _ => {
                    pure = false;
                    if move_idx.is_some() {
                        moving.push(i);
                    }
                }
            },
        }
        i += 1;
    }
    flush!();
    (n, candidates, pure)
}

/// Build a candidate from a `MoveTo` at `move_idx` and its single
/// straight cut command at `cut_idx`, or `None` when the pair does
/// not cut anything.
fn make_candidate(
    ops: &Ops,
    move_idx: usize,
    cut_idx: usize,
) -> Option<Candidate> {
    let start = ops.endpoint(move_idx);
    let end = ops.endpoint(cut_idx);
    let len = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
    if len < 1e-9 {
        return None;
    }

    let (is_scanline, power_values, has_power) =
        match &ops.commands[cut_idx].category {
            OpCategory::Moving {
                cmd: MoveCmd::ScanLine { power_values },
                ..
            } => (
                true,
                Arc::clone(power_values),
                power_values.iter().any(|&b| b != 0),
            ),
            OpCategory::Moving {
                cmd: MoveCmd::LineTo,
                ..
            } => {
                let power = ops.state(cut_idx).map(|s| s.power).unwrap_or(0.0);
                (false, Arc::new(Vec::new()), power > 0.0)
            }
            _ => return None,
        };
    if !has_power {
        return None;
    }

    let state = ops.state(cut_idx).cloned().unwrap_or_default();

    // Canonical direction: the traversal angle normalized into
    // [0, π) so opposite traversal directions compare equal.
    let theta = (end.y - start.y)
        .atan2(end.x - start.x)
        .rem_euclid(std::f64::consts::PI);
    let dir = (theta.cos(), theta.sin());
    let perp = dir.0 * start.y - dir.1 * start.x;
    let proj_start = dir.0 * start.x + dir.1 * start.y;
    let proj_end = dir.0 * end.x + dir.1 * end.y;

    let (lo, hi, padded_lo, padded_hi, aligned) = if proj_start <= proj_end {
        (start, end, proj_start, proj_end, true)
    } else {
        (end, start, proj_end, proj_start, false)
    };

    // Content extent: the samples with non-zero power. Overscan pads
    // the ends with zeros; the content drives row assembly while the
    // pads are re-emitted around the merged run.
    let (proj_lo, proj_hi, pad_lo, pad_hi) = if is_scanline {
        let n = power_values.len();
        let canon = |j: usize| {
            if aligned {
                power_values[j]
            } else {
                power_values[n - 1 - j]
            }
        };
        let nz = (0..n).filter(|&j| canon(j) != 0);
        let first = nz.clone().min().unwrap_or(n);
        let last = nz.max().unwrap_or(0);
        if first > last {
            return None;
        }
        let step = (padded_hi - padded_lo) / n as f64;
        (
            padded_lo + first as f64 * step,
            padded_lo + (last + 1) as f64 * step,
            first,
            n - 1 - last,
        )
    } else {
        (padded_lo, padded_hi, 0, 0)
    };

    Some(Candidate {
        lo,
        hi,
        is_scanline,
        power_values,
        state,
        proj_lo,
        proj_hi,
        pad_lo,
        pad_hi,
        aligned,
        perp,
        angle_bucket: (theta / ANGLE_BUCKET_RAD).floor() as i64,
    })
}

// ---------------------------------------------------------------------------
// Row bucketing (union-find over occupied hash buckets)
// ---------------------------------------------------------------------------

type CellKey = (i64, i64);

/// Group candidate indices (into `unit.candidates`) into rows.
///
/// Candidates first group by (angle bucket, Z bucket); occupied
/// angle buckets within one bucket of each other are unioned so a
/// row whose members scatter across the angle quantization still
/// groups, without any pairwise geometry tests.
///
/// Within such a group, rows are found by sorting on the
/// perpendicular offset and clustering with a threshold derived from
/// the data: the tolerance, capped at half the smallest gap between
/// distinct offsets. A fixed tolerance alone cannot work for rasters
/// whose line interval is finer than the tolerance — the entire band
/// would chain into one "row" and be dropped as overlapping.
fn bucket_rows(unit: &Unit, tolerance: f64) -> Vec<Vec<usize>> {
    let angle_buckets = (std::f64::consts::PI / ANGLE_BUCKET_RAD).ceil() as i64;

    let mut cells: HashMap<CellKey, Vec<usize>> = HashMap::new();
    for (idx, c) in unit.candidates.iter().enumerate() {
        let z_bucket = (c.lo.z / Z_QUANTUM).round() as i64;
        let key = (c.angle_bucket.rem_euclid(angle_buckets), z_bucket);
        cells.entry(key).or_default().push(idx);
    }

    let mut parent: HashMap<CellKey, CellKey> = HashMap::new();
    for key in cells.keys() {
        parent.insert(*key, *key);
    }

    fn find(parent: &mut HashMap<CellKey, CellKey>, k: CellKey) -> CellKey {
        let mut root = k;
        while parent[&root] != root {
            root = parent[&root];
        }
        let mut cur = k;
        while parent[&cur] != root {
            let next = parent[&cur];
            parent.insert(cur, root);
            cur = next;
        }
        root
    }

    let occupied: Vec<CellKey> = cells.keys().copied().collect();
    for key in occupied {
        for da in -1i64..=1 {
            let neighbor = ((key.0 + da).rem_euclid(angle_buckets), key.1);
            if parent.contains_key(&neighbor) {
                let ra = find(&mut parent, key);
                let rb = find(&mut parent, neighbor);
                if ra != rb {
                    parent.insert(ra, rb);
                }
            }
        }
    }

    let mut groups: HashMap<CellKey, Vec<usize>> = HashMap::new();
    for (key, members) in &cells {
        let root = find(&mut parent, *key);
        groups
            .entry(root)
            .or_default()
            .extend(members.iter().copied());
    }

    let mut result: Vec<Vec<usize>> = Vec::new();
    for mut members in groups.into_values() {
        if members.len() < 2 {
            continue;
        }

        // Cluster on the perpendicular offset: sort, then split
        // wherever the gap to the previous offset exceeds the
        // threshold. The threshold adapts to the row spacing so a
        // line interval finer than the tolerance still yields
        // distinct rows.
        members.sort_by(|&a, &b| {
            perp_of(unit, a)
                .partial_cmp(&perp_of(unit, b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut min_spacing = f64::INFINITY;
        for w in members.windows(2) {
            let d = perp_of(unit, w[1]) - perp_of(unit, w[0]);
            if d > PERP_NOISE_FLOOR {
                min_spacing = min_spacing.min(d);
            }
        }
        let threshold = if min_spacing.is_finite() {
            tolerance.min(min_spacing * 0.5)
        } else {
            tolerance
        };

        let mut rows: Vec<Vec<usize>> = Vec::new();
        let mut current: Vec<usize> = vec![members[0]];
        for w in members.windows(2) {
            if perp_of(unit, w[1]) - perp_of(unit, w[0]) > threshold {
                rows.push(std::mem::take(&mut current));
            }
            current.push(w[1]);
        }
        rows.push(current);

        for mut row in rows {
            if row.len() < 2 {
                continue;
            }
            row.sort_by(|&a, &b| {
                unit.candidates[a]
                    .proj_lo
                    .partial_cmp(&unit.candidates[b].proj_lo)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Drop rows whose projections overlap.
            let mut overlap = false;
            for w in row.windows(2) {
                let prev = &unit.candidates[w[0]];
                let next = &unit.candidates[w[1]];
                if next.proj_lo < prev.proj_hi - tolerance {
                    overlap = true;
                    break;
                }
            }
            if !overlap {
                result.push(row);
            }
        }
    }
    result
}

fn perp_of(unit: &Unit, idx: usize) -> f64 {
    unit.candidates[idx].perp
}

// ---------------------------------------------------------------------------
// Unit emission
// ---------------------------------------------------------------------------

/// Try to merge and re-emit `unit` into `new_ops`.
///
/// Returns `true` when at least one run bridged a gap *and* the
/// restructured plan is strictly cheaper than passing the unit
/// through, `false` when the unit should be emitted verbatim.
fn try_emit_unit(
    ops: &Ops,
    unit: &Unit,
    spec: &MergeScanlinesSpec,
    new_ops: &mut Ops,
) -> bool {
    let rows = bucket_rows(unit, spec.tolerance);

    let mut merged_rows: Vec<(Vec<Run>, usize)> = Vec::new();
    let mut any_merged = false;

    for members in rows {
        let first_appearance =
            members.iter().copied().min().unwrap_or(usize::MAX);
        let mut runs: Vec<Run> = Vec::new();
        let mut current = Run {
            segments: vec![members[0]],
            proj_lo: unit.candidates[members[0]].proj_lo,
            proj_hi: unit.candidates[members[0]].proj_hi,
        };

        for w in members.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (ca, cb) = (&unit.candidates[a], &unit.candidates[b]);
            let gap = (cb.proj_lo - ca.proj_hi).max(0.0);
            let run_span = current.proj_hi - current.proj_lo;

            if gap_allows_bridge(ca, cb, gap, run_span, spec) {
                current.segments.push(b);
                current.proj_hi = cb.proj_hi;
                any_merged = true;
            } else {
                runs.push(Run {
                    segments: std::mem::take(&mut current.segments),
                    proj_lo: current.proj_lo,
                    proj_hi: current.proj_hi,
                });
                current = Run {
                    segments: vec![b],
                    proj_lo: cb.proj_lo,
                    proj_hi: cb.proj_hi,
                };
            }
        }
        runs.push(current);

        merged_rows.push((runs, first_appearance));
    }

    if !any_merged {
        return false;
    }

    // Emit rows in first-appearance order so the original ordering is
    // disturbed as little as possible.
    merged_rows.sort_by_key(|(_, appearance)| *appearance);

    // Restructure only when it strictly beats the original plan
    // under the shared motion model: partial merging can lose time
    // when unbridged gaps end up crossed once per row instead of
    // once per workpiece block. A manual max-gap ceiling overrides
    // the guard — the user asked for these bridges explicitly.
    if spec.max_gap_mm <= 0.0
        && restructured_unit_cost(unit, &merged_rows, spec)
            >= passthrough_unit_cost(ops, unit, spec)
    {
        return false;
    }

    // Bookend markers: keep the outermost workpiece markers and one
    // section wrapper so encoders and progress consumers see a
    // well-formed region. Interior markers cannot be represented
    // once rows interleave workpieces and are dropped.
    let bookends = UnitBookends::collect(ops, unit);
    bookends.emit_start(unit, new_ops);

    let mut sync = StateSync::new();
    let mut pos: Option<Point3D> = None;

    for (runs, _) in &merged_rows {
        emit_row(unit, runs, new_ops, &mut sync, &mut pos);
    }

    bookends.emit_end(unit, new_ops);
    true
}

/// Simulated cost of the restructured plan, using the same motion
/// model as the time estimator: each run is one scan sweep, travels
/// between runs are rapids. The approach into the unit's first run
/// is excluded so both plans start on equal footing.
fn restructured_unit_cost(
    unit: &Unit,
    merged_rows: &[(Vec<Run>, usize)],
    spec: &MergeScanlinesSpec,
) -> f64 {
    let mut cost = 0.0;
    let mut pos: Option<Point3D> = None;
    for (runs, _) in merged_rows {
        for run in runs {
            let segments = &run.segments;
            let first = &unit.candidates[segments[0]];
            let last = &unit.candidates[segments[segments.len() - 1]];
            let reversed = starts_from_hi(first.lo, last.hi, pos);
            let entry = if reversed { last.hi } else { first.lo };

            if let Some(p) = pos {
                let d = dist3(p, entry);
                cost += move_duration(
                    d,
                    first.rapid_rate(spec.rapid_speed),
                    spec.acceleration,
                );
            }
            let span = dist3(first.lo, last.hi);
            cost += move_duration(
                span,
                first.feed_rate(spec.cut_speed),
                spec.acceleration,
            );
            pos = Some(if reversed { first.lo } else { last.hi });
        }
    }
    cost
}

/// True when a run should be emitted from its high end: the same
/// nearest-end rule `emit_row` applies, kept in one place so the
/// cost simulation cannot diverge from the emission.
fn starts_from_hi(lo: Point3D, hi: Point3D, pos: Option<Point3D>) -> bool {
    match pos {
        Some(p) => {
            (hi.x - p.x).hypot(hi.y - p.y) < (lo.x - p.x).hypot(lo.y - p.y)
        }
        None => false,
    }
}

/// Simulated cost of passing the unit through unchanged, mirroring
/// the time estimator's per-command model (every move starts and
/// ends at rest; MoveTo runs at the rapid rate, cuts at the feed
/// rate). The approach into the unit's first move is excluded.
fn passthrough_unit_cost(
    ops: &Ops,
    unit: &Unit,
    spec: &MergeScanlinesSpec,
) -> f64 {
    let mut cost = 0.0;
    let mut last: Option<Point3D> = None;
    let mut feed = spec.cut_speed;
    let mut rapid = spec.rapid_speed;
    for j in unit.start..unit.end {
        match &ops.commands[j].category {
            OpCategory::State(StateCmd::SetFeedRate(s)) => {
                feed = *s as f64;
            }
            OpCategory::State(StateCmd::SetRapidRate(s)) => {
                rapid = *s as f64;
            }
            OpCategory::Moving { end, cmd } => {
                let distance = last.map_or(0.0, |l| dist3(l, *end));
                let speed = if matches!(cmd, MoveCmd::MoveTo) {
                    rapid
                } else {
                    feed
                };
                if distance > 0.0 {
                    cost += move_duration(distance, speed, spec.acceleration);
                }
                last = Some(*end);
            }
            _ => {}
        }
    }
    cost
}

fn dist3(a: Point3D, b: Point3D) -> f64 {
    (b.x - a.x).hypot(b.y - a.y).hypot(b.z - a.z)
}

/// Decide whether the gap between two adjacent row segments should
/// be bridged, given the accumulated span of the run being extended.
fn gap_allows_bridge(
    a: &Candidate,
    b: &Candidate,
    gap: f64,
    run_span: f64,
    spec: &MergeScanlinesSpec,
) -> bool {
    if spec.max_gap_mm > 0.0 && gap > spec.max_gap_mm {
        return false;
    }
    // Different speeds or heads must not be bridged over.
    if a.state.feed_rate != b.state.feed_rate {
        return false;
    }
    if a.state.active_head_uid != b.state.active_head_uid {
        return false;
    }
    if a.is_scanline != b.is_scanline {
        return false;
    }
    if a.is_scanline {
        let (da, db) = (a.density(), b.density());
        if da <= 0.0 || db <= 0.0 {
            return false;
        }
        if (da - db).abs() > DENSITY_TOLERANCE * da.max(db) {
            return false;
        }
    } else if a.state.power != b.state.power {
        // Constant-power lines with different powers could still be
        // bridged with a state switch, but keep the cost model honest
        // and the output simple: only equal-power lines merge.
        return false;
    }

    if spec.max_gap_mm > 0.0 {
        // A manual ceiling is an explicit user decision to bridge
        // every gap within it; the cost model does not get a veto.
        return true;
    }

    if spec.acceleration <= 0.0 {
        // Automatic mode has no cost model to justify a bridge.
        return false;
    }

    let v_scan = a.feed_rate(spec.cut_speed);
    if v_scan <= 0.0 {
        return false;
    }

    // Extend-vs-close for the run being extended. The alternative to
    // bridging is not a rapid dash across the gap: the unmerged plan
    // engraves each workpiece block completely and crosses between
    // blocks once per band, so the honest comparison is between the
    // merged sweep and the two sweeps standing alone, both at scan
    // speed. In the cruise regime this reduces to a pure gap
    // threshold — bridge iff g < v_scan²/a — the exact regime where
    // stop/start cycles cost more than sweeping the gap. A unit-level
    // cost guard (see `restructured_unit_cost`) rejects the whole
    // restructuring when partial merging would still lose.
    let separate = move_duration(run_span, v_scan, spec.acceleration)
        + move_duration(b.content_len(), v_scan, spec.acceleration);
    let merged = move_duration(
        run_span + gap + b.content_len(),
        v_scan,
        spec.acceleration,
    );
    merged < separate
}

/// The outermost markers of a restructured unit.
struct UnitBookends {
    first_wp_start: Option<Arc<str>>,
    last_wp_end: Option<Arc<str>>,
    section_uid: Option<Arc<str>>,
}

impl UnitBookends {
    fn collect(ops: &Ops, unit: &Unit) -> Self {
        let mut bookends = UnitBookends {
            first_wp_start: None,
            last_wp_end: None,
            section_uid: None,
        };
        for j in unit.start..unit.end {
            match &ops.commands[j].category {
                OpCategory::Marker(MarkerCmd::WorkpieceStart(uid)) => {
                    if bookends.first_wp_start.is_none() {
                        bookends.first_wp_start = Some(Arc::clone(uid));
                    }
                }
                OpCategory::Marker(MarkerCmd::WorkpieceEnd(uid)) => {
                    bookends.last_wp_end = Some(Arc::clone(uid));
                }
                OpCategory::Marker(MarkerCmd::OpsSectionStart {
                    workpiece_uid,
                    ..
                }) if bookends.section_uid.is_none() => {
                    bookends.section_uid = workpiece_uid.clone();
                }
                _ => {}
            }
        }
        bookends
    }

    fn emit_start(&self, unit: &Unit, new_ops: &mut Ops) {
        if let Some(uid) = &self.first_wp_start {
            new_ops.workpiece_start(uid);
        }
        if let Some(section_type) = unit.signature.section_type {
            let _ = new_ops.ops_section_start(
                section_type,
                self.section_uid.as_deref().unwrap_or(""),
                unit.signature.raster_mode,
            );
        }
    }

    fn emit_end(&self, unit: &Unit, new_ops: &mut Ops) {
        if let Some(section_type) = unit.signature.section_type {
            let _ = new_ops
                .ops_section_end(section_type, unit.signature.raster_mode);
        }
        if let Some(uid) = &self.last_wp_end {
            new_ops.workpiece_end(uid);
        }
    }
}

/// Emit one row's runs as merged scanlines (or bridged line
/// segments), choosing each run's start nearest the current head
/// position so a serpentine pattern emerges naturally.
fn emit_row(
    unit: &Unit,
    runs: &[Run],
    new_ops: &mut Ops,
    sync: &mut StateSync,
    pos: &mut Option<Point3D>,
) {
    for run in runs {
        let segments = &run.segments;
        let first = &unit.candidates[segments[0]];
        let last = &unit.candidates[segments[segments.len() - 1]];

        // Direction: start the run at the end nearest the head (the
        // same rule the cost simulation applies).
        let reversed = starts_from_hi(first.lo, last.hi, *pos);

        if first.is_scanline {
            emit_scanline_run(unit, segments, reversed, new_ops, sync);
        } else {
            emit_line_run(unit, segments, reversed, new_ops, sync);
        }
        *pos = Some(if reversed { first.lo } else { last.hi });
    }
}

fn emit_scanline_run(
    unit: &Unit,
    segments: &[usize],
    reversed: bool,
    new_ops: &mut Ops,
    sync: &mut StateSync,
) {
    let first = &unit.candidates[segments[0]];
    let last = &unit.candidates[segments[segments.len() - 1]];

    sync.sync(new_ops, &first.state, true);

    // Build the power profile in canonical (lo -> hi) order: the
    // first segment's leading overscan pad, each segment's content
    // with zero pads across the bridged gaps (interior pads are
    // trimmed — the head passes through at constant speed), and the
    // last segment's trailing pad.
    let mut power: Vec<u8> = Vec::new();
    power.extend(std::iter::repeat_n(0u8, first.pad_lo));
    for (i, &c_idx) in segments.iter().enumerate() {
        let c = &unit.candidates[c_idx];
        if i > 0 {
            let prev = &unit.candidates[segments[i - 1]];
            let gap = (c.proj_lo - prev.proj_hi).max(0.0);
            let pad = (gap * c.density()).round() as usize;
            power.extend(std::iter::repeat_n(0u8, pad));
        }
        power.extend(c.content_samples().map(|j| c.sample(j)));
    }
    power.extend(std::iter::repeat_n(0u8, last.pad_hi));

    if reversed {
        power.reverse();
        new_ops.move_to(last.hi.x, last.hi.y, last.hi.z, None);
        new_ops.scan_to(first.lo.x, first.lo.y, first.lo.z, power, None);
    } else {
        new_ops.move_to(first.lo.x, first.lo.y, first.lo.z, None);
        new_ops.scan_to(last.hi.x, last.hi.y, last.hi.z, power, None);
    }
}

fn emit_line_run(
    unit: &Unit,
    segments: &[usize],
    reversed: bool,
    new_ops: &mut Ops,
    sync: &mut StateSync,
) {
    let order: Vec<usize> = if reversed {
        segments.iter().rev().copied().collect()
    } else {
        segments.to_vec()
    };

    for (i, &c_idx) in order.iter().enumerate() {
        let c = &unit.candidates[c_idx];
        let (entry, exit) = if reversed { (c.hi, c.lo) } else { (c.lo, c.hi) };

        if i == 0 {
            sync.sync(new_ops, &c.state, false);
            new_ops.move_to(entry.x, entry.y, entry.z, None);
        } else {
            // Bridge the gap at zero power.
            new_ops.set_power(0.0);
            new_ops.line_to(entry.x, entry.y, entry.z, None);
            sync.sync(new_ops, &c.state, false);
        }
        new_ops.line_to(exit.x, exit.y, exit.z, None);
    }
}

/// Emits only the state commands that differ from what the machine
/// already has, so restructured units don't bloat the output.
///
/// The baseline starts from `State::default()`: the unit's original
/// leading state commands are consumed by the restructuring, so the
/// first sync must re-establish everything the first segment needs.
struct StateSync {
    prev: State,
}

impl StateSync {
    fn new() -> Self {
        StateSync {
            prev: State::default(),
        }
    }

    fn sync(&mut self, new_ops: &mut Ops, state: &State, scanline: bool) {
        // Scanline power values are absolute (they replace, not
        // scale, the state power), so power is only synced for
        // constant-power line segments.
        if !scanline && (state.power - self.prev.power).abs() > f64::EPSILON {
            new_ops.set_power(state.power);
            self.prev.power = state.power;
        }
        if let Some(f) = state.feed_rate {
            if self.prev.feed_rate != Some(f) {
                new_ops.set_feed_rate(f);
                self.prev.feed_rate = Some(f);
            }
        }
        if let Some(f) = state.rapid_rate {
            if self.prev.rapid_rate != Some(f) {
                new_ops.set_rapid_rate(f);
                self.prev.rapid_rate = Some(f);
            }
        }
        if let Some(f) = state.frequency {
            if self.prev.frequency != Some(f) {
                new_ops.set_frequency(f);
                self.prev.frequency = Some(f);
            }
        }
        if let Some(p) = state.pulse_width {
            if self.prev.pulse_width != Some(p) {
                new_ops.set_pulse_width(p);
                self.prev.pulse_width = Some(p);
            }
        }
        if state.coolant != self.prev.coolant {
            if let Some(mode) = state.coolant {
                new_ops.set_coolant(mode);
            }
            self.prev.coolant = state.coolant;
        }
        if state.air_assist != self.prev.air_assist {
            if let Some(mode) = state.air_assist {
                new_ops.set_air_assist(mode);
            }
            self.prev.air_assist = state.air_assist;
        }
        if state.head_coolant != self.prev.head_coolant {
            if let Some(mode) = state.head_coolant {
                new_ops.set_head_coolant(mode);
            }
            self.prev.head_coolant = state.head_coolant;
        }
        if state.power_mode != self.prev.power_mode {
            if let Some(mode) = state.power_mode {
                new_ops.set_power_mode(mode);
            }
            self.prev.power_mode = state.power_mode;
        }
        if let Some(h) = &state.active_head_uid {
            if self.prev.active_head_uid.as_deref() != Some(h.as_str()) {
                new_ops.set_head(h);
                self.prev.active_head_uid = Some(h.clone());
            }
        }
    }
}
