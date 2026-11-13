//! The sketch constraint contract.
//!
//! Every constraint mirrors one class of the reference sketch model
//! and owns its own residual and gradient math (see the [`constraints`
//! submodule]).  The caller resolves entity references to concrete
//! point ids and parameter expressions to plain floats, so a
//! constraint sees only a point context and never entity or parameter
//! knowledge.

use serde_json::Value;

use crate::sketch::draw::{CairoLike, DrawContext, DrawStyle};
use crate::sketch::registry::EntityRegistry;

/// Read/write view of the sketch's point coordinates, addressed by
/// point id.  Constraints read coordinates through [`PointContext::xy`];
/// constraints whose Jacobian is computed by finite differences
/// (e.g. the angle constraint) additionally write perturbed
/// coordinates through [`PointContext::set_xy`], mirroring how the
/// reference implementation mutates registry points.
pub trait PointContext {
    /// Returns the `(x, y)` coordinates of a point id.
    fn xy(&self, id: u64) -> (f64, f64);

    /// Overwrites the coordinates of a point id.
    fn set_xy(&mut self, id: u64, x: f64, y: f64);
}

/// One Jacobian entry: derivative of residual `row` w.r.t. the two
/// coordinates of `point_id`.
pub type JacobianEntry = (u64, usize, f64, f64);

/// Base contract of all sketch constraints, mirroring the reference
/// implementation's constraint base class.
pub trait SketchConstraint: Send + Sync {
    /// Downcast hook for the binding layer.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Serialization to the reference constraint dictionary shape.
    fn to_json(&self) -> Value;

    /// Returns all point ids this constraint couples.
    fn referenced_points(&self) -> Vec<u64>;

    /// Returns the number of scalar residuals.
    fn residual_count(&self) -> usize;

    /// Calculates the error of the constraint.
    fn error(&self, reg: &dyn PointContext) -> Vec<f64>;

    /// Calculates the partial derivatives of the error as
    /// `(point_id, row, d/dx, d/dy)` entries.  Entries referencing
    /// points outside the solver's state vector are skipped by the
    /// assembler; entries for the same `(point_id, row)` accumulate.
    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry>;

    /// Whether the constraint's rendered symbol is hit at (mx, my).
    /// `to_screen` maps model coordinates to screen coordinates (the
    /// reference constraint symbol layout math runs in screen space).
    fn is_hit(
        &self,
        _mx: f64,
        _my: f64,
        _threshold: f64,
        _model: &EntityRegistry,
        _to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        false
    }

    /// Draws the constraint's rendered symbol onto `ctx` (screen
    /// space).  Mirrors the reference base class, whose default
    /// implementation draws nothing.
    fn draw(
        &self,
        _context: &dyn DrawContext,
        _to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        _ctx: &dyn CairoLike,
        _style: &DrawStyle,
    ) {
    }
}
