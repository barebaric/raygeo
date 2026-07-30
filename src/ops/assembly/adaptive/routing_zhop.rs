use prof_macros::prof;

use crate::ops::assembly::adaptive::routing::{RouteCtx, RoutingStrategy};
use crate::types::Point3D;

/// Z-hop routing: always succeeds by retracting to safe Z, traveling
/// directly at safe Z, then plunging to the target.  No obstacle check
/// needed since the tool is well above all material at safe Z.
pub struct RoutingZHop;

impl RoutingStrategy for RoutingZHop {
    fn label(&self) -> &'static str {
        "zhop"
    }

    #[prof]
    fn find_route(
        &self,
        ctx: &RouteCtx,
        from: Point3D,
        to: Point3D,
        _detail: &mut u8,
    ) -> Option<Vec<Point3D>> {
        let safe_z = ctx.opts.safe_z;
        let plunge_z = ctx.opts.target_z + 0.5;
        // path[0] is the starting position (skipped by emit_resume_travel),
        // so the retract is path[1] — emitted as the first move.
        Some(vec![
            from,
            Point3D::new(from.x, from.y, safe_z),
            Point3D::new(to.x, to.y, safe_z),
            Point3D::new(to.x, to.y, plunge_z),
        ])
    }
}
