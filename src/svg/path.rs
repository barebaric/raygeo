use std::f64::consts::PI;

use crate::geo::geometry::Geometry;
use crate::geo::matrix::Matrix;
use crate::geo::shape::bezier::evaluate_cubic;
use crate::geo::types::Point3D;
use crate::svg::arc::{convert_arc_to_beziers, get_arc_center};

/// Apply the affine transform + scale to a single SVG coordinate.
fn txfm_pt(px: f64, py: f64, m: &Matrix, sx: f64, sy: f64) -> (f64, f64) {
    let (tx, ty) = m.transform_point(px, py);
    (tx / sx, ty / sy)
}

/// Transform an arc's center offset and determine if direction flips.
fn txfm_arc(
    start: (f64, f64),
    center: (f64, f64),
    clockwise: bool,
    m: &Matrix,
    sx: f64,
    sy: f64,
) -> (f64, f64, f64, f64, bool) {
    let (tsx, tsy) = txfm_pt(start.0, start.1, m, sx, sy);
    let (tex, tey) = txfm_pt(center.0, center.1, m, sx, sy);
    let ti = tex - tsx;
    let tj = tey - tsy;
    let det = m.determinant_2x2();
    let cw = if (det < 0.0) != (sx * sy < 0.0) {
        !clockwise
    } else {
        clockwise
    };
    (tex, tey, ti, tj, cw)
}

/// Sample N evenly-spaced points on a cubic bezier (used by C, S, Q, T).
fn flatten_cubic(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    n: usize,
) -> Vec<(f64, f64)> {
    (1..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            (
                evaluate_cubic(p0.0, p1.0, p2.0, p3.0, t),
                evaluate_cubic(p0.1, p1.1, p2.1, p3.1, t),
            )
        })
        .collect()
}

fn push_line(geo: &mut Geometry, x: f64, y: f64, m: &Matrix, sx: f64, sy: f64) {
    let (tx, ty) = txfm_pt(x, y, m, sx, sy);
    geo.line_to(tx, ty, 0.0);
}

/// Mutable state accumulated while parsing an SVG path `d` string.
pub(crate) struct PathBuildContext<'a> {
    pos: (f64, f64),
    subpath_start: (f64, f64),
    prev_c2: Option<(f64, f64)>,
    prev_q: Option<(f64, f64)>,
    current_geo: Option<Geometry>,
    geometries: Vec<Geometry>,
    transform: &'a Matrix,
    scale_x: f64,
    scale_y: f64,
}

impl<'a> PathBuildContext<'a> {
    pub(crate) fn new(
        transform: &'a Matrix,
        scale_x: f64,
        scale_y: f64,
    ) -> Self {
        Self {
            pos: (0.0, 0.0),
            subpath_start: (0.0, 0.0),
            prev_c2: None,
            prev_q: None,
            current_geo: None,
            geometries: Vec::new(),
            transform,
            scale_x,
            scale_y,
        }
    }

    fn flush_current(&mut self) {
        self.prev_c2 = None;
        self.prev_q = None;
        if let Some(g) = self.current_geo.take() {
            if !g.is_empty() {
                self.geometries.push(g);
            }
        }
    }

    fn resolve_pos(&self, abs: bool, x: f64, y: f64) -> (f64, f64) {
        if abs {
            (x, y)
        } else {
            (self.pos.0 + x, self.pos.1 + y)
        }
    }

    fn push_flattened(&mut self, points: &[(f64, f64)], end: (f64, f64)) {
        self.pos = end;
        if let Some(ref mut g) = self.current_geo {
            for &(px, py) in points {
                push_line(
                    g,
                    px,
                    py,
                    self.transform,
                    self.scale_x,
                    self.scale_y,
                );
            }
        }
    }

    pub(crate) fn handle_moveto(&mut self, abs: bool, x: f64, y: f64) {
        self.flush_current();
        let pos = self.resolve_pos(abs, x, y);
        self.pos = pos;
        self.subpath_start = pos;
        let mut g = Geometry::new();
        let (tx, ty) =
            txfm_pt(pos.0, pos.1, self.transform, self.scale_x, self.scale_y);
        g.move_to(tx, ty, 0.0);
        self.current_geo = Some(g);
    }

    pub(crate) fn handle_lineto(&mut self, abs: bool, x: f64, y: f64) {
        let pos = self.resolve_pos(abs, x, y);
        self.prev_c2 = None;
        self.prev_q = None;
        self.pos = pos;
        if let Some(ref mut g) = self.current_geo {
            push_line(
                g,
                pos.0,
                pos.1,
                self.transform,
                self.scale_x,
                self.scale_y,
            );
        }
    }

    pub(crate) fn handle_hline_to(&mut self, abs: bool, x: f64) {
        self.prev_c2 = None;
        self.prev_q = None;
        if abs {
            self.pos.0 = x;
        } else {
            self.pos.0 += x;
        }
        if let Some(ref mut g) = self.current_geo {
            push_line(
                g,
                self.pos.0,
                self.pos.1,
                self.transform,
                self.scale_x,
                self.scale_y,
            );
        }
    }

