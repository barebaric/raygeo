//! Entities: the shapes a DXF drawing is made of.
//!
//! Each entity type is parsed into a small struct holding its geometry in
//! world coordinates, already resolved from its object coordinate system
//! and from any block insertion it came through. Everything downstream
//! therefore works with plain world-space geometry and never needs to
//! know about OCS, extrusion vectors or INSERT nesting.

use glam::{DMat4, DVec3};

use crate::formats::dxf::ocs::Ocs;
use crate::geo::types::Point;

/// How an arc or circle winds in its own plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Winding {
    /// Counter-clockwise in the entity's plane, as seen from +Z.
    CounterClockwise,
    /// Clockwise in the entity's plane, as seen from +Z.
    Clockwise,
}

/// A single drawable DXF entity, in world coordinates.
#[derive(Clone, Debug)]
pub struct Entity {
    /// The layer name the entity was drawn on.
    pub layer: String,
    /// The entity's shape.
    pub shape: Shape,
}

/// The geometry of one entity.
///
/// Every variant is already resolved into world space: an `INSERT` of a
/// block has been expanded into its constituent shapes, and points that
/// were expressed in an object coordinate system have been rotated.
#[derive(Clone, Debug)]
pub enum Shape {
    /// A single point, which draws nothing.
    Point(DVec3),
    /// A straight segment.
    Line {
        /// Start point.
        start: DVec3,
        /// End point.
        end: DVec3,
    },
    /// A circular arc.
    Arc {
        /// Centre of the circle the arc lies on.
        center: DVec3,
        /// Radius of that circle.
        radius: f64,
        /// Angle where the arc starts, in radians.
        start_angle: f64,
        /// Angle where the arc ends, in radians.
        end_angle: f64,
        /// Winding direction.
        winding: Winding,
    },
    /// A full circle.
    Circle {
        /// Centre of the circle.
        center: DVec3,
        /// Radius.
        radius: f64,
        /// Plane normal, used to pick the winding.
        normal: DVec3,
    },
    /// An elliptical arc.
    Ellipse {
        /// Centre of the ellipse.
        center: DVec3,
        /// Direction of the major axis, scaled to the major radius.
        major_axis: DVec3,
        /// Length of the minor axis.
        minor_ratio: f64,
        /// Parameter where the arc starts, in radians.
        start_param: f64,
        /// Parameter where the arc ends, in radians.
        end_param: f64,
    },
    /// A polyline, as a chain of vertices with optional bulges.
    Polyline {
        /// The vertices in order.
        vertices: Vec<Vertex>,
        /// Whether the polyline is closed.
        closed: bool,
    },
    /// A spline, flattened to a chain of cubic beziers.
    Spline {
        /// The curve segments in order, as world-space control points
        /// `(start, control1, control2, end)`.
        segments: Vec<[DVec3; 4]>,
    },
}

/// One polyline vertex.
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    /// Position in world coordinates.
    pub position: DVec3,
    /// Bulge factor, the tangent of a quarter of the arc's included
    /// angle. Zero for a straight segment. Positive bulges bow to the
    /// left of the direction of travel.
    pub bulge: f64,
}

impl Vertex {
    /// Whether the segment leaving this vertex is an arc.
    pub fn has_bulge(&self) -> bool {
        self.bulge.abs() > 1e-12
    }
}

/// A block definition, with its entities in block-local coordinates.
#[derive(Clone, Debug, Default)]
pub struct Block {
    /// The block's name.
    pub name: String,
    /// The drawable entities the block contains.
    pub entities: Vec<Entity>,
    /// The anchor point of the block.
    pub base_point: DVec3,
}

/// A block insertion, before the block's entities are resolved.
#[derive(Clone, Debug)]
pub struct Insert {
    /// The layer the insertion is on.
    pub layer: String,
    /// Name of the block to insert.
    pub block_name: String,
    /// Point the block's base point is placed at.
    pub insertion_point: DVec3,
    /// X scale factor.
    pub x_scale: f64,
    /// Y scale factor.
    pub y_scale: f64,
    /// Z scale factor.
    pub z_scale: f64,
    /// Rotation about the extrusion axis, in radians.
    pub rotation: f64,
    /// The transform mapping block-local coordinates to world space.
    pub transform: DMat4,
}

