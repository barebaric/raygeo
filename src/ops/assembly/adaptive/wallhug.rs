//! Wall-hug tracking for [`super::adaptive_clearing`].
//!
//! When the tool cutting edge reaches the pocket boundary, the tracker
//! records the minimum-distance pose per envelope visit.  These
//! recorded poses are used by the
//! [`ResumeWallHug`](super::resume_wall_hug) strategy to reposition
//! the tool after a stall.

use prof_macros::prof;

use crate::geo::shape::polygon::get_polygons_closest_point;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Point3D, Polygon};

// ── envelope_distance ────────────────────────────────────────────────

/// Minimum distance from `point` to the nearest boundary edge of any
/// polygon in `area`.
#[prof]
fn envelope_distance(point: Point, area: &[Polygon]) -> f64 {
    get_polygons_closest_point(area, point)
        .map(|(_, _, _, d2)| d2.sqrt())
        .unwrap_or(f64::MAX)
}

// ── WallHugSegments ───────────────────────────────────────────────────

/// Accumulates wall-hug poses across the current and previous cut
/// segments.  Current-segment points are collected during envelope
/// visits; on resume success the current segment is finalized and
/// older segments are preserved as fallback resume candidates.
struct WallHugSegments {
    current: Vec<ToolPose>,
    previous: Vec<Vec<ToolPose>>,
}

impl WallHugSegments {
    fn new() -> Self {
        Self {
            current: Vec::new(),
            previous: Vec::new(),
        }
    }

    fn push(&mut self, pose: ToolPose) {
        self.current.push(pose);
    }

    fn finalize_segment(&mut self) {
        if !self.current.is_empty() {
            self.previous.push(std::mem::take(&mut self.current));
        }
    }

    fn prune(&mut self, pos: Point3D, radius: f64) {
        let r2 = radius * radius;
        for segment in &mut self.previous {
            segment.retain(|p| {
                let dx = p.pos.x - pos.x;
                let dy = p.pos.y - pos.y;
                dx * dx + dy * dy > r2
            });
        }
        self.previous.retain(|s| !s.is_empty());
    }

    fn ordered_points(&self) -> Vec<ToolPose> {
        let total: usize = self.current.len()
            + self.previous.iter().map(|s| s.len()).sum::<usize>();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(&self.current);
        for segment in &self.previous {
            out.extend_from_slice(segment);
        }
        out
    }

    fn segment_counts(&self) -> Vec<u32> {
        let mut counts = Vec::with_capacity(1 + self.previous.len());
        counts.push(self.current.len() as u32);
        for segment in &self.previous {
            counts.push(segment.len() as u32);
        }
        counts
    }

    pub fn wall_hug_ref(&self) -> Vec<(f64, f64)> {
        self.ordered_points()
            .iter()
            .map(|p| (p.pos.x, p.pos.y))
            .collect()
    }

    pub fn segment_counts_ref(&self) -> Vec<u32> {
        self.segment_counts()
    }
}

// ── WallHugTracker ──────────────────────────────────────────────────

/// Tracks envelope proximity to record wall-hug poses for the
/// [`ResumeWallHug`](super::resume_wall_hug) strategy.
pub(super) struct WallHugTracker {
    segments: WallHugSegments,
    in_envelope: bool,
    tracking: Option<(ToolPose, f64)>,
}

impl WallHugTracker {
    pub fn new() -> Self {
        Self {
            segments: WallHugSegments::new(),
            in_envelope: false,
            tracking: None,
        }
    }

    /// Per-step wall-hug tracking.  Call on each successful step to
    /// detect envelope entry/departure and record the minimum-distance
    /// pose.
    pub fn on_step(
        &mut self,
        pos: Point3D,
        heading: f64,
        radius: f64,
        valid_tool_area: &[Polygon],
    ) {
        let dist = envelope_distance(
            crate::types::Point::new(pos.x, pos.y),
            valid_tool_area,
        );
        let inside = dist < radius + 1e-9;

        if !self.in_envelope && inside {
            self.in_envelope = true;
            self.tracking = Some((ToolPose { pos, heading }, dist));
        } else if self.in_envelope && !inside {
            if let Some((candidate, _)) = self.tracking.take() {
                self.segments.push(candidate);
            }
            self.in_envelope = false;
        } else if self.in_envelope && inside {
            if let Some((candidate, min_dist)) = self.tracking.take() {
                if dist < min_dist {
                    self.tracking = Some((ToolPose { pos, heading }, dist));
                } else if dist > min_dist + 1e-9 {
                    self.segments.push(candidate);
                    self.tracking = None;
                    self.in_envelope = false;
                } else {
                    self.tracking = Some((candidate, min_dist));
                }
            }
        }
    }

    /// Prune older-segment hug points that the tool has now swept.
    /// Call every iteration (not just on successful steps).
    pub fn prune(&mut self, pos: Point3D, radius: f64) {
        self.segments.prune(pos, radius);
    }

    /// Ordered wall-hug points (current segment first, then previous)
    /// for passing to [`ResumeCtx`](super::resume::ResumeCtx).
    pub fn ordered_points(&self) -> Vec<ToolPose> {
        self.segments.ordered_points()
    }

    /// Flat `(x, y)` pairs for the trace recorder.
    pub fn wall_hug_ref(&self) -> Vec<(f64, f64)> {
        self.segments.wall_hug_ref()
    }

    /// Per-segment point counts for the trace recorder.
    pub fn segment_counts_ref(&self) -> Vec<u32> {
        self.segments.segment_counts_ref()
    }

    /// Finalize the current segment and clear tracking state.
    /// Call after a successful resume.
    pub fn reset(&mut self) {
        self.in_envelope = false;
        self.tracking = None;
        self.segments.finalize_segment();
    }
}
