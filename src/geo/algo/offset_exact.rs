//! Exact planar offsetting of closed contours made of lines and circular arcs.
//!
//! Unlike [`crate::geo::algo::offset`], which routes every boundary through
//! polygon approximations, this module keeps lines and arcs exact: an arc in
//! the boundary comes out as an arc in the offset, joined to its neighbours by
//! tangent rolling arcs around corners that open away from the offset side.
//! The method is the classic one: every boundary piece is offset, the offset
//! curves are cut where they cross, and a cut piece is kept only where it
//! stays no closer to the original boundary than the offset — which also
//! trims the offset where it would run past a concave corner or into another
//! loop's kept side. The surviving pieces are stitched into closed loops.
//!
//! Loops follow the winding convention of [`crate::geo::algo::topology`]:
//! counter-clockwise loops bound material, clockwise loops bound holes, and
//! the region is what the material loops enclose minus their holes. A positive
//! `offset` grows the region (hole loops shrink), a negative one shrinks it.
//! Open subpaths are ignored, and other curve kinds are followed as line
//! chains within `tolerance`.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::geo::geometry::Geometry;
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

/// Angle wrapped into `[0, 2π)`.
fn wrap(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    if r.is_finite() {
        r
    } else {
        0.0
    }
}

/// Signed angle from `a` to `b` in `(-π, π]`.
fn turn(a: Point, b: Point) -> f64 {
    a.perp_dot(b).atan2(a.dot(b))
}

/// The angle of `p` seen from the origin.
fn angle_of(p: Point) -> f64 {
    p.y.atan2(p.x)
}

/// The 2D part of a 3D point.
fn xy(p: Point3D) -> Point {
    Point::new(p.x, p.y)
}

#[derive(Clone, Copy, Debug)]
enum Piece {
    Line {
        a: Point,
        b: Point,
    },
    /// Arc about `c` from angle `a0` sweeping `sweep` radians (positive = counter-clockwise).
    Arc {
        c: Point,
        r: f64,
        a0: f64,
        sweep: f64,
    },
}

impl Piece {
    fn start(&self) -> Point {
        self.point_at(0.0)
    }

    fn end(&self) -> Point {
        self.point_at(1.0)
    }

    fn point_at(&self, t: f64) -> Point {
        match *self {
            Piece::Line { a, b } => a.lerp(b, t),
            Piece::Arc { c, r, a0, sweep } => {
                c + Point::from_angle(a0 + sweep * t) * r
            }
        }
    }

    fn length(&self) -> f64 {
        match *self {
            Piece::Line { a, b } => a.distance(b),
            Piece::Arc { r, sweep, .. } => r * sweep.abs(),
        }
    }

    /// Unit direction of travel.
    fn tangent_at(&self, t: f64) -> Point {
        match *self {
            Piece::Line { a, b } => (b - a).normalize_or_zero(),
            Piece::Arc { a0, sweep, .. } => {
                Point::from_angle(a0 + sweep * t).perp() * sweep.signum()
            }
        }
    }

    /// The part between parameters `t0` and `t1`.
    fn sub(&self, t0: f64, t1: f64) -> Piece {
        match *self {
            Piece::Line { a, b } => Piece::Line {
                a: a.lerp(b, t0),
                b: a.lerp(b, t1),
            },
            Piece::Arc { c, r, a0, sweep } => Piece::Arc {
                c,
                r,
                a0: a0 + sweep * t0,
                sweep: sweep * (t1 - t0),
            },
        }
    }

