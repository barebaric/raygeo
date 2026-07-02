use crate::geo::shape::does_line_cross_polygon;
use crate::geo::shape::get_polygon_signed_area;
use crate::geo::shape::is_point_in_polygon;
use crate::ops::assembly::adaptive::routing::{RouteCtx, RoutingStrategy};
use crate::types::Point;

/// Direct-line routing: accept the straight segment from `from` to `to`
/// only when its centreline does not cross any obstacle polygon boundary.
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
    ) -> Option<Vec<Point>> {
        let obstacles = ctx.obstacles;
        if obstacles.is_empty() {
            return Some(vec![to]);
        }

        let bounds = ctx.obstacle_bounds;
        let signs: Vec<i8> = obstacles
            .iter()
            .map(|obs| {
                if get_polygon_signed_area(obs) > 0.0 {
                    1
                } else {
                    -1
                }
            })
            .collect();

        // Winding-number point-in-region test using NonZero rule.
        let in_remaining = |p: Point| -> bool {
            let mut winding = 0i32;
            for ((obs, b), &sign) in obstacles.iter().zip(bounds).zip(&signs) {
                if obs.len() < 3 {
                    continue;
                }
                if p.x < b.min.x
                    || p.x > b.max.x
                    || p.y < b.min.y
                    || p.y > b.max.y
                {
                    continue;
                }
                if is_point_in_polygon(p, obs) {
                    winding += sign as i32;
                }
            }
            winding > 0
        };

        // Start point MUST be in cleared territory.
        if in_remaining(from) {
            return None;
        }

        // Centreline must not cross any obstacle boundary.
        if obstacles
            .iter()
            .all(|obs| !does_line_cross_polygon(from, to, obs))
        {
            Some(vec![to])
        } else {
            None
        }
    }
}
