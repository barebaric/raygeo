use prof_macros::prof;

use crate::ops::assembly::adaptive::routing::{
    sweep_clear, RouteCtx, RoutingStrategy, ROUTE_MAT_NO_AXIS,
    ROUTE_MAT_NO_CLEARED, ROUTE_MAT_NO_PATH, ROUTE_MAT_SWEEP_COLLIDE,
};
use crate::types::{Point, Point3D};

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

    #[prof]
    fn find_route(
        &self,
        ctx: &RouteCtx,
        from: Point3D,
        to: Point3D,
        detail: &mut u8,
    ) -> Option<Vec<Point3D>> {
        let axis = match ctx.mat {
            Some(m) => m,
            None => {
                *detail = ROUTE_MAT_NO_AXIS;
                return None;
            }
        };
        let cleared = ctx.part.cleared.fragments();
        if cleared.is_empty() {
            *detail = ROUTE_MAT_NO_CLEARED;
            return None;
        }

        let from_2d = Point::new(from.x, from.y);
        let to_2d = Point::new(to.x, to.y);

        let path = match axis.path_between_cleared(from_2d, to_2d, cleared) {
            Some(p) => p,
            None => {
                *detail = ROUTE_MAT_NO_PATH;
                return None;
            }
        };

        let route_z = from.z.max(to.z) + 0.1;
        let path_3d: Vec<Point3D> = path
            .iter()
            .map(|p| Point3D::new(p.x, p.y, route_z))
            .collect();

        if !path_3d.is_empty() {
            // Build the full travel polyline: from → waypoints → to.
            let mut travel = Vec::with_capacity(path_3d.len() + 2);
            travel.push(from);
            travel.extend_from_slice(&path_3d);
            travel.push(to);

            // Tool-disc sweep must not intersect any obstacle.
            if !sweep_clear(&travel, ctx) {
                *detail = ROUTE_MAT_SWEEP_COLLIDE;
                return None;
            }
        }

        Some(path_3d)
    }
}
