//! Travel-path routing strategies for resume / re-engagement moves.
//!
//! When the tool needs to move from its current position to a resume
//! position, [`optimize_route`] tries each [`RoutingStrategy`] in
//! priority order and returns the first safe path.

use prof_macros::prof;

use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::smooth::build_smoothed_path;
use crate::geo::shape::compute_polygon_bounds;
use crate::geo::shape::does_path_sweep_intersect_polygon;
use crate::ops::assembly::adaptive::AdaptiveClearingOptions;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::Part;
use crate::types::{Point, Point3D, Polygon, Rect};

use super::chain::StrategyChain;

pub use super::routing_direct::RoutingDirect;
pub use super::routing_frontier::RoutingFrontier;
pub use super::routing_mat::RoutingMat;
pub use super::routing_zhop::RoutingZHop;

// ── RoutingContext ─────────────────────────────────────────────────

/// Read-only snapshot of everything a routing strategy may need.
pub struct RouteCtx<'a> {
    pub cleared: &'a ClearedArea,
    pub opts: &'a AdaptiveClearingOptions,
    pub mat: Option<&'a MedialAxis>,
    pub obstacles: &'a [Polygon],
    pub obstacle_bounds: &'a [Rect],
    pub part: &'a Part,
}

// ── Detail codes (shared across all routing strategies) ────────────

pub const ROUTE_OK: u8 = 0;

// Direct
pub const ROUTE_DIRECT_SWEEP_COLLIDE: u8 = 1;

// Frontier
pub const ROUTE_FRONTIER_NO_OBSTACLES: u8 = 2;
pub const ROUTE_FRONTIER_NO_FRONTIER: u8 = 3;
pub const ROUTE_FRONTIER_OFFSET_EMPTY: u8 = 4;
pub const ROUTE_FRONTIER_DIFFERENT_POLYGONS: u8 = 5;
pub const ROUTE_FRONTIER_TOO_FEW_VERTS: u8 = 6;
pub const ROUTE_FRONTIER_SAME_VERTEX: u8 = 7;
pub const ROUTE_FRONTIER_SWEEP_COLLIDE: u8 = 8;

// MAT
pub const ROUTE_MAT_NO_AXIS: u8 = 9;
pub const ROUTE_MAT_NO_CLEARED: u8 = 10;
pub const ROUTE_MAT_NO_PATH: u8 = 11;
pub const ROUTE_MAT_SWEEP_COLLIDE: u8 = 12;

// ZHop
pub const ROUTE_ZHOP_OK: u8 = 17;

/// Per-strategy detail label (for logging / error messages).
pub fn route_detail_label(code: u8) -> &'static str {
    match code {
        ROUTE_DIRECT_SWEEP_COLLIDE => "sweep_collide",
        ROUTE_FRONTIER_NO_OBSTACLES => "no_obstacles",
        ROUTE_FRONTIER_NO_FRONTIER => "no_frontier",
        ROUTE_FRONTIER_OFFSET_EMPTY => "offset_empty",
        ROUTE_FRONTIER_DIFFERENT_POLYGONS => "diff_polygons",
        ROUTE_FRONTIER_TOO_FEW_VERTS => "too_few_verts",
        ROUTE_FRONTIER_SAME_VERTEX => "same_vertex",
        ROUTE_FRONTIER_SWEEP_COLLIDE => "sweep_collide",
        ROUTE_MAT_NO_AXIS => "no_axis",
        ROUTE_MAT_NO_CLEARED => "no_cleared",
        ROUTE_MAT_NO_PATH => "no_path",
        ROUTE_MAT_SWEEP_COLLIDE => "sweep_collide",
        ROUTE_ZHOP_OK => "zhop_ok",
        _ => "unknown",
    }
}

// ── RoutingStrategy trait ──────────────────────────────────────────

/// A strategy that finds a safe travel path between two points.
pub trait RoutingStrategy {
    fn label(&self) -> &'static str;

    /// Return a sequence of waypoints (excluding `from`, including `to`)
    /// that the tool can safely travel through.
    ///
    /// On failure `detail` is set to one of the `ROUTE_*` constants above.
    fn find_route(
        &self,
        ctx: &RouteCtx,
        from: Point3D,
        to: Point3D,
        detail: &mut u8,
    ) -> Option<Vec<Point3D>>;
}

// ── Strategy enum ─────────────────────────────────────────────────

/// Which routing strategy produced the winning path, in priority order.
#[derive(Clone, Copy, PartialEq, Eq, num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum RouteSource {
    /// No strategy was tried / applicable (trace-record sentinel).
    None = 0,
    /// Direct straight-line travel (centreline does not cross obstacles).
    RoutingDirect = 1,
    /// Frontier-walking travel along the cleared-area boundary.
    RoutingFrontier = 2,
    /// MAT-guided travel through the cleared fragments.
    RoutingMat = 3,
    /// Safe-Z direct travel (retract → direct move → plunge).
    RoutingZHop = 4,
}

