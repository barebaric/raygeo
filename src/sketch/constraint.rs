//! The sketch constraint contract.
//!
//! Every constraint mirrors one class of the reference sketch model
//! and owns its own residual and gradient math (see the [`constraints`
//! submodule]).  Constraints reference points directly, or reference
//! entities and resolve them through the point context at residual
//! time — mirroring the reference model, where entity-referencing
//! constraints looked their geometry up through the registry on every
//! evaluation.  The caller resolves parameter expressions to plain
//! floats before solving.

use serde_json::Value;

use crate::sketch::draw::{CairoLike, DrawContext, DrawPoint, DrawStyle};

/// Read/write view of the sketch's points and entities, addressed by
/// point id.  Constraints read coordinates through [`PointContext::xy`];
/// constraints whose Jacobian is computed by finite differences
/// (e.g. the angle constraint) additionally write perturbed
/// coordinates through [`PointContext::set_xy`], mirroring how the
/// reference implementation mutates registry points.  Entities
/// resolve their defining-point geometry and their entity
/// dependencies through the same surface, so both layers work
/// against snapshots and against the application's live document.
pub trait PointContext {
    /// Returns the `(x, y)` coordinates of a point id, or `None`
    /// when the id is unknown.
    fn point_position(&self, id: u64) -> Option<(f64, f64)>;

    /// Returns the point state for a point id, or `None` when the
    /// id is unknown.
    ///
    /// Implementations must serve the *same* positions as
    /// [`PointContext::point_position`]: constraints read geometry
    /// through `point_position`/`xy` during solving, so a `get_point`
    /// that reports stale snapshot positions silently desynchronizes
    /// `error()` from `is_hit`/`draw`.  Use it for state flags and
    /// display, and prefer `point_position` for coordinates.
    fn get_point(&self, id: u64) -> Option<DrawPoint> {
        self.point_position(id).map(|(x, y)| DrawPoint {
            x,
            y,
            fixed: false,
            constrained: false,
        })
    }

    /// Returns the `(x, y)` coordinates of a point id.
    fn xy(&self, id: u64) -> (f64, f64) {
        self.point_position(id).unwrap_or((0.0, 0.0))
    }

    /// Overwrites the coordinates of a point id.
    fn set_xy(&mut self, id: u64, x: f64, y: f64);

    /// Returns the entity with the given id, cloned out of the
    /// context, for constraints that reference entities and resolve
    /// them at residual time.  Ownership keeps snapshot and live
    /// contexts symmetric.
    fn get_entity(&self, id: u64) -> Option<Box<dyn Entity>>;

    /// Returns all entity ids of the document.
    fn entity_ids(&self) -> Vec<u64>;

    /// Returns the type name of an entity id.
    fn entity_type(&self, id: u64) -> Option<std::borrow::Cow<'static, str>>;

    /// Returns the point ids of an entity id.
    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>>;
}

/// One Jacobian entry: derivative of residual `row` w.r.t. the two
/// coordinates of `point_id`.
use crate::sketch::entities::entity::Entity;

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

    /// Returns all point ids this constraint couples.  Entity
    /// references resolve through the context.
    fn referenced_points(&self, ctx: &dyn PointContext) -> Vec<u64>;

    /// Returns the number of scalar residuals.  Entity references
    /// resolve through the context.
    ///
    /// # Contract
    ///
    /// [`SketchConstraint::error`] must return exactly this many
    /// rows, so the residual vector and the Jacobian row layout stay
    /// in sync.  Static-count implementations return zero rows here
    /// only for constraints that never contribute; dynamic-count
    /// implementations (such as `EqualLengthConstraint`, which
    /// depends on resolvable entity pairs) return `0` exactly when
    /// their `error` returns an empty vector.  When referenced
    /// entities cannot be resolved, a constraint imposes nothing:
    /// static-count implementations return zero-valued rows.
    fn residual_count(&self, ctx: &dyn PointContext) -> usize;

    /// Calculates the error of the constraint.
    ///
    /// Must return exactly [`SketchConstraint::residual_count`] rows
    /// (see its contract).
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
        _model: &dyn PointContext,
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
