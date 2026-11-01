//! Exact planar offsetting of closed contours made of lines and circular arcs.
//!
//! Unlike [`crate::geo::algo::offset`], which routes every boundary through
//! polygon approximations, this module keeps lines and arcs exact: an arc in
//! the boundary comes out as an arc in the offset, joined to its neighbours
//! by tangent rolling arcs around corners that open away from the offset
//! side. The method is the classic one: every boundary piece is offset, the
//! offset curves are cut where they cross, and a cut piece is kept only
//! where it stays no closer to the original boundary than the offset —
//! which also trims the offset where it would run past a concave corner or
//! into another loop's kept side. The surviving pieces are stitched into
//! closed loops.
//!
//! Loops follow the winding convention of [`crate::geo::algo::topology`]:
//! counter-clockwise loops bound material, clockwise loops bound holes, and
//! the region is what the material loops enclose minus their holes. A
//! positive `offset` grows the region (hole loops shrink), a negative one
//! shrinks it. Open subpaths are ignored, and other curve kinds are
//! followed as line chains within `tolerance`.
//!
//! The geometry itself is not re-implemented here: pieces are
//! [`Command`]s queried through their own methods, intersections come from
//! [`crate::geo::shape::line`] and [`crate::geo::shape::circle`], arc
//! angles from [`crate::geo::shape::arc`].

use std::f64::consts::TAU;

use crate::geo::geometry::Geometry;
use crate::geo::shape::arc::{
    get_arc_chord_steps, get_arc_sweep, is_angle_between, normalize_angle,
};
use crate::geo::shape::circle::{
    get_circle_circle_intersections, get_line_circle_intersections,
};
use crate::geo::shape::line::{
    get_line_line_intersection, get_line_segment_intersection,
};
use crate::geo::types::{Command, Point, Point3D};

/// Most boundary pieces of one call (a spline counts by its line chains).
const MAX_PIECES: usize = 20_000;
/// Most pieces after cutting the offset curves where they cross.
const MAX_SPLIT_PIECES: usize = 400_000;
/// How far apart consecutive pieces of a contour may start and end.
const CLOSED_TOL: f64 = 1e-6;
/// A point this much closer to the boundary than the offset distance still counts as clear.
const CLEAR_TOL: f64 = 1e-9;
/// Arcs and lines shorter than this are degenerate.
const MIN_PIECE: f64 = 1e-9;

/// A parameter within this of 0 or 1 counts as on the piece.
const PARAM_SLACK: f64 = 1e-9;

/// The 2D part of a 3D point.
fn xy(p: Point3D) -> Point {
    Point::new(p.x, p.y)
}

/// Signed angle from `a` to `b` in `(-π, π]`.
fn turn(a: Point, b: Point) -> f64 {
    a.perp_dot(b).atan2(a.dot(b))
}

/// The angle of `p` seen from the origin.
fn angle_of(p: Point) -> f64 {
    p.y.atan2(p.x)
}

/// `(center, radius, start angle, signed sweep)` of an arc command in the XY
/// plane, or `None` for other commands and degenerate arcs.
fn arc_parts(
    start: Point3D,
    command: &Command,
) -> Option<(Point, f64, f64, f64)> {
    let Command::Arc {
        end,
        center_offset,
        normal,
    } = command
    else {
        return None;
    };
    let (from, to, off) = (xy(start), xy(*end), xy(*center_offset));
    let c = from + off;
    let r = off.length();
    if r <= MIN_PIECE {
        return None;
    }
    let clockwise = normal.z < 0.0;
    let sweep = if from.distance(to) <= MIN_PIECE {
        // The command closes its own circle.
        if clockwise {
            -TAU
        } else {
            TAU
        }
    } else {
        get_arc_sweep(angle_of(from - c), angle_of(to - c), clockwise)
    };
    if sweep.abs() <= 1e-9 {
        return None;
    }
    Some((c, r, angle_of(from - c), sweep))
}

/// One drawing command of a contour, with the point it starts from.
#[derive(Clone, Debug)]
struct Span {
    start: Point3D,
    command: Command,
}

impl Span {
    fn end(&self) -> Point3D {
        self.command.end_point()
    }

    fn point_at(&self, t: f64) -> Point3D {
        self.command.point_at(self.start, t).unwrap_or(self.start)
    }

