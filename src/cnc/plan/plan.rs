use std::sync::Arc;

use crate::ops::assembly::Assembler;
use crate::types::Polygon;

/// One step in a [`Plan`]: a typed assembler spec targeting a face.
///
/// When `region_boundary` is `Some`, the assembler temporarily swaps the
/// face's stock region for that boundary before running — this lets each
/// step see a different pocket subset (e.g. a single region) while still
/// sharing the same face's cleared area.  `None` means use the face's
/// default stock region (usually the full pocket boundary).
#[derive(Clone)]
pub struct PlanStep {
    /// Which face of the input `Part` this step operates on.
    pub face_id: String,
    /// The assembler spec for this step. A downstream consumer maps
    /// it to its own step representation, and `create_intent` clones
    /// it into `NodeRequest`s.
    pub spec: Arc<dyn Assembler>,
    /// Optional per-step boundary + islands.  When set, the executor
    /// temporarily replaces the face's stock region with this boundary
    /// before calling the assembler, then restores the original after.
    pub region_boundary: Option<(Polygon, Vec<Polygon>)>,
}

impl std::fmt::Debug for PlanStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanStep")
            .field("face_id", &self.face_id)
            .field("spec", &self.spec.name())
            .finish()
    }
}

/// A **descriptive** plan produced by a planner function.
///
/// A `Plan` is never executed directly. Its sole purpose is to be
/// inspected by a downstream consumer that derives its own step
/// representation from the typed specs.
pub struct Plan {
    /// Ordered list of steps.
    pub steps: Vec<PlanStep>,
    /// Safe Z height for travel moves between steps.
    pub safe_z: f64,
    /// Boundary of the pocket this plan applies to.
    pub pocket_boundary: Polygon,
    /// Islands inside the pocket.
    pub islands: Vec<Polygon>,
}

impl std::fmt::Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plan")
            .field("steps", &self.steps.len())
            .field("safe_z", &self.safe_z)
            .finish()
    }
}

impl Plan {
    pub fn new(
        pocket_boundary: Polygon,
        islands: Vec<Polygon>,
        safe_z: f64,
    ) -> Self {
        Plan {
            steps: Vec::new(),
            pocket_boundary,
            islands,
            safe_z,
        }
    }

    pub fn extend(&mut self, steps: Vec<PlanStep>) {
        self.steps.extend(steps);
    }

    pub fn push(&mut self, step: PlanStep) {
        self.steps.push(step);
    }
}
