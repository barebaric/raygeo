use crate::geo::algo::astar;
use crate::geo::shape::polygon::get_polygon_group_bounds;
use crate::ops::assembly::adaptive::routing::{
    RouteCtx, RoutingStrategy, ROUTE_ASTAR_FAILED, ROUTE_ASTAR_NO_FREE_SPACE,
    ROUTE_ASTAR_NO_OBSTACLES, ROUTE_ASTAR_TOO_FEW_WAYPOINTS,
};
use crate::types::Point;

/// Grid-based A* routing: rasterises the cleared fragments at ~2000
/// cells on the longest side and finds a path that avoids obstacles
/// with a tool-radius safety margin.
///
/// Free space = the cleared fragments (where the tool disc has already
/// been, so the tool centre is safe).  Remaining stock within those
/// fragments is dilated by `tool_radius` so the path never collides
/// with uncut material.
///
/// This is the most expensive strategy (O(grid cells)) and should only
/// be tried after Direct and MAT have both failed.
pub struct RoutingAStar;

impl RoutingStrategy for RoutingAStar {
    fn label(&self) -> &'static str {
        "astar"
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
            *detail = ROUTE_ASTAR_NO_OBSTACLES;
            return None;
        }

        // Free space = the cleared fragments  (the area already cut).
        let free_space = ctx.cleared.fragments();
        if free_space.is_empty() {
            *detail = ROUTE_ASTAR_NO_FREE_SPACE;
            return None;
        }

        // Cell size: longest side ≈ 2000 cells, floor at 0.1 mm.
        let bounds = get_polygon_group_bounds(free_space);
        let longest =
            (bounds.max.x - bounds.min.x).max(bounds.max.y - bounds.min.y);
        let cell_size = (longest / 2000.0).max(0.1);

        let result = match astar::find_path(
            from,
            to,
            free_space,
            obstacles,
            ctx.opts.radius,
            cell_size,
        ) {
            Some(r) => r,
            None => {
                *detail = ROUTE_ASTAR_FAILED;
                return None;
            }
        };

        if result.waypoints.len() < 2 {
            *detail = ROUTE_ASTAR_TOO_FEW_WAYPOINTS;
            return None;
        }

        Some(result.waypoints)
    }
}