    fn length(&self) -> f64 {
        self.command.length(self.start)
    }

    /// Unit direction of travel at parameter `t`, projected into the XY plane.
    fn tangent_at(&self, t: f64) -> Point {
        self.command.tangent_at(self.start, t).unwrap_or_default()
    }

    /// Distance from `p` to the nearest point of the span.
    fn dist(&self, p: Point) -> f64 {
        self.command
            .closest_point_to(self.start, p.x, p.y)
            .map_or(f64::INFINITY, |(_, _, dist_sq)| dist_sq.sqrt())
    }

    /// The part of the span between the parameters `t0` and `t1` (t0 < t1).
    fn sub(&self, t0: f64, t1: f64) -> Span {
        let start = self.point_at(t0);
        let command = match self.command {
            Command::Line { .. } => Command::Line {
                end: self.point_at(t1),
            },
            Command::Arc { normal, .. } => {
                match arc_parts(self.start, &self.command) {
                    Some((c, ..)) => {
                        let end = self.point_at(t1);
                        Command::Arc {
                            end,
                            center_offset: Point3D::new(
                                c.x - start.x,
                                c.y - start.y,
                                0.0,
                            ),
                            normal,
                        }
                    }
                    None => Command::Line {
                        end: self.point_at(t1),
                    },
                }
            }
            ref other => other.clone(),
        };
        Span { start, command }
    }

    /// A box that contains the span (an arc's whole circle, so pruning with
    /// it never misses a crossing).
    fn bounds(&self) -> (Point, Point) {
        match self.command {
            Command::Line { end } => {
                let (a, b) = (xy(self.start), xy(end));
                (
                    Point::new(a.x.min(b.x), a.y.min(b.y)),
                    Point::new(a.x.max(b.x), a.y.max(b.y)),
                )
            }
            Command::Arc { .. } => match arc_parts(self.start, &self.command) {
                Some((c, r, ..)) => {
                    (Point::new(c.x - r, c.y - r), Point::new(c.x + r, c.y + r))
                }
                None => (xy(self.start), xy(self.start)),
            },
            _ => (xy(self.start), xy(self.start)),
        }
    }

    /// Half of ∮(x dy − y dx) along the span: the spans of a closed loop sum
    /// to the loop's signed area (positive = counter-clockwise).
    fn area_term(&self) -> f64 {
        match self.command {
            Command::Line { end } => 0.5 * xy(self.start).perp_dot(xy(end)),
            Command::Arc { .. } => match arc_parts(self.start, &self.command) {
                Some((c, r, _, sweep)) => {
                    0.5 * (r * r * sweep
                        + c.perp_dot(xy(self.end()) - xy(self.start)))
                }
                None => 0.0,
            },
            _ => 0.0,
        }
    }

    fn is_finite(&self) -> bool {
        self.start.is_finite() && self.end().is_finite()
    }

    /// Where the two spans cross: `(parameter on self, parameter on other)`
    /// pairs, each within `[-PARAM_SLACK, 1 + PARAM_SLACK]`. Spans lying
    /// along each other (collinear lines, arcs of one circle) report nothing.
    fn intersections(&self, o: &Span) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        let a_line = matches!(self.command, Command::Line { .. });
        let b_line = matches!(o.command, Command::Line { .. });
        match (a_line, b_line) {
            (true, true) => {
                let Some(p) = get_line_segment_intersection(
                    xy(self.start),
                    xy(self.end()),
                    xy(o.start),
                    xy(o.end()),
                ) else {
                    return out;
                };
                let t = line_parameter(xy(self.start), xy(self.end()), p);
                let u = line_parameter(xy(o.start), xy(o.end()), p);
                if on_piece(t) && on_piece(u) {
                    out.push((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)));
                }
            }
            (true, false) | (false, true) => {
                let (line, arc, flip) = if a_line {
                    (self, o, false)
                } else {
                    (o, self, true)
                };
                let Some((c, r, a0, sweep)) =
                    arc_parts(arc.start, &arc.command)
                else {
                    return out;
                };
                for p in get_line_circle_intersections(
                    xy(line.start),
                    xy(line.end()),
                    c,
                    r,
                ) {
                    let t = line_parameter(xy(line.start), xy(line.end()), p);
                    if let (Some(u), true) =
                        (arc_parameter(a0, sweep, angle_of(p - c)), on_piece(t))
                    {
                        if flip {
                            out.push((u, t.clamp(0.0, 1.0)));
                        } else {
                            out.push((t.clamp(0.0, 1.0), u));
                        }
                    }
                }
            }
            (false, false) => {
                let (Some((c1, r1, a01, sweep1)), Some((c2, r2, a02, sweep2))) = (
                    arc_parts(self.start, &self.command),
                    arc_parts(o.start, &o.command),
                ) else {
                    return out;
                };
                for p in get_circle_circle_intersections(c1, r1, c2, r2) {
                    if let (Some(t), Some(u)) = (
                        arc_parameter(a01, sweep1, angle_of(p - c1)),
                        arc_parameter(a02, sweep2, angle_of(p - c2)),
                    ) {
                        out.push((t, u));
                    }
                }
            }
        }
        out
    }
}