    /// Distance from `p` to the nearest point of the piece.
    fn dist(&self, p: Point) -> f64 {
        match *self {
            Piece::Line { a, b } => {
                let d = b - a;
                let l2 = d.length_squared();
                let t = if l2 > 0.0 {
                    ((p - a).dot(d) / l2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                p.distance(a + d * t)
            }
            Piece::Arc { c, r, a0, sweep } => {
                match arc_param(a0, sweep, angle_of(p - c)) {
                    Some(_) => (p.distance(c) - r).abs(),
                    None => {
                        p.distance(self.start()).min(p.distance(self.end()))
                    }
                }
            }
        }
    }

    /// A box that contains the piece (an arc's whole circle).
    fn bounds(&self) -> (Point, Point) {
        match *self {
            Piece::Line { a, b } => (
                Point::new(a.x.min(b.x), a.y.min(b.y)),
                Point::new(a.x.max(b.x), a.y.max(b.y)),
            ),
            Piece::Arc { c, r, .. } => {
                (Point::new(c.x - r, c.y - r), Point::new(c.x + r, c.y + r))
            }
        }
    }

    /// Half of ∮(x dy − y dx) along the piece: the pieces of a closed loop sum to its signed area.
    fn area_term(&self) -> f64 {
        match *self {
            Piece::Line { a, b } => 0.5 * a.perp_dot(b),
            Piece::Arc { c, r, sweep, .. } => {
                0.5 * (r * r * sweep + c.perp_dot(self.end() - self.start()))
            }
        }
    }

    fn is_finite(&self) -> bool {
        match *self {
            Piece::Line { a, b } => a.is_finite() && b.is_finite(),
            Piece::Arc { c, r, a0, sweep } => {
                c.is_finite()
                    && r.is_finite()
                    && a0.is_finite()
                    && sweep.is_finite()
            }
        }
    }

    /// Where the two pieces cross: `(parameter on self, parameter on o)` pairs. Pieces lying
    /// along each other (collinear lines, arcs of one circle) report nothing.
    fn intersections(&self, o: &Piece, out: &mut Vec<(f64, f64)>) {
        match (*self, *o) {
            (Piece::Line { a: a1, b: b1 }, Piece::Line { a: a2, b: b2 }) => {
                let (d1, d2) = (b1 - a1, b2 - a2);
                let den = d1.perp_dot(d2);
                if den.is_nan()
                    || den.abs() <= 1e-12 * d1.length() * d2.length()
                {
                    return;
                }
                let w = a2 - a1;
                let (t, u) = (w.perp_dot(d2) / den, w.perp_dot(d1) / den);
                if on_piece(t) && on_piece(u) {
                    out.push((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)));
                }
            }
            (Piece::Line { a, b }, Piece::Arc { c, r, a0, sweep }) => {
                for (t, p) in line_circle(a, b, c, r) {
                    if let Some(u) = arc_param(a0, sweep, angle_of(p - c)) {
                        out.push((t, u));
                    }
                }
            }
            (Piece::Arc { .. }, Piece::Line { .. }) => {
                let mut swapped = Vec::new();
                o.intersections(self, &mut swapped);
                out.extend(swapped.into_iter().map(|(u, t)| (t, u)));
            }
            (
                Piece::Arc {
                    c: c1,
                    r: r1,
                    a0: s1,
                    sweep: w1,
                },
                Piece::Arc {
                    c: c2,
                    r: r2,
                    a0: s2,
                    sweep: w2,
                },
            ) => {
                let between = c2 - c1;
                let d = between.length();
                if d.is_nan()
                    || d <= 1e-12
                    || d > r1 + r2 + 1e-9
                    || d < (r1 - r2).abs() - 1e-9
                {
                    return;
                }
                let along = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
                let h = (r1 * r1 - along * along).max(0.0).sqrt();
                let dir = between / d;
                let base = c1 + dir * along;
                let pts = if h > 1e-12 {
                    vec![base + dir.perp() * h, base - dir.perp() * h]
                } else {
                    vec![base]
                };
                for p in pts {
                    if let (Some(t), Some(u)) = (
                        arc_param(s1, w1, angle_of(p - c1)),
                        arc_param(s2, w2, angle_of(p - c2)),
                    ) {
                        out.push((t, u));
                    }
                }
            }
        }
    }
}

fn on_piece(t: f64) -> bool {
    (-PARAM_SLACK..=1.0 + PARAM_SLACK).contains(&t)
}

/// The parameter at which an arc from `a0` sweeping `sweep` passes angle `theta`.
fn arc_param(a0: f64, sweep: f64, theta: f64) -> Option<f64> {
    let span = sweep.abs();
    if span.is_nan() || span <= 0.0 {
        return None;
    }
    let delta = if sweep > 0.0 {
        wrap(theta - a0)
    } else {
        wrap(a0 - theta)
    };
    if delta <= span + PARAM_SLACK {
        Some((delta / span).min(1.0))
    } else if delta >= TAU - PARAM_SLACK {
        // Just before the start: the angle wrapped to nearly a full turn.
        Some(0.0)
    } else {
        None
    }
}

/// Where the segment `a`–`b` meets the circle: `(parameter, point)`.
fn line_circle(a: Point, b: Point, c: Point, r: f64) -> Vec<(f64, Point)> {
    let d = b - a;
    let len = d.length();
    if len.is_nan() || len <= 1e-12 {
        return Vec::new();
    }
    let f = a - c;
    let h = d.perp_dot(f).abs() / len;
    if h > r + 1e-9 {
        return Vec::new();
    }
    let mid = -f.dot(d) / (len * len);
    let half = (r * r - h * h).max(0.0).sqrt() / len;
    let ts = if half * len > 1e-12 {
        vec![mid - half, mid + half]
    } else {
        vec![mid]
    };
    ts.into_iter()
        .filter(|t| on_piece(*t))
        .map(|t| (t.clamp(0.0, 1.0), a + d * t.clamp(0.0, 1.0)))
        .collect()
}

/// A closed contour as pieces, with the Z its source subpath sits in.
struct Contour {
    pieces: Vec<Piece>,
    z: f64,
}

/// Appends `command`'s pieces, starting at `start`; returns the command's end point.
fn push_command(
    pieces: &mut Vec<Piece>,
    command: &Command,
    start: Point3D,
    tolerance: f64,
) -> Point3D {
    match *command {
        Command::Line { end } => {
            if xy(start).distance(xy(end)) > MIN_PIECE {
                pieces.push(Piece::Line {
                    a: xy(start),
                    b: xy(end),
                });
            }
            end
        }
        Command::Arc {
            end,
            center_offset,
            normal,
        } => {
            let (from, to, off) = (xy(start), xy(end), xy(center_offset));
            let c = from + off;
            let r = off.length();
            let raw = turn(from - c, to - c);
            let sweep = if from.distance(to) <= MIN_PIECE {
                // The command closes its own circle.
                if normal.z >= 0.0 {
                    TAU
                } else {
                    -TAU
                }
            } else if normal.z >= 0.0 {
                if raw >= 0.0 {
                    raw
                } else {
                    raw + TAU
                }
            } else if raw <= 0.0 {
                raw
            } else {
                raw - TAU
            };
            if r > MIN_PIECE && sweep.abs() > 1e-9 {
                pieces.push(Piece::Arc {
                    c,
                    r,
                    a0: angle_of(from - c),
                    sweep,
                });
            }
            end
        }
        Command::Bezier { end, .. } => {
            let mut flat: Vec<(Point3D, Point3D)> = Vec::new();
            command.linearize(start, tolerance, &mut flat);
            for (a, b) in flat {
                if xy(a).distance(xy(b)) > MIN_PIECE {
                    pieces.push(Piece::Line { a: xy(a), b: xy(b) });
                }
            }
            end
        }
        Command::Move { end } => end,
    }
}

/// A closed subpath as pieces, or `None` when it is open, degenerate, or not finite.
fn finish_contour(pieces: Vec<Piece>, z: f64) -> Option<Contour> {
    if pieces.len() < 2 || !pieces.iter().all(Piece::is_finite) {
        return None;
    }
    let first = *pieces.first()?;
    if first.start().distance(pieces[pieces.len() - 1].end()) > CLOSED_TOL {
        return None;
    }
    let area: f64 = pieces.iter().map(Piece::area_term).sum();
    if area.is_nan() || area.abs() <= 1e-9 {
        return None;
    }
    Some(Contour { pieces, z })
}

/// The closed subpaths of `geometry` as contours.
fn contours(geometry: &Geometry, tolerance: f64) -> Vec<Contour> {
    let mut out = Vec::new();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut pos = Point3D::ZERO;
    let mut z = 0.0;
    let mut moved = false;
    for command in geometry.data() {
        if let Command::Move { end } = *command {
            if let Some(contour) =
                finish_contour(std::mem::take(&mut pieces), z)
            {
                out.push(contour);
            }
            pos = end;
            z = end.z;
            moved = true;
            continue;
        }
        if !moved {
            continue;
        }
        pos = push_command(&mut pieces, command, pos, tolerance);
        if pieces.len() > MAX_PIECES {
            return Vec::new();
        }
    }
    if let Some(contour) = finish_contour(pieces, z) {
        out.push(contour);
    }
    out
}

/// How many chords walk an arc with at most `sag` between chord and arc.
fn arc_steps(r: f64, sweep: f64, sag: f64) -> usize {
    let step = if sag > 0.0 && sag < r {
        2.0 * (1.0 - sag / r).acos()
    } else {
        FRAC_PI_2
    };
    let n = (sweep.abs() / step.clamp(1e-3, FRAC_PI_2)).ceil();
    if n.is_finite() {
        (n as usize).clamp(1, 8192)
    } else {
        1
    }
}

/// The boundary loops of one geometry, with the region on each loop's left: counter-clockwise
/// material loops, clockwise hole loops, as the commands already wind them.
struct Boundary {
    loops: Vec<Contour>,
}

impl Boundary {
    fn dist(&self, p: Point) -> f64 {
        self.loops
            .iter()
            .flat_map(|c| &c.pieces)
            .map(|piece| piece.dist(p))
            .fold(f64::INFINITY, f64::min)
    }