    pub(crate) fn handle_vline_to(&mut self, abs: bool, y: f64) {
        self.prev_c2 = None;
        self.prev_q = None;
        if abs {
            self.pos.1 = y;
        } else {
            self.pos.1 += y;
        }
        if let Some(ref mut g) = self.current_geo {
            push_line(
                g,
                self.pos.0,
                self.pos.1,
                self.transform,
                self.scale_x,
                self.scale_y,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_cubic_to(
        &mut self,
        abs: bool,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    ) {
        self.prev_q = None;
        let c1 = self.resolve_pos(abs, x1, y1);
        let c2 = self.resolve_pos(abs, x2, y2);
        let end = self.resolve_pos(abs, x, y);
        self.prev_c2 = Some(c2);
        let pts = flatten_cubic(self.pos, c1, c2, end, 20);
        self.push_flattened(&pts, end);
    }

    pub(crate) fn handle_smooth_cubic_to(
        &mut self,
        abs: bool,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    ) {
        self.prev_q = None;
        let c2 = self.resolve_pos(abs, x2, y2);
        let end = self.resolve_pos(abs, x, y);
        let c1 = match self.prev_c2 {
            Some((px, py)) => (2.0 * self.pos.0 - px, 2.0 * self.pos.1 - py),
            None => self.pos,
        };
        self.prev_c2 = Some(c2);
        let pts = flatten_cubic(self.pos, c1, c2, end, 20);
        self.push_flattened(&pts, end);
    }

    pub(crate) fn handle_quadratic(
        &mut self,
        abs: bool,
        x1: f64,
        y1: f64,
        x: f64,
        y: f64,
    ) {
        self.prev_c2 = None;
        let q = self.resolve_pos(abs, x1, y1);
        let end = self.resolve_pos(abs, x, y);
        self.prev_q = Some(q);
        let c1 = (
            self.pos.0 + 2.0 / 3.0 * (q.0 - self.pos.0),
            self.pos.1 + 2.0 / 3.0 * (q.1 - self.pos.1),
        );
        let c2 = (
            q.0 + 1.0 / 3.0 * (end.0 - q.0),
            q.1 + 1.0 / 3.0 * (end.1 - q.1),
        );
        let pts = flatten_cubic(self.pos, c1, c2, end, 20);
        self.push_flattened(&pts, end);
    }

    pub(crate) fn handle_smooth_quadratic(
        &mut self,
        abs: bool,
        x: f64,
        y: f64,
    ) {
        self.prev_c2 = None;
        let end = self.resolve_pos(abs, x, y);
        let q = match self.prev_q {
            Some((px, py)) => (2.0 * self.pos.0 - px, 2.0 * self.pos.1 - py),
            None => self.pos,
        };
        self.prev_q = Some(q);
        let c1 = (
            self.pos.0 + 2.0 / 3.0 * (q.0 - self.pos.0),
            self.pos.1 + 2.0 / 3.0 * (q.1 - self.pos.1),
        );
        let c2 = (
            q.0 + 1.0 / 3.0 * (end.0 - q.0),
            q.1 + 1.0 / 3.0 * (end.1 - q.1),
        );
        let pts = flatten_cubic(self.pos, c1, c2, end, 20);
        self.push_flattened(&pts, end);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_elliptical_arc(
        &mut self,
        abs: bool,
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64,
        y: f64,
    ) {
        self.prev_c2 = None;
        self.prev_q = None;
        let end = self.resolve_pos(abs, x, y);
        let ac = match get_arc_center(
            self.pos.0,
            self.pos.1,
            rx,
            ry,
            x_axis_rotation,
            large_arc,
            sweep,
            end.0,
            end.1,
        ) {
            Some(ac) => ac,
            None => {
                self.pos = end;
                return;
            }
        };
        let is_circular =
            (ac.radii.0 - ac.radii.1).abs() / ac.radii.0.max(ac.radii.1) < 1e-3;
        let is_short_arc = ac.sweep.abs() <= PI + 1e-9;
        if let Some(ref mut g) = self.current_geo {
            if is_circular && is_short_arc {
                let (_cx, _cy, ti, tj, cw) = txfm_arc(
                    self.pos,
                    (ac.center.x, ac.center.y),
                    !sweep,
                    self.transform,
                    self.scale_x,
                    self.scale_y,
                );
                let (end_x, end_y) = txfm_pt(
                    end.0,
                    end.1,
                    self.transform,
                    self.scale_x,
                    self.scale_y,
                );
                g.arc_to(end_x, end_y, ti, tj, cw, 0.0);
            } else {
                for seg in convert_arc_to_beziers(&ac) {
                    let (tc1x, tc1y) = txfm_pt(
                        seg.1.x,
                        seg.1.y,
                        self.transform,
                        self.scale_x,
                        self.scale_y,
                    );
                    let (tc2x, tc2y) = txfm_pt(
                        seg.2.x,
                        seg.2.y,
                        self.transform,
                        self.scale_x,
                        self.scale_y,
                    );
                    let (tex, tey) = txfm_pt(
                        seg.3.x,
                        seg.3.y,
                        self.transform,
                        self.scale_x,
                        self.scale_y,
                    );
                    g.bezier_to(
                        Point3D::new(tc1x, tc1y, 0.0),
                        Point3D::new(tc2x, tc2y, 0.0),
                        Point3D::new(tex, tey, 0.0),
                    );
                }
            }
        }
        self.pos = end;
    }

    pub(crate) fn handle_close_path(&mut self) {
        self.prev_c2 = None;
        self.prev_q = None;
        if let Some(ref mut g) = self.current_geo {
            g.close_path();
        }
        self.pos = self.subpath_start;
    }

    pub(crate) fn finish(self) -> Vec<Geometry> {
        let mut geometries = self.geometries;
        if let Some(geo) = self.current_geo {
            if !geo.is_empty() {
                geometries.push(geo);
            }
        }
        geometries
    }
}
