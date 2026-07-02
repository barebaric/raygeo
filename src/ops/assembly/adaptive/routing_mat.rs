use crate::ops::assembly::adaptive::routing::{RouteCtx, RoutingStrategy};
use crate::types::Point;

/// MAT-guided routing: use the pocket-wide MAT (built once at startup) to
/// find a path through cleared territory via the medial-axis skeleton.
///
/// Only nodes inside the current cleared fragments are visited, so the
/// waypoints stay inside already-cut territory.  The entry/exit segments
/// (from the tool position to the nearest MAT node) are not individually
/// validated, but the path itself follows the tree skeleton.
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
    ) -> Option<Vec<Point>> {
        let axis = ctx.mat?;
        let cleared = ctx.cleared.fragments();
        if cleared.is_empty() {
            return None;
        }

        axis.path_between_cleared(from, to, cleared)
    }
}
