use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::arc::get_arc_sweep;

/// A 2D point represented as (x, y) coordinates.
pub type Point = (f64, f64);

/// A 3D point represented as (x, y, z) coordinates.
pub type Point3D = (f64, f64, f64);

/// A 2D axis-aligned bounding box represented as (x_min, y_min, x_max, y_max).
pub type Rect = (f64, f64, f64, f64);

/// A cubic Bezier curve defined by four control points: (p0, c1, c2, p1).
/// - p0: Start point
/// - c1: First control point
/// - c2: Second control point
/// - p1: End point
pub type CubicBezier = (Point, Point, Point, Point);

/// Control points for a cubic Bezier curve: (c1, c2, p1).
/// c1 and c2 are the control points, p1 is the end point.
pub type BezierControls = (Point, Point, Point);

/// Result of splitting a cubic Bezier curve: (first_half, second_half).
pub type BezierSplit = (CubicBezier, CubicBezier);

/// A pair of geometry vectors: (inner_contours, outer_contours).
pub type GeometryPair<T> = (Vec<T>, Vec<T>);

/// A 2D polygon represented as a list of vertices in order.
pub type Polygon = Vec<Point>;

/// A 3D polygon represented as a list of 3D vertices in order.
pub type Polygon3D = Vec<Point3D>;

/// An edge represented as a pair of points: (start, end).
pub type Edge = (Point, Point);

/// A line segment in 3D space represented as (start, end).
pub type Segment3D = (Point3D, Point3D);

/// Typed view over a single `[f64; 8]` geometry command row.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Move {
        end: Point3D,
    },
    Line {
        end: Point3D,
    },
    Arc {
        end: Point3D,
        center_offset: Point,
        clockwise: bool,
    },
    Bezier {
        end: Point3D,
        control1: Point,
        control2: Point,
    },
}

impl Command {
    pub fn end_point(&self) -> Point3D {
        match self {
            Command::Move { end } => *end,
            Command::Line { end } => *end,
            Command::Arc { end, .. } => *end,
            Command::Bezier { end, .. } => *end,
        }
    }

    pub fn length(&self, start_point: Point3D) -> f64 {
        match self {
            Command::Move { .. } | Command::Line { .. } => {
                let end = self.end_point();
                (end.0 - start_point.0).hypot(end.1 - start_point.1)
            }
            Command::Arc {
                end,
                center_offset,
                clockwise,
            } => {
                let cx = start_point.0 + center_offset.0;
                let cy = start_point.1 + center_offset.1;
                let radius = center_offset.0.hypot(center_offset.1);
                if radius < EPSILON_COLLINEAR {
                    return 0.0;
                }
                let start_angle =
                    (start_point.1 - cy).atan2(start_point.0 - cx);
                let end_angle = (end.1 - cy).atan2(end.0 - cx);
                let angle_span =
                    get_arc_sweep(start_angle, end_angle, *clockwise);
                (angle_span * radius).abs()
            }
            Command::Bezier {
                end,
                control1,
                control2,
            } => {
                let sx = start_point.0;
                let sy = start_point.1;
                let ex = end.0;
                let ey = end.1;
                let l01 = (sx - control1.0).hypot(sy - control1.1);
                let l12 =
                    (control1.0 - control2.0).hypot(control1.1 - control2.1);
                let l23 = (control2.0 - ex).hypot(control2.1 - ey);
                let estimated_len = l01 + l12 + l23;
                let num_steps = (estimated_len / 0.1).ceil().max(2.0) as usize;
                let step_f = num_steps as f64;
                let mut total = 0.0;
                let mut prev = (sx, sy);
                for i in 1..=num_steps {
                    let t = i as f64 / step_f;
                    let omt = 1.0 - t;
                    let px = omt.powi(3) * sx
                        + 3.0 * omt.powi(2) * t * control1.0
                        + 3.0 * omt * t.powi(2) * control2.0
                        + t.powi(3) * ex;
                    let py = omt.powi(3) * sy
                        + 3.0 * omt.powi(2) * t * control1.1
                        + 3.0 * omt * t.powi(2) * control2.1
                        + t.powi(3) * ey;
                    total += (px - prev.0).hypot(py - prev.1);
                    prev = (px, py);
                }
                total
            }
        }
    }

