//! Cleanup: Deduplication and gap closing for geometry data.
//!
//! Provides functions for cleaning geometry command arrays by removing
//! duplicate segments and closing small gaps between connected paths,
//! plus contour-level clean-up operations (closing, joining, de-duplicating
//! and splitting contours).

use std::collections::{HashMap, HashSet};

use glam::{DVec3, DVec4};

use crate::geo::algo::topology::{reverse_contour, split_into_contours};
use crate::geo::geometry::Geometry;
use crate::geo::query::find_closest_point_on_path_from_array;
use crate::geo::types::{Command, Point, Point3D, Rect};

/// Extract a hashable key for a segment. Returns None for MOVE commands.
pub fn get_segment_key(cmd: &Command) -> Option<(u32, DVec3, DVec4)> {
    match cmd {
        Command::Move { .. } => None,
        Command::Line { end } => {
            Some((2, DVec3::new(end.x, end.y, end.z), DVec4::ZERO))
        }
        Command::Arc {
            end,
            center_offset,
            normal,
        } => {
            let clockwise = normal.z < 0.0;
            Some((
                3,
                DVec3::new(end.x, end.y, end.z),
                DVec4::new(
                    center_offset.x,
                    center_offset.y,
                    if clockwise { 1.0 } else { 0.0 },
                    0.0,
                ),
            ))
        }
        Command::Bezier {
            end,
            control1,
            control2,
        } => Some((
            4,
            DVec3::new(end.x, end.y, end.z),
            DVec4::new(control1.x, control1.y, control2.x, control2.y),
        )),
    }
}

/// Check if two segment keys represent identical segments within tolerance.
pub fn are_segments_equal(
    k1: &(u32, DVec3, DVec4),
    k2: &(u32, DVec3, DVec4),
    tolerance: f64,
) -> bool {
    if k1.0 != k2.0 {
        return false;
    }
    if k1.1.distance(k2.1) > tolerance {
        return false;
    }
    if k1.0 == 2 {
        return true;
    }
    if k1.0 == 3 {
        return DVec3::new(k1.2.x, k1.2.y, 0.0)
            .distance(DVec3::new(k2.2.x, k2.2.y, 0.0))
            <= tolerance
            && (k1.2.z - k2.2.z).abs() < tolerance;
    }
    if k1.0 == 4 {
        return DVec3::new(k1.2.x, k1.2.y, k1.2.z)
            .distance(DVec3::new(k2.2.x, k2.2.y, k2.2.z))
            <= tolerance
            && (k1.2.w - k2.2.w).abs() < tolerance;
    }
    false
}

/// Remove duplicate segments from geometry command data.
pub fn remove_duplicate_segments(
    data: &[Command],
    tolerance: f64,
) -> Vec<Command> {
    if data.is_empty() {
        return data.to_vec();
    }

    let mut result: Vec<Command> = Vec::new();
    let mut seen_segments: Vec<(u32, DVec3, DVec4)> = Vec::new();

    for cmd in data {
        if matches!(cmd, Command::Move { .. }) {
            seen_segments.clear();
            result.push(cmd.clone());
            continue;
        }

        if let Some(key) = get_segment_key(cmd) {
            let is_dup = seen_segments
                .iter()
                .any(|sk| are_segments_equal(&key, sk, tolerance));
            if is_dup {
                continue;
            }
            seen_segments.push(key);
        }
        result.push(cmd.clone());
    }

    result
}

/// Close small gaps in a geometry data array to form clean, connected paths.
pub fn close_geometry_gaps_from_array(
    data: &[Command],
    tolerance: f64,
) -> Vec<Command> {
    if data.len() < 2 {
        return data.to_vec();
    }

    let tol_sq = tolerance * tolerance;

    let mut move_indices: Vec<usize> = Vec::new();
    for (i, cmd) in data.iter().enumerate() {
        if matches!(cmd, Command::Move { .. }) {
            move_indices.push(i);
        }
    }

    let mut modified: Vec<Command> = data.to_vec();

    let sub_ranges: Vec<(usize, usize)> = if move_indices.is_empty() {
        vec![(0, data.len())]
    } else {
        let mut ranges = Vec::new();
        let mut prev = 0;
        for &mi in &move_indices[1..] {
            ranges.push((prev, mi));
            prev = mi;
        }
        ranges.push((prev, data.len()));
        ranges
    };

    for &(start, end) in &sub_ranges {
        if end - start >= 2 {
            let s = modified[start].end_point();
            let e_cmd = &modified[end - 1];
            let e = e_cmd.end_point();
            let dsq = s.distance_squared(e);
            if dsq < tol_sq {
                let new_cmd = match e_cmd {
                    Command::Move { .. } => Command::Move { end: s },
                    Command::Line { .. } => Command::Line { end: s },
                    Command::Arc {
                        center_offset,
                        normal,
                        ..
                    } => Command::Arc {
                        end: s,
                        center_offset: *center_offset,
                        normal: *normal,
                    },
                    Command::Bezier {
                        control1, control2, ..
                    } => Command::Bezier {
                        end: s,
                        control1: *control1,
                        control2: *control2,
                    },
                };
                modified[end - 1] = new_cmd;
            }
        }
    }

    let mut final_rows: Vec<Command> = Vec::new();
    let mut last_end: Option<Point3D> = None;

    for cmd in &modified {
        let end_pt = cmd.end_point();

        if matches!(cmd, Command::Move { .. }) {
            if let Some(prev) = last_end {
                let dsq = end_pt.distance_squared(prev);
                if dsq < tol_sq {
                    final_rows.push(Command::Line { end: prev });
                } else {
                    final_rows.push(cmd.clone());
                }
            } else {
                final_rows.push(cmd.clone());
            }
        } else {
            final_rows.push(cmd.clone());
        }
        last_end = Some(end_pt);
    }

    final_rows
}

