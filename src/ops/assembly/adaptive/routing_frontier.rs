use prof_macros::prof;

use crate::geo::shape::polygon::get_polygons_closest_point;
use crate::geo::shape::polygon::offset_polygon;
use crate::geo::shape::polygon::resample_polygon;
use crate::geo::shape::polygon::JoinStyle;
use crate::ops::assembly::adaptive::routing::{
    sweep_clear, RouteCtx, RoutingStrategy, ROUTE_FRONTIER_DIFFERENT_POLYGONS,
    ROUTE_FRONTIER_NO_FRONTIER, ROUTE_FRONTIER_NO_OBSTACLES,
    ROUTE_FRONTIER_OFFSET_EMPTY, ROUTE_FRONTIER_SAME_VERTEX,
    ROUTE_FRONTIER_SWEEP_COLLIDE, ROUTE_FRONTIER_TOO_FEW_VERTS,
};
use crate::types::{Point, Point3D};

/// Frontier-walking routing: find the shortest path along the cleared-area
/// frontier between `from` and `to`, then offset the segment inward by
/// `tool_radius + tolerance` so the tool disc stays inside cleared territory.
///
/// This strategy sits between Direct and MAT in priority order.
pub struct RoutingFrontier;

impl RoutingStrategy for RoutingFrontier {
    fn label(&self) -> &'static str {
        "frontier"
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
            *detail = ROUTE_FRONTIER_NO_OBSTACLES;
            return None;
        }

        crate::dbg_log!(
            "  FRONTIER  try  from=({:.3},{:.3})  to=({:.3},{:.3})",
            from.x,
            from.y,
            to.x,
            to.y,
        );

        let spacing = ctx.opts.tool_radius * 0.3;
        let offset_dist = ctx.opts.tool_radius + spacing;

        // Walk the frontier: offset each polygon inward by offset_dist,
        // then find the shortest segment between the closest points.
        let frontier = ctx.cleared.frontier(0.5);
        if frontier.is_empty() {
            crate::dbg_log!("  FRONTIER  no frontier");
            *detail = ROUTE_FRONTIER_NO_FRONTIER;
            return None;
        }

        // Offset inward.
        let inner: Vec<Vec<Point>> = frontier
            .iter()
            .filter_map(|poly| {
                let result =
                    offset_polygon(poly, -offset_dist, JoinStyle::Round);
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            })
            .flatten()
            .collect();
        if inner.is_empty() {
            crate::dbg_log!("  FRONTIER  offset empty");
            *detail = ROUTE_FRONTIER_OFFSET_EMPTY;
            return None;
        }

        // Find the offset polygon closest to `from` (projected to XY).
        let from_2d = Point::new(from.x, from.y);
        let to_2d = Point::new(to.x, to.y);
        let (pi, _ft, fp, _fd2) = get_polygons_closest_point(&inner, from_2d)?;
        let poly = &inner[pi];

        // `to` must be closest to the same offset polygon.
        let (tj, _tt, tp, _td2) = get_polygons_closest_point(&inner, to_2d)?;
        if tj != pi {
            crate::dbg_log!("  FRONTIER  different polygons {} {}", pi, tj);
            *detail = ROUTE_FRONTIER_DIFFERENT_POLYGONS;
            return None;
        }

        // Resample so vertices approximate arc-length uniformly.
        let dense = resample_polygon(poly, spacing);
        let n = dense.len();
        if n < 4 {
            *detail = ROUTE_FRONTIER_TOO_FEW_VERTS;
            return None;
        }

        // Find the dense vertex nearest fp.
        let nearest = |target: Point| -> Option<usize> {
            let mut best = (0usize, f64::MAX);
            for (i, &v) in dense.iter().enumerate() {
                let d = v.distance(target);
                if d < best.1 {
                    best = (i, d);
                }
            }
            Some(best.0)
        };
        let fi = nearest(fp)?;
        let ti = nearest(tp)?;
        if fi == ti {
            *detail = ROUTE_FRONTIER_SAME_VERTEX;
            return None;
        }

        // Walk both directions, pick shorter.
        let walk = |forward: bool| -> Vec<Point> {
            let mut pts = Vec::new();
            let mut idx = fi;
            loop {
                if forward {
                    pts.push(dense[idx]);
                    idx = (idx + 1) % n;
                } else {
                    pts.push(dense[idx]);
                    idx = if idx == 0 { n - 1 } else { idx - 1 };
                }
                if idx == ti {
                    pts.push(dense[ti]);
                    break;
                }
            }
            pts
        };
        let fwd = walk(true);
        let rev = walk(false);
        let mut seg = if fwd.len() <= rev.len() { fwd } else { rev };

        // Replace first/last with exact closest points on the offset poly.
        if !seg.is_empty() {
            seg[0] = fp;
            *seg.last_mut().unwrap() = tp;
        }

        // Lift to 3D for sweep check and return.
        let route_z = from.z.max(to.z) + 0.1;
        let seg_3d: Vec<Point3D> = seg
            .into_iter()
            .map(|p| Point3D::new(p.x, p.y, route_z))
            .collect();

        crate::dbg_log!("  FRONTIER  seg_len={}", seg_3d.len());

        // Sweep check.
        if !sweep_clear(&seg_3d, ctx) {
            crate::dbg_log!("  FRONTIER  sweep collide");
            *detail = ROUTE_FRONTIER_SWEEP_COLLIDE;
            return None;
        }

        Some(seg_3d)
    }
}