    pub fn split_at_t(&self, start_point: Point3D, t: f64) -> Option<Command> {
        let sx = start_point.0;
        let sy = start_point.1;
        let sz = start_point.2;
        let end = self.end_point();
        let ex = end.0;
        let ey = end.1;
        let ez = end.2;
        match self {
            Command::Line { .. } => {
                let nx = sx + t * (ex - sx);
                let ny = sy + t * (ey - sy);
                let nz = sz + t * (ez - sz);
                Some(Command::Line { end: (nx, ny, nz) })
            }
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => {
                let i_off = center_offset.0;
                let j_off = center_offset.1;
                let cx = sx + i_off;
                let cy = sy + j_off;
                let radius_start = i_off.hypot(j_off);
                let radius_end = (ex - cx).hypot(ey - cy);
                let start_angle = (sy - cy).atan2(sx - cx);
                let end_angle = (ey - cy).atan2(ex - cx);
                let angle_span =
                    get_arc_sweep(start_angle, end_angle, *clockwise);
                let mid_angle = start_angle + t * angle_span;
                let radius = radius_start + t * (radius_end - radius_start);
                let nx = cx + radius * mid_angle.cos();
                let ny = cy + radius * mid_angle.sin();
                let nz = sz + t * (ez - sz);
                Some(Command::Arc {
                    end: (nx, ny, nz),
                    center_offset: (i_off, j_off),
                    clockwise: *clockwise,
                })
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let c1x = control1.0;
                let c1y = control1.1;
                let c2x = control2.0;
                let c2y = control2.1;
                let p01x = sx + t * (c1x - sx);
                let p01y = sy + t * (c1y - sy);
                let p12x = c1x + t * (c2x - c1x);
                let p12y = c1y + t * (c2y - c1y);
                let p23x = c2x + t * (ex - c2x);
                let p23y = c2y + t * (ey - c2y);
                let p012x = p01x + t * (p12x - p01x);
                let p012y = p01y + t * (p12y - p01y);
                let p123x = p12x + t * (p23x - p12x);
                let p123y = p12y + t * (p23y - p12y);
                let p0123x = p012x + t * (p123x - p012x);
                let p0123y = p012y + t * (p123y - p012y);
                let nz = sz + t * (ez - sz);
                Some(Command::Bezier {
                    end: (p0123x, p0123y, nz),
                    control1: (p01x, p01y),
                    control2: (p012x, p012y),
                })
            }
            Command::Move { .. } => None,
        }
    }
}

/// A 2D integer point for grid-based operations.
pub type IntPoint = (i64, i64);

/// A 2D integer polygon for grid-based operations.
pub type IntPolygon = Vec<IntPoint>;

/// A 3D axis-aligned bounding box with separate min/max bounds for each axis.
#[derive(Clone, Debug, Default)]
pub struct Rect3D {
    /// Minimum x coordinate (left face).
    pub x_min: f64,
    /// Maximum x coordinate (right face).
    pub x_max: f64,
    /// Minimum y coordinate (bottom face).
    pub y_min: f64,
    /// Maximum y coordinate (top face).
    pub y_max: f64,
    /// Minimum z coordinate (front face).
    pub z_min: f64,
    /// Maximum z coordinate (back face).
    pub z_max: f64,
}

/// The winding order of a closed polygon or path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindingOrder {
    /// Clockwise winding.
    CW,
    /// Counter-clockwise winding.
    CCW,
}

/// Container for contour/path data with geometric and topological information.
#[derive(Clone, Debug)]
pub struct ContourData {
    /// The geometric data of the contour.
    pub geo: super::Geometry,
    /// Whether the contour forms a closed path.
    pub is_closed: bool,
    /// List of vertices defining the contour.
    pub vertices: Polygon,
    /// The signed area of the contour (positive for CCW, negative for CW).
    pub area: f64,
    /// Winding order of the contour.
    pub winding_order: Option<WindingOrder>,
}
