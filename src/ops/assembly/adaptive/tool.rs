//! The cutting-tool state used by [`super::adaptive_clearing`].
//!
//! A short gyroscope buffer averages recent direction vectors so that
//! small engagement wiggles do not jerk the tool path.  A separate
//! history of recent solver deltas serves as a steering predictor.

use prof_macros::prof;

use crate::geo::types::{Point, Point3D};

// ── Tool constants ───────────────────────────────────────────────────

/// Per-step decay applied to the persisted predictor.  A converged
/// deflection feeds `decay · prev + (1-decay) · new` back in, so a
/// steady curvature is tracked while a one-off correction decays to
/// zero within ~3 steps instead of seeding the next solver trial.
pub(super) const PREDICTOR_DECAY: f64 = 0.5;
/// The predictor is only allowed to seed the solver with a deflection
/// up to this fraction of `max_deflection`.  Larger corrections must
/// come from the solver's own bracket search, not from feedforward —
/// this prevents a stale large predicted angle from dominating the
/// first trial and pinning `best_error` on an overshoot.
pub(super) const PREDICTOR_CLAMP_FRAC: f64 = 0.5;
/// Number of recent direction vectors to average for heading smoothing.
pub(super) const GYRO_BUFFER_LEN: usize = 5;

// ── Tool ─────────────────────────────────────────────────────────────

/// A cutting tool with persistent position and heading.
///
/// A short gyroscope buffer averages recent direction vectors so that
/// small engagement wiggles do not jerk the tool path.  A decayed
/// predictor feeds the last converged deflection back into the next
/// step's solver trial for steady curvature tracking.
#[derive(Clone, Copy, Debug)]
pub struct Tool {
    /// Tool centre position.
    pub pos: Point3D,
    /// Current heading angle (radians).
    pub heading: f64,
    /// Tool radius.
    pub radius: f64,
    /// Recent direction vectors used for heading smoothing.
    gyro: [Point; GYRO_BUFFER_LEN],
    /// Number of valid entries in `gyro` (0..GYRO_BUFFER_LEN).
    gyro_count: usize,
    /// Decayed predictor value.  Updated only on converged steps and
    /// multiplied by [`PREDICTOR_DECAY`] each step, so a single
    /// transient over-correction does not seed the next step's solver
    /// trial and create a multi-step steering oscillation.
    predictor: f64,
}

impl Tool {
    /// Create a new tool, initializing the gyroscope with the initial
    /// heading.
    #[prof]
    pub fn new(pos: Point3D, heading: f64, radius: f64) -> Self {
        let dir = Point::new(heading.cos(), heading.sin());
        Self {
            pos,
            heading,
            radius,
            gyro: [dir; GYRO_BUFFER_LEN],
            gyro_count: GYRO_BUFFER_LEN,
            predictor: 0.0,
        }
    }

    #[prof]
    pub fn smoothed_heading(&self) -> f64 {
        if self.gyro_count == 0 {
            return self.heading;
        }
        let mut sum = Point::ZERO;
        for i in 0..self.gyro_count {
            sum += self.gyro[i];
        }
        let avg = sum / self.gyro_count as f64;
        let len = avg.length();
        if len < 1e-9 {
            return self.heading;
        }
        avg.y.atan2(avg.x)
    }

    #[prof]
    pub fn push_gyro(&mut self, dir: Point) {
        if GYRO_BUFFER_LEN == 0 {
            return;
        }
        for i in (1..GYRO_BUFFER_LEN).rev() {
            self.gyro[i] = self.gyro[i - 1];
        }
        self.gyro[0] = dir;
        if self.gyro_count < GYRO_BUFFER_LEN {
            self.gyro_count += 1;
        }
    }

    #[prof]
    pub fn reset_gyro(&mut self) {
        let dir = Point::new(self.heading.cos(), self.heading.sin());
        self.gyro = [dir; GYRO_BUFFER_LEN];
        self.gyro_count = 1;
        self.predictor = 0.0;
    }

    /// Update the decayed predictor.  Called only when a step
    /// converged (the deflection is trustworthy signal of real
    /// curvature, not a transient correction).  The new estimate is
    /// a low-pass blend of the previous predictor and the latest
    /// deflection, so steady curvature is tracked while one-off
    /// corrections decay away within a few steps.
    #[prof]
    pub fn update_predictor(&mut self, delta: f64) {
        self.predictor =
            PREDICTOR_DECAY * self.predictor + (1.0 - PREDICTOR_DECAY) * delta;
    }

    /// Predictor seed for [`step`].  Clamped to a fraction of
    /// `max_deflection` so a stale large estimate can never dominate
    /// the first solver trial and pin `best_error` on an overshoot.
    pub fn predicted_angle(&self, max_deflection: f64) -> f64 {
        let clamp = max_deflection * PREDICTOR_CLAMP_FRAC;
        self.predictor.clamp(-clamp, clamp)
    }

    /// Raw (un-clamped) predictor value, exposed for trace records so
    /// the inspector can show the true internal state.
    pub fn raw_predictor(&self) -> f64 {
        self.predictor
    }
}