/// The largest gap that is still considered "closed" on its own.
const CLOSED_EPS: f64 = 1e-6;
/// Samples taken along a contour when comparing it to a duplicate.
const DUPLICATE_SAMPLES: usize = 32;

/// The 2D start point of a contour (the end of its first command).
fn start_point(contour: &Geometry) -> Point {
    let p = contour.data.first().map(|c| c.end_point());
    match p {
        Some(p) => Point::new(p.x, p.y),
        None => Point::ZERO,
    }
}

/// The 2D end point of a contour (the end of its last command).
fn end_point(contour: &Geometry) -> Point {
    let p = contour.data.last().map(|c| c.end_point());
    match p {
        Some(p) => Point::new(p.x, p.y),
        None => Point::ZERO,
    }
}

/// The number of drawing (non-Move) commands in a contour.
fn segment_count(contour: &Geometry) -> usize {
    contour
        .data
        .iter()
        .filter(|c| !matches!(c, Command::Move { .. }))
        .count()
}

/// Splits into contours, dropping those that draw nothing.
fn drawn_contours(geo: &Geometry) -> Vec<Geometry> {
    if geo.is_empty() {
        return vec![];
    }
    split_into_contours(geo)
        .into_iter()
        .filter(|c| segment_count(c) > 0)
        .collect()
}

/// True when a contour's start and end point do not coincide.
fn is_open(contour: &Geometry) -> bool {
    !contour.is_closed(CLOSED_EPS)
}

/// Closes an open contour of at least two segments in place when its
/// start and end are within the tolerance. Returns true if it closed it.
fn close_if_near(contour: &mut Geometry, tolerance: f64) -> bool {
    if !is_open(contour) || segment_count(contour) < 2 {
        return false;
    }
    let start = contour
        .data
        .first()
        .map(|c| c.end_point())
        .unwrap_or(Point3D::ZERO);
    if start.truncate().distance(end_point(contour)) > tolerance {
        return false;
    }
    contour.line_to(start.x, start.y, start.z);
    true
}

/// Concatenates contours into a single geometry.
fn concat(contours: &[Geometry]) -> Geometry {
    let mut result = Geometry::new();
    for contour in contours {
        result.extend(contour);
    }
    result
}

/// Closes every open contour whose start and end point lie within the
/// tolerance of each other. Existing points are never moved: the gap is
/// bridged with a straight segment between two existing end points.
///
/// Returns the new geometry and the number of closed contours.
pub fn close_open_contours(
    geo: &Geometry,
    tolerance: f64,
) -> (Geometry, usize) {
    let mut contours = drawn_contours(geo);
    let mut closed = 0;
    for contour in &mut contours {
        if close_if_near(contour, tolerance) {
            closed += 1;
        }
    }
    (concat(&contours), closed)
}

/// A spatial hash of open contour end points for neighbour lookups.
struct EndpointGrid {
    cell: f64,
    cells: HashMap<(i64, i64), HashSet<(usize, usize)>>,
    points: HashMap<(usize, usize), Point>,
}

impl EndpointGrid {
    fn new(cell_size: f64) -> Self {
        EndpointGrid {
            cell: cell_size.max(1e-9),
            cells: HashMap::new(),
            points: HashMap::new(),
        }
    }

    fn key(&self, p: Point) -> (i64, i64) {
        (
            (p.x / self.cell).floor() as i64,
            (p.y / self.cell).floor() as i64,
        )
    }

    fn add(&mut self, index: usize, end: usize, p: Point) {
        self.points.insert((index, end), p);
        self.cells
            .entry(self.key(p))
            .or_default()
            .insert((index, end));
    }

