//! Bed-mesh correction: warp toolpath Z onto a probed height map.
//!
//! Lasers and CNC machines assume the work surface is flat. On large
//! beds it often is not; a height map probed on a grid lets the focal
//! point (or cut depth) follow the surface. [`mesh_correction`] adds
//! the bilinearly interpolated grid height to every moving command's
//! Z, in place.

use crate::error::RaygeoError;
use crate::ops::callbacks::Callbacks;
use crate::ops::container::Ops;
use crate::ops::types::{MoveCmd, OpCategory};

/// A probed bed height map and the parameters needed to sample it.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshCorrectionSpec {
    /// X coordinate of the first grid column.
    pub x0: f64,
    /// Y coordinate of the first grid row.
    pub y0: f64,
    /// Spacing between grid columns along X. Must be positive when
    /// ``nx > 1``.
    pub dx: f64,
    /// Spacing between grid rows along Y. Must be positive when
    /// ``ny > 1``.
    pub dy: f64,
    /// Surface heights in row-major order: ``ny`` rows of ``nx``
    /// values, ``heights[j * nx + i]`` being the height at
    /// ``(x0 + i * dx, y0 + j * dy)``.
    pub heights: Vec<f64>,
    /// Number of grid columns (X direction), at least 1.
    pub nx: usize,
    /// Number of grid rows (Y direction), at least 1.
    pub ny: usize,
    /// Constant Z added on top of every sampled height, e.g. a focus
    /// offset above the surface or the distance between the original
    /// Z reference plane and the height map datum.
    pub z_offset: f64,
}

impl MeshCorrectionSpec {
    /// Bilinearly interpolated height at ``(x, y)``. Coordinates
    /// outside the grid are clamped to the nearest edge sample.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let last_col = (self.nx - 1) as f64;
        let last_row = (self.ny - 1) as f64;
        let fx = if self.nx > 1 {
            ((x - self.x0) / self.dx).clamp(0.0, last_col)
        } else {
            0.0
        };
        let fy = if self.ny > 1 {
            ((y - self.y0) / self.dy).clamp(0.0, last_row)
        } else {
            0.0
        };
        let i = (fx as usize).min(self.nx - 1);
        let j = (fy as usize).min(self.ny - 1);
        let i1 = (i + 1).min(self.nx - 1);
        let j1 = (j + 1).min(self.ny - 1);
        let tx = fx - i as f64;
        let ty = fy - j as f64;
        let h = &self.heights;
        let lo = h[j * self.nx + i] * (1.0 - tx) + h[j * self.nx + i1] * tx;
        let hi = h[j1 * self.nx + i] * (1.0 - tx) + h[j1 * self.nx + i1] * tx;
        lo * (1.0 - ty) + hi * ty
    }

    fn validate(&self) -> Result<(), RaygeoError> {
        if self.nx == 0 || self.ny == 0 {
            return Err(RaygeoError::InvalidCommand(
                "mesh grid must have at least one row and one column"
                    .to_string(),
            ));
        }
        if self.heights.len() != self.nx * self.ny {
            return Err(RaygeoError::InvalidCommand(format!(
                "heights length {} does not match a {} x {} grid",
                self.heights.len(),
                self.ny,
                self.nx
            )));
        }
        if self.nx > 1 && (!self.dx.is_finite() || self.dx <= 0.0) {
            return Err(RaygeoError::InvalidCommand(
                "dx must be positive and finite".to_string(),
            ));
        }
        if self.ny > 1 && (!self.dy.is_finite() || self.dy <= 0.0) {
            return Err(RaygeoError::InvalidCommand(
                "dy must be positive and finite".to_string(),
            ));
        }
        if !self.x0.is_finite()
            || !self.y0.is_finite()
            || !self.z_offset.is_finite()
        {
            return Err(RaygeoError::InvalidCommand(
                "x0, y0 and z_offset must be finite".to_string(),
            ));
        }
        if let Some(idx) = self.heights.iter().position(|h| !h.is_finite()) {
            return Err(RaygeoError::InvalidCommand(format!(
                "height at flat index {idx} is not finite"
            )));
        }
        Ok(())
    }
}

/// Warp every moving command's Z onto the height map in *spec*.
///
/// The height at each command's XY position is sampled bilinearly from
/// the grid and added to the command's Z together with
/// [`MeshCorrectionSpec::z_offset`]. This covers travel moves as well
/// as cutting moves, so travel at cut height clears the wavy surface
/// too. Arcs keep their shape: arc centres are relative I/J offsets
/// in the XY plane, so a corrected arc becomes a helical arc, and
/// bezier control points are corrected at their own XY positions.
/// Commands with non-finite XY are left unchanged.
///
/// Operates in place; the command buffer is only re-allocated (CoW)
/// when it is still shared with another `Ops` clone.
pub fn mesh_correction(
    ops: &mut Ops,
    spec: &MeshCorrectionSpec,
    callbacks: &dyn Callbacks,
) -> Result<(), RaygeoError> {
    spec.validate()?;
    if callbacks.is_cancelled() || ops.is_empty() {
        return Ok(());
    }

    let total = ops.len();
    let stride = (total / 8).max(1);
    let z_offset = spec.z_offset;
    for (i, node) in ops.cmds_mut().iter_mut().enumerate() {
        if i % stride == 0 {
            callbacks
                .report_progress(i as f64 / total as f64, "mesh_correction");
        }
        let OpCategory::Moving { end, cmd } = &mut node.category else {
            continue;
        };
        if !end.x.is_finite() || !end.y.is_finite() {
            continue;
        }
        end.z += spec.sample(end.x, end.y) + z_offset;
        match cmd {
            MoveCmd::BezierTo(data) => {
                data.control1.z +=
                    spec.sample(data.control1.x, data.control1.y) + z_offset;
                data.control2.z +=
                    spec.sample(data.control2.x, data.control2.y) + z_offset;
            }
            MoveCmd::QuadraticBezierTo { control } => {
                control.z += spec.sample(control.x, control.y) + z_offset;
            }
            _ => {}
        }
    }
    ops.last_move_to.z +=
        spec.sample(ops.last_move_to.x, ops.last_move_to.y) + z_offset;
    callbacks.report_progress(1.0, "mesh_correction");
    ops.invalidate_time_cache();
    Ok(())
}
