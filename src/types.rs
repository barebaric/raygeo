use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::arc::{
    get_arc_bounds, get_arc_closest_point, get_arc_sweep_3d, linearize_arc,
};
use crate::geo::shape::bezier::{
    get_bezier_bounds, get_bezier_closest_point, linearize_bezier_from_params,
};
use crate::geo::shape::line::get_line_segment_closest_point;

/// A 2D point represented as (x, y) coordinates.
pub type Point = glam::DVec2;

/// Project a 3D point to the XY plane, dropping Z.
pub fn project_point_to_xy(p: Point3D) -> Point {
    Point::new(p.x, p.y)
}

/// Project a slice of 3D points to the XY plane, dropping Z.
pub fn project_points_to_xy(points: &[Point3D]) -> Vec<Point> {
    points.iter().map(|p| Point::new(p.x, p.y)).collect()
}

/// Lift 2D points to the XY plane at a given Z height.
pub fn lift_points_to_xy_plane(points: &[Point], z: f64) -> Vec<Point3D> {
    points.iter().map(|p| Point3D::new(p.x, p.y, z)).collect()
}

/// Check whether all points share the same Z (within tolerance).
/// Returns `Some(z)` if planar in Z, `None` otherwise.
pub fn is_planar_in_z(points: &[Point3D], tol: f64) -> Option<f64> {
    let z0 = points.first()?.z;
    if points.iter().all(|p| (p.z - z0).abs() <= tol) {
        Some(z0)
    } else {
        None
    }
}

/// A 3D point represented as (x, y, z) coordinates.
pub type Point3D = glam::DVec3;

/// A 2D axis-aligned bounding box represented as (x_min, y_min, x_max, y_max).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect(pub f64, pub f64, pub f64, pub f64);

/// A cubic Bezier curve defined by four control points: (p0, c1, c2, p1).
/// - p0: Start point
/// - c1: First control point
/// - c2: Second control point
/// - p1: End point
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier(pub Point, pub Point, pub Point, pub Point);

