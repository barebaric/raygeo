//! Stuck-detection and per-step safety invariants for
//! [`super::adaptive_clearing`].

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::types::Point;
use crate::ops::part::ClearedArea;

// ── Constants ──────────────────────────────────────────────────────────

/// Check progress every N successful steps.
const STUCK_CHECK_INTERVAL: usize = 100;

/// Minimum fraction of theoretical target cut-area throughput that the
/// cleared area must grow by during a progress window.
const STUCK_MIN_GROWTH_FACTOR: f64 = 0.15;

// ── StuckOutcome ──────────────────────────────────────────────────────

/// Result of a stuck-detection tick.
pub(super) enum StuckOutcome {
    /// Tool is making progress or it's a stall step (no tick).
    Ok,
    /// Tool is oscillating — growth fell below the expected threshold.
    Oscillating { growth: f64, expected: f64 },
}

// ── StuckDetector ─────────────────────────────────────────────────────

/// Tracks step count and cleared-area growth to detect oscillation.
pub(super) struct StuckDetector {
    step_count: usize,
    last_check_area: f64,
    target_area_pd: f64,
    step_length: f64,
}

impl StuckDetector {
    pub fn new(
        target_area_pd: f64,
        step_length: f64,
        initial_area: f64,
    ) -> Self {
        Self {
            step_count: 0,
            last_check_area: initial_area,
            target_area_pd,
            step_length,
        }
    }

    /// Advance the detector by one step.  Only successful (non-stall)
    /// steps increment the counter; every `STUCK_CHECK_INTERVAL` steps
    /// the cleared-area growth is compared against the expected
    /// throughput.
    pub fn tick(&mut self, current_area: f64, stalled: bool) -> StuckOutcome {
        if stalled {
            return StuckOutcome::Ok;
        }
        self.step_count += 1;
        if !self.step_count.is_multiple_of(STUCK_CHECK_INTERVAL) {
            return StuckOutcome::Ok;
        }
        let growth = current_area - self.last_check_area;
        self.last_check_area = current_area;
        let expected = STUCK_CHECK_INTERVAL as f64
            * self.step_length
            * self.target_area_pd
            * STUCK_MIN_GROWTH_FACTOR;
        if growth < expected {
            StuckOutcome::Oscillating { growth, expected }
        } else {
            StuckOutcome::Ok
        }
    }

    /// Reset after a successful resume (step count → 0, baseline area).
    pub fn reset(&mut self, current_area: f64) {
        self.step_count = 0;
        self.last_check_area = current_area;
    }

    pub fn step_count(&self) -> usize {
        self.step_count
    }
}

// ── Wrong-side safehold ───────────────────────────────────────────────

/// Accumulates the fresh cut area on each side of the step direction over
/// a clearing run, and fails the run when the walk has **sustained** on
/// the wrong side of its direction.
///
/// Islands legitimately produce transient wrong-side steps: a frontier
/// fold or a breakthrough between two cleared regions puts fresh material
/// on the side the direction bias calls wrong for a step or two, and the
/// walk recovers. A solver that yields incorrect *angles* systematically —
/// the defect this guard exists to catch — accumulates wrong-side area
/// every step with nothing on the correct side, so a sustained-dominance
/// check catches it within a few steps while leaving island transients
/// alone.
pub(super) struct WrongSideGuard {
    wrong: f64,
    correct: f64,
}

impl WrongSideGuard {
    pub(super) fn new() -> Self {
        WrongSideGuard {
            wrong: 0.0,
            correct: 0.0,
        }
    }

    /// Accumulates this step's fresh cut sides and fails the run once the
    /// wrong side dominates the run's cut area by more than two steps'
    /// worth of target engagement.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn check_step(
        &mut self,
        cleared: &ClearedArea,
        dir_sign: f64,
        prev_pos: Point,
        tool_pos: Point,
        radius: f64,
        target_area_pd: f64,
        step_length: f64,
    ) -> RaygeoResult<()> {
        let (total, left) = cleared.cut_area_split(prev_pos, tool_pos, radius);
        let right = total - left;
        let (wrong, correct) = if dir_sign < 0.0 {
            (left, right)
        } else {
            (right, left)
        };
        self.wrong += wrong;
        self.correct += correct;
        let per_step_target = target_area_pd * step_length;
        if self.wrong > self.correct
            && self.wrong - self.correct > per_step_target * 2.0
        {
            return Err(RaygeoError::WrongSideCut(format!(
                "sustained wrong-side cutting: wrong={:.6}  correct={:.6}  \
                 dir_sign={:+.1}  pos=({:.3},{:.3})",
                self.wrong, self.correct, dir_sign, tool_pos.x, tool_pos.y,
            )));
        }
        Ok(())
    }
}