    /// Whether `p` is inside the region. `sag`: how far `p` is known to be from the boundary
    /// (arcs are walked in chords that stay within a quarter of it).
    fn inside(&self, p: Point, sag: f64) -> bool {
        let mut total = 0.0;
        for piece in self.loops.iter().flat_map(|c| &c.pieces) {
            match *piece {
                Piece::Line { a, b } => total += turn(a - p, b - p),
                Piece::Arc { r, sweep, .. } => {
                    let n = arc_steps(r, sweep, sag * 0.25);
                    let mut prev = piece.start();
                    for i in 1..=n {
                        let q = piece.point_at(i as f64 / n as f64);
                        total += turn(prev - p, q - p);
                        prev = q;
                    }
                }
            }
        }
        (total / TAU).round() != 0.0
    }

    /// The boundary moved by `d` (positive: into the region, so it shrinks; negative: away from
    /// it), as closed loops of what is kept, each with the Z of its first piece's contour.
    /// `tolerance` decides miter against roll at corners that open away from the offset side.
    fn offset(&self, d: f64, tolerance: f64) -> Vec<(Vec<Piece>, f64)> {
        if !d.is_finite() {
            return Vec::new();
        }
        let ad = d.abs();
        if ad <= MIN_PIECE {
            return self
                .loops
                .iter()
                .map(|c| (c.pieces.clone(), c.z))
                .collect();
        }
        // Crossings closer than this to a piece's end are not cut: the leftover would be too
        // short to tell from a valid piece (see `CLEAR_TOL`), and the gap it leaves is closed
        // when the pieces are joined.
        let min_len = 2.0 * (ad * CLEAR_TOL).sqrt() + 1e-6;
        let mut raw: Vec<(Piece, f64)> = Vec::new();
        for contour in &self.loops {
            // Each piece offset; an arc whose radius the offset eats vanishes.
            let mut offset: Vec<Piece> = Vec::new();
            for piece in &contour.pieces {
                match *piece {
                    Piece::Line { a, b } => {
                        let n = piece.tangent_at(0.0).perp() * d;
                        offset.push(Piece::Line { a: a + n, b: b + n });
                    }
                    Piece::Arc { c, r, a0, sweep } => {
                        let r = r - d * sweep.signum();
                        if r > MIN_PIECE {
                            offset.push(Piece::Arc { c, r, a0, sweep });
                        }
                    }
                }
            }
            // Joins: a corner that opens away from the offset side rolls round the vertex,
            // unless a flat miter stays within `tolerance` of that round join — then the two
            // line pieces are extended to their miter point. Corners on the other side
            // overlap and are left for the crossing cut below.
            let mut rolls: Vec<(usize, Piece)> = Vec::new();
            for i in 0..offset.len() {
                let a = offset[i];
                let b = offset[(i + 1) % offset.len()];
                let bend = turn(a.tangent_at(1.0), b.tangent_at(0.0));
                if bend * d >= 0.0 || bend.abs() <= 1e-9 {
                    continue;
                }
                let miter_deviation =
                    ad * (1.0 / (bend.abs() * 0.5).cos() - 1.0);
                let joint = if miter_deviation <= tolerance {
                    match (a, b) {
                        (
                            Piece::Line { a: a1, b: b1 },
                            Piece::Line { a: a2, b: b2 },
                        ) => {
                            let (d1, d2) = (b1 - a1, b2 - a2);
                            let den = d1.perp_dot(d2);
                            if den.is_nan()
                                || den.abs() <= 1e-9 * d1.length() * d2.length()
                            {
                                None
                            } else {
                                Some(a1 + d1 * ((a2 - a1).perp_dot(d2) / den))
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                match joint {
                    Some(p) => {
                        let next = (i + 1) % offset.len();
                        if let Piece::Line { b: b1, .. } = &mut offset[i] {
                            *b1 = p;
                        }
                        if let Piece::Line { a: a2, .. } = &mut offset[next] {
                            *a2 = p;
                        }
                    }
                    None => {
                        // The roll runs round the original vertex, which the offset piece's end
                        // sits one offset away from along its own normal.
                        let dir_a = a.tangent_at(1.0);
                        let vertex = a.end() - dir_a.perp() * d;
                        let n = dir_a.perp() * d.signum();
                        rolls.push((
                            (i + 1).min(offset.len()),
                            Piece::Arc {
                                c: vertex,
                                r: ad,
                                a0: angle_of(n),
                                sweep: bend,
                            },
                        ));
                    }
                }
            }
            for (at, piece) in rolls.into_iter().rev() {
                offset.insert(at, piece);
            }
            raw.extend(offset.into_iter().map(|p| (p, contour.z)));
        }
        if raw.len() > MAX_SPLIT_PIECES {
            return Vec::new();
        }

        let boxes: Vec<(Point, Point)> =
            raw.iter().map(|(p, _)| p.bounds()).collect();
        let mut cuts: Vec<Vec<f64>> = vec![Vec::new(); raw.len()];
        let mut hits = Vec::new();
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
                a.0.intersections(&b.0, &mut hits);
                for &(t, u) in &hits {
                    if let Some(c) = cuts.get_mut(i) {
                        c.push(t);
                    }
                    if let Some(c) = cuts.get_mut(j) {
                        c.push(u);
                    }
                }
            }
        }

        let mut kept: Vec<(Piece, f64)> = Vec::new();
        let mut total = 0usize;
        for ((piece, z), ts) in raw.iter().zip(cuts.iter_mut()) {
            let len = piece.length();
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
                let sub = piece.sub(t0, t);
                let mid = sub.point_at(0.5);
                if self.dist(mid) >= ad - CLEAR_TOL
                    && self.inside(mid, ad) == (d > 0.0)
                {
                    kept.push((sub, *z));
                }
                t0 = t;
            }
        }

        let join_tol = 4.0 * min_len;
        stitch(&kept, join_tol)
            .into_iter()
            .filter(|(lp, _)| {
                let (area, length) = lp.iter().fold((0.0, 0.0), |(a, l), p| {
                    (a + p.area_term(), l + p.length())
                });
                area.abs() > join_tol * length
            })
            .collect()
    }
}

/// Joins pieces end to start into closed loops. Where several pieces leave a point the
/// left-most one is taken: the kept side of every piece is on its left, so the boundary of
/// what is kept turns left there. Chains that do not close are dropped.
fn stitch(pieces: &[(Piece, f64)], tol: f64) -> Vec<(Vec<Piece>, f64)> {
    let mut used = vec![false; pieces.len()];
    let mut loops = Vec::new();
    for (s, (first, z)) in pieces.iter().enumerate() {
        if std::mem::replace(used.get_mut(s).unwrap_or(&mut true), true) {
            continue;
        }
        let origin = first.start();
        let mut chain = vec![*first];
        let mut closed = false;
        for _ in 0..pieces.len() {
            let Some(last) = chain.last() else { break };
            let (end, dir) = (last.end(), last.tangent_at(1.0));
            if end.distance(origin) <= tol
                && (chain.len() > 1 || last.length() > 2.0 * tol)
            {
                closed = true;
                break;
            }
            let mut best: Option<(usize, f64)> = None;
            for (j, ((c, _), taken)) in pieces.iter().zip(&used).enumerate() {
                if *taken || c.start().distance(end) > tol {
                    continue;
                }
                let bend = turn(dir, c.tangent_at(0.0));
                if best.is_none_or(|(_, b)| bend > b) {
                    best = Some((j, bend));
                }
            }
            let Some((j, _)) = best else { break };
            if let (Some(flag), Some((piece, _))) =
                (used.get_mut(j), pieces.get(j))
            {
                *flag = true;
                chain.push(*piece);
            }
        }
        if closed {
            loops.push((chain, *z));
        }
    }
    loops
}

/// A kept loop as a geometry: one `move_to`, then line and exact arc commands round the loop.
fn loop_geometry(pieces: &[Piece], z: f64) -> Geometry {
    let mut g = Geometry::new();
    let Some(first) = pieces.first() else {
        return g;
    };
    let start = first.start();
    g.move_to(start.x, start.y, z);
    let mut current = start;
    for piece in pieces {
        let end = piece.end();
        match *piece {
            Piece::Line { b, .. } => {
                g.line_to(b.x, b.y, z);
            }
            Piece::Arc { c, sweep, .. } => {
                g.arc_to(
                    end.x,
                    end.y,
                    c.x - current.x,
                    c.y - current.y,
                    sweep < 0.0,
                    z,
                );
            }
        }
        current = end;
    }
    if current.distance(start) > CLOSED_TOL {
        g.close_path();
    }
    g
}

/// The boundaries of `geometry` moved by `offset` (positive: outwards, so the region grows;
/// negative: into it), as closed loops of lines and arcs.
///
/// Every closed subpath of the geometry takes part: counter-clockwise loops as material,
/// clockwise ones as holes, whatever the nesting. Lines and arcs stay exact; other curves are
/// followed as line chains within `tolerance`. Open subpaths are ignored. Degenerate input, a
/// non-finite or zero `offset`, or more pieces than the module's internal limits return an
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
    let total: usize = loops.iter().map(|c| c.pieces.len()).sum();
    if total > MAX_PIECES {
        return Vec::new();
    }
    let boundary = Boundary { loops };
    // Positive grows the region; the core moves the boundary into the region for a positive d.
    boundary
        .offset(-offset, tolerance)
        .into_iter()
        .map(|(pieces, z)| loop_geometry(&pieces, z))
        .collect()
}
