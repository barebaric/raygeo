//! OCS: the object coordinate system DXF entities are drawn in.
//!
//! An entity's points are not necessarily in world coordinates. DXF
//! gives most entities an extrusion direction (group codes 210/220/230),
//! the normal of the plane the entity lies in. Points are stated
//! relative to that plane's own axes, and must be rotated into world
//! space using an arbitrary-axis algorithm.
//!
//! For the overwhelming majority of drawings the extrusion is
//! `(0, 0, 1)`, the identity case, and this module does nothing. The
//! general case matters for parts exported from 3D CAD, where a flat
//! outline may be drawn on a plane facing down or sideways.

use glam::{DMat4, DVec3};

/// The default extrusion direction, where OCS and world coincide.
pub const DEFAULT_EXTRUSION: DVec3 = DVec3::new(0.0, 0.0, 1.0);

/// Rotation taking an entity's OCS coordinates into world coordinates.
///
/// Constructed from the entity's extrusion vector. A vector parallel to
/// the Z axis yields the identity, apart from the mirroring that a
/// downward-facing plane requires: DXF's arbitrary-axis algorithm
/// deliberately flips X for `(0, 0, -1)`, so an arc that was drawn
/// counter-clockwise in its own plane still winds the same way when seen
/// from above.
#[derive(Clone, Copy, Debug)]
pub struct Ocs {
    rotation: DMat4,
}

impl Ocs {
    /// Builds the transform for the given extrusion vector.
    ///
    /// A zero-length extrusion is treated as `(0, 0, 1)`: some writers
    /// emit the codes without a value.
    pub fn from_extrusion(extrusion: DVec3) -> Self {
        let z = if extrusion.length_squared() < 1e-18 {
            DEFAULT_EXTRUSION
        } else {
            extrusion.normalize()
        };
        Ocs {
            rotation: arbitrary_axis_rotation(z),
        }
    }

    /// The identity transform, for entities with no extrusion code.
    pub fn identity() -> Self {
        Ocs {
            rotation: DMat4::IDENTITY,
        }
    }

    /// Rotates an OCS point into world coordinates.
    pub fn to_world(&self, point: DVec3) -> DVec3 {
        self.rotation.transform_point3(point)
    }

    /// Rotates a world-space vector into the OCS.
    pub fn to_ocs_vector(&self, vector: DVec3) -> DVec3 {
        self.rotation.inverse().transform_vector3(vector)
    }

    /// The 4x4 matrix taking OCS coordinates into world coordinates.
    pub fn matrix(&self) -> DMat4 {
        self.rotation
    }

    /// Whether the plane faces away from +Z.
    ///
    /// Entities on a downward-facing plane are seen from behind, so an
    /// arc that runs counter-clockwise on the plane appears to wind the
    /// other way in the drawing.
    pub fn faces_down(&self) -> bool {
        self.rotation.z_axis.z < 0.0
    }

    /// Whether this is the identity transform.
    pub fn is_identity(&self) -> bool {
        self.rotation.abs_diff_eq(DMat4::IDENTITY, 1e-12)
    }
}

/// The arbitrary-axis algorithm from the DXF specification.
///
/// Builds an orthonormal basis whose Z axis is `normal`, and returns the
/// transform from that basis into world space. When `normal` is exactly
/// `(0, 0, 1)` the result is the identity. When it points the other way,
/// `X` and `Y` are kept and only `Z` flips, which leaves a shape in the
/// same place on the page but reverses which way round it winds.
pub fn arbitrary_axis_rotation(normal: DVec3) -> DMat4 {
    let z = normal.normalize();
    if (z - DEFAULT_EXTRUSION).length_squared() < 1e-18 {
        return DMat4::IDENTITY;
    }
    if (z + DEFAULT_EXTRUSION).length_squared() < 1e-18 {
        return DMat4::from_cols(
            glam::DVec4::new(1.0, 0.0, 0.0, 0.0),
            glam::DVec4::new(0.0, 1.0, 0.0, 0.0),
            glam::DVec4::new(0.0, 0.0, -1.0, 0.0),
            glam::DVec4::new(0.0, 0.0, 0.0, 1.0),
        );
    }
    let (x, y) = if z.x.abs() < 1.0 / 64.0 && z.y.abs() < 1.0 / 64.0 {
        // Nearly parallel to Z: use -Z as the reference vector.
        (DVec3::new(0.0, 0.0, -1.0).cross(z), DVec3::Y)
    } else {
        let cross = DVec3::Z.cross(z);
        (cross, z.cross(cross))
    };
    let x = x.normalize();
    let y = y.normalize();
    DMat4::from_cols(
        glam::DVec4::new(x.x, x.y, x.z, 0.0),
        glam::DVec4::new(y.x, y.y, y.z, 0.0),
        glam::DVec4::new(z.x, z.y, z.z, 0.0),
        glam::DVec4::new(0.0, 0.0, 0.0, 1.0),
    )
}