fn on_piece(t: f64) -> bool {
    (-PARAM_SLACK..=1.0 + PARAM_SLACK).contains(&t)
}

/// The parameter of `p` projected onto the line `a`–`b`, clamped to [0, 1].
fn line_parameter(a: Point, b: Point, p: Point) -> f64 {
    let d = b - a;
    let l2 = d.length_squared();
    if l2 < 1e-20 {
        return 0.0;
    }
    ((p - a).dot(d) / l2).clamp(0.0, 1.0)
}

/// The parameter at which an arc from `a0` sweeping `sweep` passes angle
/// `theta`, or `None` when the angle is outside the arc's span.
fn arc_parameter(a0: f64, sweep: f64, theta: f64) -> Option<f64> {
    let span = sweep.abs();
    if span <= 0.0 {
        return None;
    }
    if !is_angle_between(theta, a0, a0 + sweep, sweep < 0.0) {
        return None;
    }
    let delta = if sweep < 0.0 {
        normalize_angle(a0 - theta)
    } else {
        normalize_angle(theta - a0)
    };
    Some((delta / span).min(1.0))
}

/// A closed contour as spans.
struct Contour {
    spans: Vec<Span>,
}

/// The closed subpaths of `geometry` as contours. Lines and arcs stay exact;
/// other curves are followed as line chains within `tolerance`. Subpaths
/// that are open, degenerate, or not finite are dropped.
fn contours(geometry: &Geometry, tolerance: f64) -> Vec<Contour> {
    let mut out = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut pos = Point3D::ZERO;
    let mut moved = false;
    for command in geometry.data() {
        if let Command::Move { end } = *command {
            if let Some(contour) = finish_contour(std::mem::take(&mut spans)) {
                out.push(contour);
            }
            pos = end;
            moved = true;
            continue;
        }
        if !moved {
            continue;
        }
        push_span(&mut spans, command, pos, tolerance);
        pos = command.end_point();
        if spans.len() > MAX_PIECES {
            return Vec::new();
        }
    }
    if let Some(contour) = finish_contour(spans) {
        out.push(contour);
    }
    out
}

/// Appends `command` as spans starting at `start`. Lines and arcs are kept
/// exact; other curves become line chains within `tolerance`.
fn push_span(
    spans: &mut Vec<Span>,
    command: &Command,
    start: Point3D,
    tolerance: f64,
) {
    match *command {
        Command::Line { end } => {
            if xy(start).distance(xy(end)) > MIN_PIECE {
                spans.push(Span {
                    start,
                    command: Command::Line { end },
                });
            }
        }
        Command::Arc { .. } => {
            if arc_parts(start, command).is_some() {
                spans.push(Span {
                    start,
                    command: command.clone(),
                });
            }
        }
        Command::Bezier { .. } => {
            let mut flat: Vec<(Point3D, Point3D)> = Vec::new();
            command.linearize(start, tolerance, &mut flat);
            for (a, b) in flat {
                if xy(a).distance(xy(b)) > MIN_PIECE {
                    spans.push(Span {
                        start: a,
                        command: Command::Line { end: b },
                    });
                }
            }
        }
        Command::Move { .. } => {}
    }
}