impl Insert {
    /// The order in which rotation is applied relative to scaling, as
    /// DXF requires: scale first, then rotation, then translation.
    pub fn build_transform(
        insertion_point: DVec3,
        x_scale: f64,
        y_scale: f64,
        z_scale: f64,
        rotation: f64,
        ocs: &Ocs,
    ) -> DMat4 {
        let scale = DMat4::from_diagonal(glam::DVec4::new(
            x_scale, y_scale, z_scale, 1.0,
        ));
        let rotation = DMat4::from_rotation_z(rotation);
        let translation = DMat4::from_translation(insertion_point);
        ocs.matrix() * translation * rotation * scale
    }
}

/// Converts an OCS point plus an extrusion elevation into a world point.
pub fn ocs_point(ocs: &Ocs, elevation: f64, x: f64, y: f64) -> DVec3 {
    ocs.to_world(DVec3::new(x, y, elevation))
}

/// The world-space winding of an arc, given the OCS plane's effect.
///
/// A mirroring transform reverses the apparent direction of travel, so
/// an arc that is counter-clockwise in its own plane is clockwise when
/// the plane is seen from above.
pub fn resolve_winding(winding: Winding, ocs: &Ocs) -> Winding {
    if ocs.faces_down() {
        match winding {
            Winding::CounterClockwise => Winding::Clockwise,
            Winding::Clockwise => Winding::CounterClockwise,
        }
    } else {
        winding
    }
}

/// Interprets the group code that states an arc's direction.
pub fn winding_from_counterclockwise_flag(counterclockwise: bool) -> Winding {
    if counterclockwise {
        Winding::CounterClockwise
    } else {
        Winding::Clockwise
    }
}

/// The point on an ellipse at the given parameter.
pub fn ellipse_point(
    center: DVec3,
    major_axis: DVec3,
    minor_ratio: f64,
    parameter: f64,
) -> DVec3 {
    let major = major_axis;
    let minor = minor_ratio
        * major_axis.length()
        * DVec3::Z.cross(major.normalize()).normalize();
    center + major * parameter.cos() + minor * parameter.sin()
}

/// Converts a bulged segment into the arc through it.
///
/// `bulge` is the tangent of a quarter of the included angle, so the
/// included angle is `4 * atan(bulge)`. The arc's radius follows from
/// the chord length and that angle. Returns `None` for a zero bulge,
/// which is a straight segment.
pub fn bulge_to_arc(start: DVec3, end: DVec3, bulge: f64) -> Option<BulgeArc> {
    if bulge.abs() < 1e-12 {
        return None;
    }
    let chord = end - start;
    let chord_len = chord.truncate().length();
    if chord_len < 1e-12 {
        return None;
    }
    let included = 4.0 * bulge.atan();
    let half = included / 2.0;
    let radius = (chord_len / 2.0) / half.sin().abs();
    // The centre lies on the chord's perpendicular bisector, at the
    // signed distance r * cos (included / 2) from the midpoint. The
    // cosine carries the sign: within a half turn the centre stays on the
    // chord's near side, past it the centre crosses over on its own. The
    // side depends only on which way the bulge turns, not on its size.
    let midpoint = (start + end) * 0.5;
    let left = DVec3::new(-chord.y, chord.x, 0.0).normalize();
    let side = if bulge > 0.0 { 1.0 } else { -1.0 };
    let center = midpoint + left * (radius * half.cos() * side);
    Some(BulgeArc {
        center: center.truncate(),
        radius,
        // Travel between the two end points takes the short way round the
        // centre.
        winding: if bulge > 0.0 {
            Winding::CounterClockwise
        } else {
            Winding::Clockwise
        },
    })
}

/// The arc that replaces a bulged polyline segment.
#[derive(Clone, Copy, Debug)]
pub struct BulgeArc {
    /// Centre of the arc.
    pub center: Point,
    /// Radius of the arc.
    pub radius: f64,
    /// Winding direction.
    pub winding: Winding,
}

impl BulgeArc {
    /// The centre as a 3D point at the given elevation.
    pub fn center_3d(&self, z: f64) -> DVec3 {
        DVec3::new(self.center.x, self.center.y, z)
    }
}
