use crate::geo::shape::does_path_sweep_intersect_polygon;
use crate::ops::assembly::adaptive::routing::{
    RouteCtx, RoutingStrategy, ROUTE_DIRECT_SWEEP_COLLIDE,
};
use crate::types::Point;

/// Direct-line routing: accept the straight segment from `from` to `to`
/// only when the tool-disc sweep along it does not intersect any
/// obstacle polygon.
pub struct RoutingDirect;

impl RoutingStrategy for RoutingDirect {
    fn label(&self) -> &'static str {
        "direct"
    }

    fn find_route(
        &self,
        ctx: &RouteCtx,
        from: Point,
        to: Point,
        detail: &mut u8,
    ) -> Option<Vec<Point>> {
        let obstacles = ctx.obstacles;
        if obstacles.is_empty() {
            return Some(vec![to]);
        }

        // Shorten the segment by `tolerance` at each end to avoid false
        // positives from the edge-proximity check when the tool centre is
        // at distance ≈ radius (numerical boundary of remaining material).
        let eps = ctx.opts.tolerance;
        let dir = to - from;
        let len = dir.length();
        let (a, b) = if len > 2.0 * eps {
            let d = dir / len;
            (from + d * eps, to - d * eps)
        } else {
            (from, to)
        };

        let seg = vec![a, b];
        if does_path_sweep_intersect_polygon(
            &seg,
            ctx.opts.radius,
            obstacles,
            ctx.obstacle_bounds,
        ) {
            *detail = ROUTE_DIRECT_SWEEP_COLLIDE;
            return None;
        }

        Some(vec![to])
    }
}