    fn remove_contour(&mut self, index: usize) {
        for end in [0, 1] {
            if let Some(p) = self.points.remove(&(index, end)) {
                if let Some(cell) = self.cells.get_mut(&self.key(p)) {
                    cell.remove(&(index, end));
                }
            }
        }
    }

    /// The stored entry closest to `p`, within the tolerance.
    fn nearest(&self, p: Point, tolerance: f64) -> Option<(usize, usize)> {
        let (kx, ky) = self.key(p);
        let mut entries: Vec<(usize, usize)> = Vec::new();
        for dx in [-1_i64, 0, 1] {
            for dy in [-1_i64, 0, 1] {
                if let Some(cell) = self.cells.get(&(kx + dx, ky + dy)) {
                    entries.extend(cell.iter().copied());
                }
            }
        }
        entries.sort_unstable();
        let mut best: Option<(usize, usize)> = None;
        let mut best_dist = tolerance;
        for entry in entries {
            if let Some(&q) = self.points.get(&entry) {
                let d = p.distance(q);
                if d <= best_dist {
                    best = Some(entry);
                    best_dist = d;
                }
            }
        }
        best
    }
}

/// Appends a piece to the end of a chain, bridging any gap.
fn append_contour(chain: &mut Geometry, piece: &Geometry) {
    let piece_start = piece
        .data
        .first()
        .map(|c| c.end_point())
        .unwrap_or(Point3D::ZERO);
    if end_point(chain).distance(piece_start.truncate()) > 0.0 {
        chain.line_to(piece_start.x, piece_start.y, piece_start.z);
    }
    for cmd in piece.data.iter().skip(1) {
        chain.data.push(cmd.clone());
    }
}

/// Keeps appending the nearest open contour to the chain's end, and
/// records the appended contours in `consumed`.
fn grow_tail(
    chain: &mut Geometry,
    contours: &[Geometry],
    grid: &mut EndpointGrid,
    tolerance: f64,
    consumed: &mut HashSet<usize>,
) -> usize {
    let mut joins = 0;
    while let Some((index, end)) = grid.nearest(end_point(chain), tolerance) {
        grid.remove_contour(index);
        consumed.insert(index);
        let piece = if end == 1 {
            reverse_contour(&contours[index])
        } else {
            contours[index].copy()
        };
        append_contour(chain, &piece);
        joins += 1;
    }
    joins
}

/// Joins open contours whose end points meet within the tolerance into
/// longer contours, reversing contours where needed. A joined contour
/// whose own ends then meet within the tolerance is closed. Closed
/// contours are left untouched.
///
/// Returns the new geometry and the number of joins made.
pub fn join_open_contours(geo: &Geometry, tolerance: f64) -> (Geometry, usize) {
    let contours = drawn_contours(geo);
    let mut grid = EndpointGrid::new(tolerance);
    let open_indices: Vec<usize> = contours
        .iter()
        .enumerate()
        .filter(|(_, c)| is_open(c))
        .map(|(i, _)| i)
        .collect();
    for &i in &open_indices {
        grid.add(i, 0, start_point(&contours[i]));
        grid.add(i, 1, end_point(&contours[i]));
    }

    let mut chains: HashMap<usize, Geometry> = HashMap::new();
    let mut consumed: HashSet<usize> = HashSet::new();
    let mut joins = 0;
    for &i in &open_indices {
        if consumed.contains(&i) {
            continue;
        }
        grid.remove_contour(i);
        let mut chain = contours[i].copy();
        joins += grow_tail(
            &mut chain,
            &contours,
            &mut grid,
            tolerance,
            &mut consumed,
        );
        chain = reverse_contour(&chain);
        joins += grow_tail(
            &mut chain,
            &contours,
            &mut grid,
            tolerance,
            &mut consumed,
        );
        chain = reverse_contour(&chain);
        close_if_near(&mut chain, tolerance);
        chains.insert(i, chain);
    }

    let mut result = Geometry::new();
    for (i, contour) in contours.iter().enumerate() {
        if let Some(chain) = chains.get(&i) {
            result.extend(chain);
        } else if !consumed.contains(&i) {
            result.extend(contour);
        }
    }
    (result, joins)
}

/// The contour's vertices plus evenly spaced points along it.
fn sample_points(contour: &Geometry) -> Vec<Point> {
    let mut points: Vec<Point> = contour
        .data
        .iter()
        .map(|c| {
            let p = c.end_point();
            Point::new(p.x, p.y)
        })
        .collect();
    let length = contour.distance();
    if length > 0.0 {
        let step = length / DUPLICATE_SAMPLES as f64;
        let distances: Vec<f64> =
            (0..=DUPLICATE_SAMPLES).map(|k| step * k as f64).collect();
        for (_idx, _t, p) in contour.get_positions_at_distances(&distances) {
            points.push(p);
        }
    }
    points
}

