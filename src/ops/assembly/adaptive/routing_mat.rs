use crate::ops::assembly::adaptive::routing::{
    sweep_clear, RouteCtx, RoutingStrategy, ROUTE_MAT_NO_AXIS,
    ROUTE_MAT_NO_CLEARED, ROUTE_MAT_NO_PATH, ROUTE_MAT_SWEEP_COLLIDE,
};
use crate::types::Point;

/// MAT-guided routing: use the pocket-wide MAT (built once at startup) to
/// find a path through cleared territory via the medial-axis skeleton.
///
/// Nodes are filtered to those inside cleared fragments, and every
/// straight-line segment between consecutive waypoints is validated
/// against the obstacle polygons via a tool-disc sweep check so the path
/// never collides with uncut material.
pub struct RoutingMat;

impl RoutingStrategy for RoutingMat {
    fn label(&self) -> &'static str {
        "mat"
    }

    fn find_route(
        &self,
        ctx: &RouteCtx,
        from: Point,
        to: Point,
        detail: &mut u8,
    ) -> Option<Vec<Point>> {
        let axis = match ctx.mat {
            Some(m) => m,
            None => {
                *detail = ROUTE_MAT_NO_AXIS;
                return None;
            }
        };
        let cleared = ctx.cleared.fragments();
        if cleared.is_empty() {
            *detail = ROUTE_MAT_NO_CLEARED;
            return None;
        }

        let path = match axis.path_between_cleared(from, to, cleared) {
            Some(p) => p,
            None => {
                *detail = ROUTE_MAT_NO_PATH;
                return None;
            }
        };

        if !path.is_empty() {
            // Build the full travel polyline: from → waypoints → to.
            let mut travel = Vec::with_capacity(path.len() + 2);
            travel.push(from);
            travel.extend_from_slice(&path);
            travel.push(to);

            // Tool-disc sweep must not intersect any obstacle.
            if !sweep_clear(&travel, ctx) {
                *detail = ROUTE_MAT_SWEEP_COLLIDE;
                return None;
            }
        }

        Some(path)
    }
}
