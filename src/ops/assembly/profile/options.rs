use std::path::PathBuf;

use crate::geo::types::Point3D;
use crate::ops::assembly::{result::AssemblyMeta, AssembleCtx, Assembler};
use crate::ops::types::CutDirection;

/// Which boundary the [`ProfileSpec`] walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileKind {
    /// Inset (inward-offset) boundary; material-aware around islands.
    Inner,
    /// Grown (outward-offset) boundary; ignores islands.
    Outer,
}

/// Spec for the adaptive-profile assembler.
///
/// Walks a tool around the **inner** or **outer** profile of a closed
/// boundary depending on [`Self::kind`]. Inner profiling follows the
/// inset boundary (offset inward by tool radius), material-aware
/// around islands. Outer profiling follows the grown boundary
/// (offset outward) and ignores islands.
///
/// Geometry is supplied via a [`Part`](crate::ops::part::Part) —
/// boundary and islands are extracted internally.
#[derive(Clone, Debug)]
pub struct ProfileSpec {
    /// Inner or outer profile.
    pub kind: ProfileKind,
    pub tool_radius: f64,
    pub step_over: f64,
    pub step_length: f64,
    pub target_z: f64,
    pub safe_z: f64,
    pub wall_margin: f64,
    pub stock_to_leave: f64,
    pub cut_direction: CutDirection,
    pub start_pos: Option<Point3D>,
    pub tolerance: f64,
    pub expansion_batch_size: usize,
    pub cancel_check: Option<fn() -> bool>,
    pub engagement_area_threshold: f64,
    pub engagement_angle_threshold: f64,
    /// Factor (0–1) by which feed is reduced on over-engagement, as a
    /// runtime safety modulation of the caller-provided cut feed. The
    /// base feed comes from the cut `State`; this is the only feed
    /// value the profile engine derives itself.
    pub feed_reduction_factor: f64,
    pub trace_path: Option<PathBuf>,
}

impl Default for ProfileSpec {
    fn default() -> Self {
        Self {
            kind: ProfileKind::Inner,
            tool_radius: 3.0,
            step_over: 1.5,
            step_length: 0.6,
            target_z: -5.0,
            safe_z: 2.0,
            wall_margin: 0.0,
            stock_to_leave: 0.0,
            cut_direction: CutDirection::Ccw,
            start_pos: None,
            tolerance: 0.1,
            expansion_batch_size: 20,
            cancel_check: None,
            engagement_area_threshold: 0.0,
            engagement_angle_threshold: std::f64::consts::PI,
            feed_reduction_factor: 0.5,
            trace_path: None,
        }
    }
}

impl ProfileSpec {
    /// A spec carrying the planner's canonical tuning for `tool_radius`:
    /// a radius-wide step-over, a fifth-of-a-radius step length, the
    /// clearing planner's engagement gates (a 1.0 mm² area gate, and a
    /// 30-rad angle gate that never trips, so feed reduction is
    /// area-driven) and its 0.5 feed-reduction factor. The fields a
    /// caller must shape per operation — `wall_margin`,
    /// `stock_to_leave`, `cut_direction`, `tolerance`, `start_pos`,
    /// `cancel_check`, `trace_path` — keep their neutral defaults; set
    /// them on the returned value.
    pub fn for_tool(
        kind: ProfileKind,
        tool_radius: f64,
        target_z: f64,
        safe_z: f64,
    ) -> Self {
        Self {
            kind,
            tool_radius,
            step_over: tool_radius,
            step_length: tool_radius * 0.2,
            target_z,
            safe_z,
            wall_margin: 0.0,
            stock_to_leave: 0.0,
            cut_direction: CutDirection::Ccw,
            start_pos: None,
            tolerance: 0.1,
            expansion_batch_size: 20,
            cancel_check: None,
            engagement_area_threshold: 1.0,
            engagement_angle_threshold: 30.0,
            feed_reduction_factor: 0.5,
            trace_path: None,
        }
    }
}

impl Assembler for ProfileSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        let label = match self.kind {
            ProfileKind::Inner => "profile_inner",
            ProfileKind::Outer => "profile_outer",
        };
        ctx.callbacks
            .report_progress(0.0, &format!("{label}: assemble"));
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        // Region scoping is polygonal, so a scoped step profiles the
        // polygon path; an unscoped step with curve-true face geometry
        // prefers the exact offset.
        let prefer_exact = ctx.region_boundary.is_none();
        let meta = match self.kind {
            ProfileKind::Inner => super::profile_inner(
                ctx.face,
                ctx.trace,
                self,
                ctx.state,
                prefer_exact,
            ),
            ProfileKind::Outer => super::profile_outer(
                ctx.face,
                ctx.trace,
                self,
                ctx.state,
                prefer_exact,
            ),
        }
        .map_err(|e| e.to_string())?;
        let fragments = ctx.face.cleared.fragments().to_vec();
        ctx.emit_vector_effect(fragments, None, Some(self.target_z));
        ctx.callbacks
            .report_progress(1.0, &format!("{label}: done"));
        Ok(meta)
    }

    fn name(&self) -> &str {
        match self.kind {
            ProfileKind::Inner => "profile_inner",
            ProfileKind::Outer => "profile_outer",
        }
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
