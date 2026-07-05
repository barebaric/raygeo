use prof_macros::prof;

use crate::ops::assembly::adaptive::routing::{
    sweep_clear, RouteCtx, RoutingStrategy, ROUTE_DIRECT_SWEEP_COLLIDE,
};
use crate::types::Point3D;

/// Direct-line routing: accept the straight segment from `from` to `to`
/// only when the tool-disc sweep along it does not intersect any
/// obstacle polygon.
pub struct RoutingDirect;

impl RoutingStrategy for RoutingDirect {
    fn label(&self) -> &'static str {
        "direct"
    }

    #[prof]
    fn find_route(
        &self,
        ctx: &RouteCtx,
        from: Point3D,
        to: Point3D,
        detail: &mut u8,
    ) -> Option<Vec<Point3D>> {
        if ctx.obstacles.is_empty() {
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
        if !sweep_clear(&seg, ctx) {
            *detail = ROUTE_DIRECT_SWEEP_COLLIDE;
            return None;
        }

        let route_z = from.z.max(to.z) + 0.1;
        Some(vec![Point3D::new(to.x, to.y, route_z)])
    }
}
