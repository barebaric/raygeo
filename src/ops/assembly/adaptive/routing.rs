//! Travel-path routing strategies for resume / re-engagement moves.
//!
//! When the tool needs to move from its current position to a resume
//! position, [`optimize_route`] tries each [`RoutingStrategy`] in
//! priority order and returns the first safe path.

use prof_macros::prof;

use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::smooth::build_smoothed_path;
use crate::geo::shape::compute_polygon_bounds;
use crate::ops::assembly::adaptive::AdaptiveClearingOptions;
use crate::ops::cut::ClearedArea;
use crate::types::{Point, Polygon, Rect};

pub use super::routing_direct::RoutingDirect;
pub use super::routing_mat::RoutingMat;

// ── RoutingContext ─────────────────────────────────────────────────

/// Read-only snapshot of everything a routing strategy may need.
pub struct RouteCtx<'a> {
    pub cleared: &'a ClearedArea,
    pub opts: &'a AdaptiveClearingOptions,
    pub mat: Option<&'a MedialAxis>,
    pub obstacles: &'a [Polygon],
    pub obstacle_bounds: &'a [Rect],
}

// ── RoutingStrategy trait ──────────────────────────────────────────

/// A strategy that finds a safe travel path between two points.
pub trait RoutingStrategy {
    fn label(&self) -> &'static str;

    /// Return a sequence of waypoints (excluding `from`, including `to`)
    /// that the tool can safely travel through.
    fn find_route(
        &self,
        ctx: &RouteCtx,
        from: Point,
        to: Point,
    ) -> Option<Vec<Point>>;
}

// ── Strategy enum ─────────────────────────────────────────────────

/// Which routing strategy produced the winning path, in priority order.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RouteSource {
    /// Direct straight-line travel (centreline does not cross obstacles).
    RoutingDirect = 1,
    /// MAT-guided travel through the cleared fragments.
    RoutingMat = 2,
}

/// Source label for a given strategy.
pub(crate) fn source_label(source: RouteSource) -> &'static str {
    match source {
        RouteSource::RoutingDirect => "direct",
        RouteSource::RoutingMat => "mat",
    }
}

// ── Smoothing ──────────────────────────────────────────────────────

/// Smooth a raw waypoint path against obstacles.
///
/// `from` is the tool's current position (preserved as path start).
/// `raw` is the waypoint list returned by a routing strategy.
#[prof]
pub(crate) fn smooth_route(
    from: Point,
    raw: &[Point],
    obstacles: &[Polygon],
    clearance: f64,
) -> Vec<Point> {
    if raw.is_empty() {
        return vec![from];
    }
    let last = raw[raw.len() - 1];
    // Fast path: just [from, to] — nothing to smooth.
    if raw.len() == 1 {
        return vec![from, last];
    }
    let waypoints: Vec<Point> = raw[..raw.len() - 1].to_vec();
    let obs_bounds = compute_polygon_bounds(obstacles);
    let smoothed = build_smoothed_path(
        from,
        last,
        &waypoints,
        obstacles,
        &obs_bounds,
        clearance,
        120,
    );
    if smoothed.is_empty() {
        vec![from, last]
    } else {
        smoothed
    }
}

// ── optimize_route orchestrator ────────────────────────────────────

/// Try each routing strategy in priority order and return the first
/// successful path, already smoothed against obstacles.
///
/// Returns `(source, smoothed_path)` where `smoothed_path` is the full
/// travel polyline from `from` to `to`.
#[prof]
pub fn optimize_route<'a>(
    ctx: &RouteCtx<'a>,
    from: Point,
    to: Point,
) -> Option<(RouteSource, Vec<Point>)> {
    let strategies: [(&dyn RoutingStrategy, RouteSource); 2] = [
        (&RoutingDirect, RouteSource::RoutingDirect),
        (&RoutingMat, RouteSource::RoutingMat),
    ];

    for (s, source) in &strategies {
        if let Some(path) = s.find_route(ctx, from, to) {
            let smoothed =
                smooth_route(from, &path, ctx.obstacles, ctx.opts.radius);
            if smoothed.len() >= 2 {
                crate::dbg_log!(
                    "  ROUTE  {}={}  from=({:.3},{:.3})  \
                     to=({:.3},{:.3})  n={}",
                    *source as u8,
                    s.label(),
                    from.x,
                    from.y,
                    to.x,
                    to.y,
                    smoothed.len(),
                );
                return Some((*source, smoothed));
            }
        }
    }

    None
}