/// A closed subpath as a contour, or `None` when it is open, degenerate, or
/// not finite.
fn finish_contour(spans: Vec<Span>) -> Option<Contour> {
    if spans.len() < 2 || !spans.iter().all(Span::is_finite) {
        return None;
    }
    let first = spans.first()?;
    let last = spans.last()?;
    if xy(first.start).distance(xy(last.end())) > CLOSED_TOL {
        return None;
    }
    let area: f64 = spans.iter().map(Span::area_term).sum();
    if area.is_nan() || area.abs() <= 1e-9 {
        return None;
    }
    Some(Contour { spans })
}

/// The boundary loops of one geometry, with the region on each loop's left:
/// counter-clockwise material loops, clockwise hole loops, as the commands
/// already wind them.
struct Boundary {
    loops: Vec<Contour>,
}

impl Boundary {
    fn dist(&self, p: Point) -> f64 {
        self.loops
            .iter()
            .flat_map(|c| &c.spans)
            .map(|span| span.dist(p))
            .fold(f64::INFINITY, f64::min)
    }

    /// Whether `p` is inside the region. `sag`: how far `p` is known to be
    /// from the boundary (arcs are walked in chords that stay within a
    /// quarter of it).
    fn inside(&self, p: Point, sag: f64) -> bool {
        let mut total = 0.0;
        for span in self.loops.iter().flat_map(|c| &c.spans) {
            match span.command {
                Command::Line { end } => {
                    total += turn(xy(span.start) - p, xy(end) - p);
                }
                Command::Arc { .. } => {
                    let Some((_, r, _, sweep)) =
                        arc_parts(span.start, &span.command)
                    else {
                        continue;
                    };
                    let n = get_arc_chord_steps(r, sweep, sag * 0.25);
                    let mut prev = span.start;
                    for i in 1..=n {
                        let q = span.point_at(i as f64 / n as f64);
                        total += turn(xy(prev) - p, xy(q) - p);
                        prev = q;
                    }
                }
                _ => {}
            }
        }
        (total / TAU).round() != 0.0
    }

