//! Stuck-detection and per-step safety invariants for
//! [`super::adaptive_clearing`].

use crate::ops::cut::ClearedArea;
use crate::types::Point;

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

/// After each successful step, verify the tool is not cutting
/// predominantly on the wrong side of the step direction.  Panics if
/// the wrong-side cut area exceeds the correct side and the per-step
/// target — this signals a stepper bug (the solver should never yield
/// an incorrect angle).
pub(super) fn wrong_side_safehold(
    cleared: &ClearedArea,
    dir_sign: f64,
    prev_pos: Point,
    tool_pos: Point,
    radius: f64,
    target_area_pd: f64,
    step_length: f64,
) {
    let (total, left) = cleared.cut_area_split(prev_pos, tool_pos, radius);
    let right = total - left;
    let wrong = if dir_sign < 0.0 { left } else { right };
    let correct = total - wrong;
    let per_step_target = target_area_pd * step_length;
    if wrong > correct && wrong > per_step_target * 0.5 {
        panic!(
            "adaptive_clearing: wrong-side safehold  \
             dir_sign={:+.1}  total={:.6}  left={:.6}  \
             right={:.6}  pos=({:.3},{:.3})  heading={:.4}",
            dir_sign, total, left, right, tool_pos.x, tool_pos.y, 0.0,
        );
    }
}