/// True when every sample point lies on the target within the tolerance.
fn covers(target: &Geometry, points: &[Point], tolerance: f64) -> bool {
    points.iter().all(|p| {
        match find_closest_point_on_path_from_array(&target.data, p.x, p.y) {
            Some((_idx, _t, hit)) => p.distance(hit) <= tolerance,
            None => false,
        }
    })
}

/// True when two rectangles agree within the tolerance.
fn rects_match(a: &Rect, b: &Rect, tolerance: f64) -> bool {
    (a.min.x - b.min.x).abs() <= tolerance
        && (a.min.y - b.min.y).abs() <= tolerance
        && (a.max.x - b.max.x).abs() <= tolerance
        && (a.max.y - b.max.y).abs() <= tolerance
}

/// True if two contours trace the same path within the tolerance,
/// regardless of their direction or start point.
fn contours_match(a: &Geometry, b: &Geometry, tolerance: f64) -> bool {
    if is_open(a) != is_open(b) {
        return false;
    }
    if !rects_match(&a.rect(), &b.rect(), tolerance) {
        return false;
    }
    covers(b, &sample_points(a), tolerance)
        && covers(a, &sample_points(b), tolerance)
}

/// A spatial hash of the lower-left bounding box corner of kept
/// contours. Duplicates have corners within the tolerance, so only
/// neighbouring cells need to be compared.
struct CornerIndex {
    cell: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl CornerIndex {
    fn new(tolerance: f64) -> Self {
        CornerIndex {
            cell: tolerance.max(1e-9),
            cells: HashMap::new(),
        }
    }

    fn key(&self, x: f64, y: f64) -> (i64, i64) {
        (
            (x / self.cell).floor() as i64,
            (y / self.cell).floor() as i64,
        )
    }

    fn add(&mut self, index: usize, rect: &Rect) {
        self.cells
            .entry(self.key(rect.min.x, rect.min.y))
            .or_default()
            .push(index);
    }

    fn near(&self, rect: &Rect) -> Vec<usize> {
        let (kx, ky) = self.key(rect.min.x, rect.min.y);
        let mut found = Vec::new();
        for dx in [-1_i64, 0, 1] {
            for dy in [-1_i64, 0, 1] {
                if let Some(cell) = self.cells.get(&(kx + dx, ky + dy)) {
                    found.extend(cell.iter().copied());
                }
            }
        }
        found
    }
}

/// Removes contours that duplicate an earlier contour within the
/// tolerance, regardless of direction or start point. The first
/// occurrence is kept and the order of the remaining contours is
/// preserved.
///
/// Returns the new geometry and the number of removed contours.
pub fn remove_duplicate_contours(
    geo: &Geometry,
    tolerance: f64,
) -> (Geometry, usize) {
    let contours = drawn_contours(geo);
    let slack = 4.0 * tolerance + 1e-9;
    let mut index = CornerIndex::new(tolerance);
    let mut lengths: Vec<f64> = Vec::new();
    let mut kept: Vec<Geometry> = Vec::new();
    let mut removed = 0;
    for contour in contours {
        let rect = contour.rect();
        let length = contour.distance();
        let duplicate = index.near(&rect).into_iter().any(|k| {
            (lengths[k] - length).abs() <= slack
                && contours_match(&kept[k], &contour, tolerance)
        });
        if duplicate {
            removed += 1;
            continue;
        }
        index.add(kept.len(), &rect);
        lengths.push(length);
        kept.push(contour);
    }
    (concat(&kept), removed)
}

/// True if both geometries consist of the same contours within the
/// tolerance, in any order, direction or start point. Empty geometries
/// never match.
pub fn geometries_match(a: &Geometry, b: &Geometry, tolerance: f64) -> bool {
    let contours_a = drawn_contours(a);
    let contours_b = drawn_contours(b);
    if contours_a.is_empty() || contours_a.len() != contours_b.len() {
        return false;
    }
    let mut taken = vec![false; contours_b.len()];
    for contour in &contours_a {
        let found = contours_b.iter().enumerate().position(|(j, other)| {
            !taken[j] && contours_match(contour, other, tolerance)
        });
        match found {
            Some(j) => taken[j] = true,
            None => return false,
        }
    }
    true
}

/// Splits a geometry into one geometry per contour. Unlike splitting
/// into connected components, holes become parts of their own and open
/// contours are kept. Contours that draw nothing are dropped.
pub fn split_drawn_contours(geo: &Geometry) -> Vec<Geometry> {
    drawn_contours(geo)
}