    /// The boundary moved by `d` (positive: into the region, so it shrinks;
    /// negative: away from it), as closed loops of what is kept.
    /// `tolerance` decides miter against roll at corners that open away from
    /// the offset side.
    fn offset(&self, d: f64, tolerance: f64) -> Vec<Vec<Span>> {
        if !d.is_finite() {
            return Vec::new();
        }
        let ad = d.abs();
        if ad <= MIN_PIECE {
            return self.loops.iter().map(|c| c.spans.clone()).collect();
        }
        // Crossings closer than this to a piece's end are not cut: the
        // leftover would be too short to tell from a valid piece (see
        // `CLEAR_TOL`), and the gap it leaves is closed when the pieces are
        // joined.
        let min_len = 2.0 * (ad * CLEAR_TOL).sqrt() + 1e-6;
        let mut raw: Vec<Span> = Vec::new();
        for contour in &self.loops {
            // Each piece offset; an arc whose radius the offset eats vanishes.
            let mut offset: Vec<Span> = Vec::new();
            for span in &contour.spans {
                match span.command {
                    Command::Line { end } => {
                        let n = span.tangent_at(0.0).perp() * d;
                        offset.push(Span {
                            start: span.start + Point3D::new(n.x, n.y, 0.0),
                            command: Command::Line {
                                end: end + Point3D::new(n.x, n.y, 0.0),
                            },
                        });
                    }
                    Command::Arc { normal, .. } => {
                        let Some((c, r, _, sweep)) =
                            arc_parts(span.start, &span.command)
                        else {
                            continue;
                        };
                        let r_new = r - d * sweep.signum();
                        if r_new <= MIN_PIECE {
                            continue;
                        }
                        // Same center, new radius: both ends move radially.
                        let scale = r_new / r;
                        let new_start = c + (xy(span.start) - c) * scale;
                        let old_end = span.end();
                        let new_end = c + (xy(old_end) - c) * scale;
                        offset.push(Span {
                            start: Point3D::new(
                                new_start.x,
                                new_start.y,
                                span.start.z,
                            ),
                            command: Command::Arc {
                                end: Point3D::new(
                                    new_end.x, new_end.y, old_end.z,
                                ),
                                center_offset: Point3D::new(
                                    c.x - new_start.x,
                                    c.y - new_start.y,
                                    0.0,
                                ),
                                normal,
                            },
                        });
                    }
                    _ => {}
                }
            }
            // Joins: a corner that opens away from the offset side rolls
            // round the vertex, unless a flat miter stays within `tolerance`
            // of that round join — then the two line pieces are extended to
            // their miter point. Corners on the other side overlap and are
            // left for the crossing cut below.
            let mut rolls: Vec<(usize, Span)> = Vec::new();
            for i in 0..offset.len() {
                let a = &offset[i];
                let b = &offset[(i + 1) % offset.len()];
                let bend = turn(a.tangent_at(1.0), b.tangent_at(0.0));
                if bend * d >= 0.0 || bend.abs() <= 1e-9 {
                    continue;
                }
                let miter_deviation =
                    ad * (1.0 / (bend.abs() * 0.5).cos() - 1.0);
                let joint = if miter_deviation <= tolerance {
                    match (&a.command, &b.command) {
                        // The infinite lines meet behind the span ends on
                        // this side; the miter point is their extrapolation.
                        (Command::Line { .. }, Command::Line { .. }) => {
                            get_line_line_intersection(
                                xy(a.start),
                                xy(a.end()),
                                xy(b.start),
                                xy(b.end()),
                            )
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                match joint {
                    Some(p) => {
                        if let Command::Line { end } = &mut offset[i].command {
                            end.x = p.x;
                            end.y = p.y;
                        }
                        let next = (i + 1) % offset.len();
                        offset[next].start =
                            Point3D::new(p.x, p.y, offset[next].start.z);
                    }
                    None => {
                        // The roll runs round the original vertex, which the
                        // offset piece's end sits one offset away from along
                        // its own normal.
                        let dir_a = a.tangent_at(1.0);
                        let vertex = xy(a.end()) - dir_a.perp() * d;
                        let n_start = dir_a.perp() * d;
                        let n_end = b.tangent_at(0.0).perp() * d;
                        let z = a.end().z;
                        rolls.push((
                            (i + 1).min(offset.len()),
                            Span {
                                start: Point3D::new(
                                    vertex.x + n_start.x,
                                    vertex.y + n_start.y,
                                    z,
                                ),
                                command: Command::Arc {
                                    end: Point3D::new(
                                        vertex.x + n_end.x,
                                        vertex.y + n_end.y,
                                        z,
                                    ),
                                    center_offset: Point3D::new(
                                        -n_start.x, -n_start.y, 0.0,
                                    ),
                                    normal: Point3D::new(
                                        0.0,
                                        0.0,
                                        if bend > 0.0 { 1.0 } else { -1.0 },
                                    ),
                                },
                            },
                        ));
                    }
                }
            }
            for (at, span) in rolls.into_iter().rev() {
                offset.insert(at, span);
            }
            raw.extend(offset);
        }
        if raw.len() > MAX_SPLIT_PIECES {
            return Vec::new();
        }

        let boxes: Vec<(Point, Point)> = raw.iter().map(Span::bounds).collect();
        let mut cuts: Vec<Vec<f64>> = vec![Vec::new(); raw.len()];
        let mut hits: Vec<(f64, f64)> = Vec::new();
        for (i, (a, (amin, amax))) in raw.iter().zip(&boxes).enumerate() {
            for (j, (b, (bmin, bmax))) in
                raw.iter().zip(&boxes).enumerate().skip(i + 1)
            {
                if amin.x > bmax.x + 1e-9
                    || bmin.x > amax.x + 1e-9
                    || amin.y > bmax.y + 1e-9
                    || bmin.y > amax.y + 1e-9
                {
                    continue;
                }
                hits.clear();
                for &(t, u) in &a.intersections(b) {
                    if let Some(c) = cuts.get_mut(i) {
                        c.push(t);
                    }
                    if let Some(c) = cuts.get_mut(j) {
                        c.push(u);
                    }
                }
            }
        }

        let mut kept: Vec<Span> = Vec::new();
        let mut total = 0usize;
        for (span, ts) in raw.iter().zip(cuts.iter_mut()) {
            let len = span.length();
            ts.retain(|t| t * len > min_len && (1.0 - t) * len > min_len);
            ts.sort_by(f64::total_cmp);
            let mut t0 = 0.0;
            for &t in ts.iter().chain(std::iter::once(&1.0)) {
                if t < 1.0 && (t - t0) * len <= min_len {
                    continue;
                }
                total += 1;
                if total > MAX_SPLIT_PIECES {
                    return Vec::new();
                }
                let sub = span.sub(t0, t);
                let mid = xy(sub.point_at(0.5));
                if self.dist(mid) >= ad - CLEAR_TOL
                    && self.inside(mid, ad) == (d > 0.0)
                {
                    kept.push(sub);
                }
                t0 = t;
            }
        }

        let join_tol = 4.0 * min_len;
        stitch(&kept, join_tol)
            .into_iter()
            .filter(|lp| {
                let (area, length) = lp.iter().fold((0.0, 0.0), |(a, l), p| {
                    (a + p.area_term(), l + p.length())
                });
                area.abs() > join_tol * length
            })
            .collect()
    }
}

/// Joins pieces end to start into closed loops. Where several pieces leave a
/// point the left-most one is taken: the kept side of every piece is on its
/// left, so the boundary of what is kept turns left there. Chains that do
/// not close are dropped.
fn stitch(pieces: &[Span], tol: f64) -> Vec<Vec<Span>> {
    let mut used = vec![false; pieces.len()];
    let mut loops = Vec::new();
    for (s, first) in pieces.iter().enumerate() {
        if std::mem::replace(used.get_mut(s).unwrap_or(&mut true), true) {
            continue;
        }
        let origin = xy(first.start);
        let mut chain = vec![first.clone()];
        let mut closed = false;
        for _ in 0..pieces.len() {
            let Some(last) = chain.last() else { break };
            let (end, dir) = (xy(last.end()), last.tangent_at(1.0));
            if end.distance(origin) <= tol
                && (chain.len() > 1 || last.length() > 2.0 * tol)
            {
                closed = true;
                break;
            }
            let mut best: Option<(usize, f64)> = None;
            for (j, (c, taken)) in pieces.iter().zip(&used).enumerate() {
                if *taken || xy(c.start).distance(end) > tol {
                    continue;
                }
                let bend = turn(dir, c.tangent_at(0.0));
                if best.is_none_or(|(_, b)| bend > b) {
                    best = Some((j, bend));
                }
            }
            let Some((j, _)) = best else { break };
            if let (Some(flag), Some(piece)) = (used.get_mut(j), pieces.get(j))
            {
                *flag = true;
                chain.push(piece.clone());
            }
        }
        if closed {
            loops.push(chain);
        }
    }
    loops
}

/// A kept loop as a geometry: one `move_to`, then the loop's line and exact
/// arc commands.
fn loop_geometry(pieces: &[Span]) -> Geometry {
    let mut g = Geometry::new();
    let Some(first) = pieces.first() else {
        return g;
    };
    g.move_to(first.start.x, first.start.y, first.start.z);
    for span in pieces {
        match span.command {
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
            _ => {}
        }
    }
    let last = pieces[pieces.len() - 1].end();
    if xy(last).distance(xy(first.start)) > CLOSED_TOL {
        g.close_path();
    }
    g
}

/// The boundaries of `geometry` moved by `offset` (positive: outwards, so
/// the region grows; negative: into it), as closed loops of lines and arcs.
///
/// Every closed subpath of the geometry takes part: counter-clockwise loops
/// as material, clockwise ones as holes, whatever the nesting. Lines and
/// arcs stay exact; other curves are followed as line chains within
/// `tolerance`. Open subpaths are ignored. Degenerate input, a non-finite or
/// zero `offset`, or more pieces than the module's internal limits return an
/// empty result.
pub fn offset_exact(
    geometry: &Geometry,
    offset: f64,
    tolerance: f64,
) -> Vec<Geometry> {
    if !offset.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return Vec::new();
    }
    let loops = contours(geometry, tolerance);
    if loops.is_empty() {
        return Vec::new();
    }
    let total: usize = loops.iter().map(|c| c.spans.len()).sum();
    if total > MAX_PIECES {
        return Vec::new();
    }
    let boundary = Boundary { loops };
    // Positive grows the region; the core moves the boundary into the
    // region for a positive d.
    boundary
        .offset(-offset, tolerance)
        .into_iter()
        .map(|lp| loop_geometry(&lp))
        .collect()
}