/// Source label for a given strategy.
pub(crate) fn source_label(source: RouteSource) -> &'static str {
    match source {
        RouteSource::None => "none",
        RouteSource::RoutingDirect => "direct",
        RouteSource::RoutingFrontier => "frontier",
        RouteSource::RoutingMat => "mat",
        RouteSource::RoutingZHop => "zhop",
    }
}

// ── Sweep-clearance helper ──────────────────────────────────────────

/// True when the tool-disc sweep along `path` does NOT intersect any
/// obstacle polygon.  When there are no obstacles the path is always
/// considered clear.
pub(super) fn sweep_clear(path: &[Point3D], ctx: &RouteCtx) -> bool {
    if ctx.obstacles.is_empty() {
        return true;
    }
    let path_2d: Vec<Point> =
        path.iter().map(|p| Point::new(p.x, p.y)).collect();
    !does_path_sweep_intersect_polygon(
        &path_2d,
        ctx.opts.tool_radius,
        ctx.obstacles,
        ctx.obstacle_bounds,
    )
}

// ── Smoothing ──────────────────────────────────────────────────────

/// Smooth a raw waypoint path against obstacles.
///
/// `from` is the tool's current position (preserved as path start).
/// `raw` is the waypoint list returned by a routing strategy.
#[prof]
pub(crate) fn smooth_route(
    from: Point3D,
    raw: &[Point3D],
    obstacles: &[Polygon],
    clearance: f64,
) -> Vec<Point3D> {
    if raw.is_empty() {
        return vec![from];
    }
    let last = raw[raw.len() - 1];
    if raw.len() == 1 {
        return vec![from, last];
    }
    let waypoints_3d: Vec<Point3D> = raw[..raw.len() - 1].to_vec();
    let obs_bounds = compute_polygon_bounds(obstacles);
    let from_2d = Point::new(from.x, from.y);
    let last_2d = Point::new(last.x, last.y);
    let waypoints_2d: Vec<Point> =
        waypoints_3d.iter().map(|p| Point::new(p.x, p.y)).collect();
    let smoothed_2d = build_smoothed_path(
        from_2d,
        last_2d,
        &waypoints_2d,
        obstacles,
        &obs_bounds,
        clearance,
        120,
    );
    if smoothed_2d.is_empty() {
        vec![from, last]
    } else {
        let z = from.z.max(last.z);
        smoothed_2d
            .iter()
            .map(|p| Point3D::new(p.x, p.y, z))
            .collect()
    }
}

// ── optimize_route orchestrator ────────────────────────────────────

/// Try each routing strategy in priority order and return the first
/// successful path, already smoothed against obstacles.
///
/// `details` is filled with the per-strategy failure detail (or 0 on
/// success for the winning strategy).
///
/// Returns `(source, smoothed_path)` where `smoothed_path` is the full
/// travel polyline from `from` to `to`.
#[prof]
pub fn optimize_route<'a>(
    ctx: &RouteCtx<'a>,
    from: Point3D,
    to: Point3D,
    details: &mut [u8; 4],
) -> Option<(RouteSource, Vec<Point3D>)> {
    let mut chain: StrategyChain<&dyn RoutingStrategy, RouteSource, 4> =
        StrategyChain::new([
            (&RoutingDirect, RouteSource::RoutingDirect),
            (&RoutingFrontier, RouteSource::RoutingFrontier),
            (&RoutingMat, RouteSource::RoutingMat),
            (&RoutingZHop, RouteSource::RoutingZHop),
        ]);

    let result = chain.run(
        |_idx, s, _source, detail| s.find_route(ctx, from, to, detail),
        Some(
            |_: usize,
             s: &dyn RoutingStrategy,
             source: RouteSource,
             detail: &mut u8,
             path: Vec<Point3D>|
             -> Option<Vec<Point3D>> {
                let smoothed = smooth_route(
                    from,
                    &path,
                    ctx.obstacles,
                    ctx.opts.tool_radius,
                );
                if smoothed.len() >= 2 {
                    *detail = ROUTE_OK; // success
                    crate::dbg_log!(
                        "  ROUTE  {}={}  from=({:.3},{:.3})  \
                     to=({:.3},{:.3})  n={}",
                        source as u8,
                        s.label(),
                        from.x,
                        from.y,
                        to.x,
                        to.y,
                        smoothed.len(),
                    );
                    Some(smoothed)
                } else {
                    None
                }
            },
        ),
    );

    *details = *chain.details();
    result
}