/// Control points for a cubic Bezier curve: (c1, c2, p1).
/// c1 and c2 are the control points, p1 is the end point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BezierControls(pub Point, pub Point, pub Point);

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
        center_offset: Point3D,
        normal: Point3D,
    },
    Bezier {
        end: Point3D,
        control1: Point3D,
        control2: Point3D,
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
                (end.x - start_point.x).hypot(end.y - start_point.y)
            }
            Command::Arc {
                end,
                center_offset,
                normal,
            } => {
                // Use 2D projection for length (XY-plane arcs; for true 3D
                // arcs this falls back to the 2D sweep in the arc's plane).
                let n =
                    glam::DVec3::new(normal.x, normal.y, normal.z).normalize();
                if n.length() < 1e-30 {
                    return (end.x - start_point.x)
                        .hypot(end.y - start_point.y);
                }
                let center = glam::DVec3::new(
                    start_point.x + center_offset.x,
                    start_point.y + center_offset.y,
                    start_point.z + center_offset.z,
                );
                let r0 = glam::DVec3::new(
                    start_point.x - center.x,
                    start_point.y - center.y,
                    start_point.z - center.z,
                );
                let r1 = glam::DVec3::new(
                    end.x - center.x,
                    end.y - center.y,
                    end.z - center.z,
                );
                let r0_proj = r0 - n * r0.dot(n);
                let radius = r0_proj.length();
                if radius < EPSILON_COLLINEAR {
                    return 0.0;
                }
                let u = r0_proj / radius;
                let v = n.cross(u);
                let theta_end = f64::atan2(r1.dot(v), r1.dot(u));
                let sweep = if theta_end.abs() < EPSILON_COLLINEAR {
                    2.0 * std::f64::consts::PI
                } else if theta_end < 0.0 {
                    theta_end + 2.0 * std::f64::consts::PI
                } else {
                    theta_end
                };
                (sweep * radius).abs()
            }
            Command::Bezier {
                end,
                control1,
                control2,
            } => {
                let sx = start_point.x;
                let sy = start_point.y;
                let ex = end.x;
                let ey = end.y;
                let l01 = (sx - control1.x).hypot(sy - control1.y);
                let l12 =
                    (control1.x - control2.x).hypot(control1.y - control2.y);
                let l23 = (control2.x - ex).hypot(control2.y - ey);
                let estimated_len = l01 + l12 + l23;
                let num_steps = (estimated_len / 0.1).ceil().max(2.0) as usize;
                let step_f = num_steps as f64;
                let mut total = 0.0;
                let mut prev = (sx, sy);
                for i in 1..=num_steps {
                    let t = i as f64 / step_f;
                    let omt = 1.0 - t;
                    let px = omt.powi(3) * sx
                        + 3.0 * omt.powi(2) * t * control1.x
                        + 3.0 * omt * t.powi(2) * control2.x
                        + t.powi(3) * ex;
                    let py = omt.powi(3) * sy
                        + 3.0 * omt.powi(2) * t * control1.y
                        + 3.0 * omt * t.powi(2) * control2.y
                        + t.powi(3) * ey;
                    total += (px - prev.0).hypot(py - prev.1);
                    prev = (px, py);
                }
                total
            }
        }
    }

    pub fn split_at_t(&self, start_point: Point3D, t: f64) -> Option<Command> {
        let sx = start_point.x;
        let sy = start_point.y;
        let sz = start_point.z;
        let end = self.end_point();
        let ex = end.x;
        let ey = end.y;
        let ez = end.z;
        match self {
            Command::Line { .. } => {
                let nx = sx + t * (ex - sx);
                let ny = sy + t * (ey - sy);
                let nz = sz + t * (ez - sz);
                Some(Command::Line {
                    end: Point3D::new(nx, ny, nz),
                })
            }
            Command::Arc {
                center_offset,
                normal,
                ..
            } => {
                let n =
                    glam::DVec3::new(normal.x, normal.y, normal.z).normalize();
                let center = glam::DVec3::new(
                    sx + center_offset.x,
                    sy + center_offset.y,
                    sz + center_offset.z,
                );
                let r0 = glam::DVec3::new(
                    sx - center.x,
                    sy - center.y,
                    sz - center.z,
                );
                let r1 = glam::DVec3::new(
                    ex - center.x,
                    ey - center.y,
                    ez - center.z,
                );
                let r0_proj = r0 - n * r0.dot(n);
                let r1_proj = r1 - n * r1.dot(n);
                let (nx, ny) = if r0_proj.length() < EPSILON_COLLINEAR {
                    // Degenerate start radius — linear interpolation
                    (sx + t * (ex - sx), sy + t * (ey - sy))
                } else {
                    let u = r0_proj.normalize();
                    let v = n.cross(u).normalize();
                    let theta_end = f64::atan2(r1_proj.dot(v), r1_proj.dot(u));
                    let sweep = if theta_end.abs() < EPSILON_COLLINEAR {
                        2.0 * std::f64::consts::PI
                    } else if theta_end < 0.0 {
                        theta_end + 2.0 * std::f64::consts::PI
                    } else {
                        theta_end
                    };
                    let angle = sweep * t;
                    let radius = r0_proj.length()
                        + (r1_proj.length() - r0_proj.length()) * t;
                    let (s, c) = angle.sin_cos();
                    let dir = u * (radius * c) + v * (radius * s);
                    let pt = center + dir;
                    (pt.x, pt.y)
                };
                let nz = sz + t * (ez - sz);
                Some(Command::Arc {
                    end: Point3D::new(nx, ny, nz),
                    center_offset: *center_offset,
                    normal: *normal,
                })
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let c1x = control1.x;
                let c1y = control1.y;
                let c1z = control1.z;
                let c2x = control2.x;
                let c2y = control2.y;
                let c2z = control2.z;
                let p01x = sx + t * (c1x - sx);
                let p01y = sy + t * (c1y - sy);
                let p01z = sz + t * (c1z - sz);
                let p12x = c1x + t * (c2x - c1x);
                let p12y = c1y + t * (c2y - c1y);
                let p12z = c1z + t * (c2z - c1z);
                let p23x = c2x + t * (ex - c2x);
                let p23y = c2y + t * (ey - c2y);
                let p23z = c2z + t * (ez - c2z);
                let p012x = p01x + t * (p12x - p01x);
                let p012y = p01y + t * (p12y - p01y);
                let p012z = p01z + t * (p12z - p01z);
                let p123x = p12x + t * (p23x - p12x);
                let p123y = p12y + t * (p23y - p12y);
                let p123z = p12z + t * (p23z - p12z);
                let p0123x = p012x + t * (p123x - p012x);
                let p0123y = p012y + t * (p123y - p012y);
                let p0123z = p012z + t * (p123z - p012z);
                Some(Command::Bezier {
                    end: Point3D::new(p0123x, p0123y, p0123z),
                    control1: Point3D::new(p01x, p01y, p01z),
                    control2: Point3D::new(p012x, p012y, p012z),
                })
            }
            Command::Move { .. } => None,
        }
    }

    pub fn point_at(&self, start: Point3D, t: f64) -> Option<Point3D> {
        let p0 = start;
        let p1 = self.end_point();
        let (px, py) = match self {
            Command::Move { .. } => return None,
            Command::Line { .. } => {
                let px = p0.x + t * (p1.x - p0.x);
                let py = p0.y + t * (p1.y - p0.y);
                (px, py)
            }
            Command::Arc {
                center_offset,
                normal,
                ..
            } => {
                let n =
                    glam::DVec3::new(normal.x, normal.y, normal.z).normalize();
                let center = glam::DVec3::new(
                    p0.x + center_offset.x,
                    p0.y + center_offset.y,
                    p0.z + center_offset.z,
                );
                let r0 = glam::DVec3::new(
                    p0.x - center.x,
                    p0.y - center.y,
                    p0.z - center.z,
                );
                let r1 = glam::DVec3::new(
                    p1.x - center.x,
                    p1.y - center.y,
                    p1.z - center.z,
                );
                let r0_proj = r0 - n * r0.dot(n);
                let r1_proj = r1 - n * r1.dot(n);
                if r0_proj.length() < EPSILON_COLLINEAR
                    && r1_proj.length() < EPSILON_COLLINEAR
                {
                    (p0.x, p0.y)
                } else {
                    let (u, v, r_start, r_end) =
                        if r0_proj.length() < EPSILON_COLLINEAR {
                            // Degenerate start — use an arbitrary reference in the plane
                            let u_ref = if n.x.abs() < 0.9 {
                                (glam::DVec3::X - n * n.x).normalize()
                            } else {
                                (glam::DVec3::Y - n * n.y).normalize()
                            };
                            let v_ref = n.cross(u_ref).normalize();
                            (u_ref, v_ref, 0.0, r1_proj.length())
                        } else {
                            let u = r0_proj.normalize();
                            let v = n.cross(u).normalize();
                            (u, v, r0_proj.length(), r1_proj.length())
                        };
                    let theta_end = f64::atan2(r1_proj.dot(v), r1_proj.dot(u));
                    let sweep = get_arc_sweep_3d(0.0, theta_end);
                    let angle = sweep * t;
                    let radius = r_start + (r_end - r_start) * t;
                    let (s, c) = angle.sin_cos();
                    let dir = u * (radius * c) + v * (radius * s);
                    let pt = center + dir;
                    (pt.x, pt.y)
                }
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let c1 = (control1.x, control1.y);
                let c2 = (control2.x, control2.y);
                let omt = 1.0 - t;
                let px = omt.powi(3) * p0.x
                    + 3.0 * omt.powi(2) * t * c1.0
                    + 3.0 * omt * t.powi(2) * c2.0
                    + t.powi(3) * p1.x;
                let py = omt.powi(3) * p0.y
                    + 3.0 * omt.powi(2) * t * c1.1
                    + 3.0 * omt * t.powi(2) * c2.1
                    + t.powi(3) * p1.y;
                (px, py)
            }
        };
        let pz = p0.z + t * (p1.z - p0.z);
        Some(Point3D::new(px, py, pz))
    }

    pub fn tangent_at(&self, start: Point3D, t: f64) -> Option<Point> {
        let p0 = Point::new(start.x, start.y);
        let p1_pt = self.end_point();
        let p1 = Point::new(p1_pt.x, p1_pt.y);

        let tangent_vec: Point = match self {
            Command::Move { .. } => return None,
            Command::Line { .. } => Point::new(p1.x - p0.x, p1.y - p0.y),
            Command::Arc {
                center_offset,
                normal,
                ..
            } => {
                // Use the original 3D start point for the arc math, then
                // project the resulting tangent into the XY plane.
                let s3 = start;
                let n =
                    glam::DVec3::new(normal.x, normal.y, normal.z).normalize();
                let center = glam::DVec3::new(
                    s3.x + center_offset.x,
                    s3.y + center_offset.y,
                    s3.z + center_offset.z,
                );
                let r0 = glam::DVec3::new(
                    s3.x - center.x,
                    s3.y - center.y,
                    s3.z - center.z,
                );
                let r1 = glam::DVec3::new(
                    p1_pt.x - center.x,
                    p1_pt.y - center.y,
                    p1_pt.z - center.z,
                );
                let r0_proj = r0 - n * r0.dot(n);
                let r1_proj = r1 - n * r1.dot(n);
                let (u, v, r_start, r_end) =
                    if r0_proj.length() < EPSILON_COLLINEAR {
                        if r1_proj.length() < EPSILON_COLLINEAR {
                            return Some(Point::new(1.0, 0.0));
                        }
                        let u_ref = if n.x.abs() < 0.9 {
                            (glam::DVec3::X - n * n.x).normalize()
                        } else {
                            (glam::DVec3::Y - n * n.y).normalize()
                        };
                        let v_ref = n.cross(u_ref).normalize();
                        (u_ref, v_ref, 0.0, r1_proj.length())
                    } else {
                        let u = r0_proj.normalize();
                        let v = n.cross(u).normalize();
                        (u, v, r0_proj.length(), r1_proj.length())
                    };
                let theta_end = f64::atan2(r1_proj.dot(v), r1_proj.dot(u));
                let sweep = get_arc_sweep_3d(0.0, theta_end);
                let angle = sweep * t;
                let radius = r_start + (r_end - r_start) * t;
                let (s, c) = angle.sin_cos();
                let dir = u * (radius * c) + v * (radius * s);
                let pt = center + dir;
                let radius_vec = pt - center;
                let tangent = n.cross(radius_vec);
                Point::new(tangent.x, tangent.y)
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let c1 = (control1.x, control1.y);
                let c2 = (control2.x, control2.y);
                let omt = 1.0 - t;
                let tx = 3.0 * omt.powi(2) * (c1.0 - p0.x)
                    + 6.0 * omt * t * (c2.0 - c1.0)
                    + 3.0 * t.powi(2) * (p1.x - c2.0);
                let ty = 3.0 * omt.powi(2) * (c1.1 - p0.y)
                    + 6.0 * omt * t * (c2.1 - c1.1)
                    + 3.0 * t.powi(2) * (p1.y - c2.1);
                Point::new(tx, ty)
            }
        };

        let norm = (tangent_vec.x.powi(2) + tangent_vec.y.powi(2)).sqrt();
        if norm < 1e-9 {
            return Some(Point::new(1.0, 0.0));
        }
        Some(Point::new(tangent_vec.x / norm, tangent_vec.y / norm))
    }

    pub fn closest_point_to(
        &self,
        start: Point3D,
        x: f64,
        y: f64,
    ) -> Option<(f64, Point, f64)> {
        match self {
            Command::Move { .. } => None,
            Command::Line { .. } => {
                let p0 = Point::new(start.x, start.y);
                let p1 = Point::new(self.end_point().x, self.end_point().y);
                Some(get_line_segment_closest_point(p0, p1, x, y))
            }
            Command::Arc {
                end,
                center_offset,
                normal,
                ..
            } => get_arc_closest_point(
                *end,
                *center_offset,
                *normal,
                start,
                x,
                y,
            ),
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => get_bezier_closest_point(
                *end, *control1, *control2, start, x, y,
            ),
        }
    }

    pub fn linearize(
        &self,
        start: Point3D,
        resolution: f64,
        out: &mut Vec<(Point3D, Point3D)>,
    ) {
        out.clear();
        match self {
            Command::Move { .. } => {}
            Command::Line { .. } => {
                out.push((start, self.end_point()));
            }
            Command::Arc {
                end,
                center_offset,
                normal,
                ..
            } => linearize_arc(
                *end,
                *center_offset,
                *normal,
                start,
                resolution,
                out,
            ),
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => {
                let segments = linearize_bezier_from_params(
                    *end, *control1, *control2, start, resolution,
                );
                out.extend(segments);
            }
        }
    }

    pub fn bounding_box(&self, start: Point) -> Option<Rect> {
        let end = self.end_point();
        let p1 = Point::new(end.x, end.y);
        match self {
            Command::Move { .. } => None,
            Command::Line { .. } => Some(Rect(
                start.x.min(p1.x),
                start.y.min(p1.y),
                start.x.max(p1.x),
                start.y.max(p1.y),
            )),
            Command::Arc {
                center_offset,
                normal,
                ..
            } => {
                let clockwise = normal.z < 0.0;
                Some(get_arc_bounds(
                    start,
                    p1,
                    Point::new(center_offset.x, center_offset.y),
                    clockwise,
                ))
            }
            Command::Bezier {
                control1, control2, ..
            } => Some(get_bezier_bounds(
                start,
                Point::new(control1.x, control1.y),
                Point::new(control2.x, control2.y),
                p1,
            )),
        }
    }
}

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
    pub geo: crate::geo::geometry::Geometry,
    /// Whether the contour forms a closed path.
    pub is_closed: bool,
    /// List of vertices defining the contour.
    pub vertices: Polygon,
    /// The signed area of the contour (positive for CCW, negative for CW).
    pub area: f64,
    /// Winding order of the contour.
    pub winding_order: Option<WindingOrder>,
}
