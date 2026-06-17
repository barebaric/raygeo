//! Motion assembly: polyline-to-Ops conversion and pass linking.
//!
//! Bridges Tier-1 pure geometry (polylines as `Vec<Point3D>`) and the
//! [`Ops`] container. Domain-neutral — no `Tool`, no machine state.

use super::container::Ops;
use super::enums::CommandCategory;
use crate::types::Point3D;

/// Strategy for linking consecutive machining passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStrategy {
    /// Retract to `safe_z` between passes, move XY at that height,
    /// then descend to the next pass start Z.
    Retract,
    /// Move directly from the previous pass end to the next pass
    /// start without retracting.
    StayDown,
}

/// Convert a 3D polyline into an [`Ops`] command sequence.
///
/// When `move_first` is `true` the first point is emitted as a
/// [`MoveTo`][super::enums::CommandType::MoveTo] and subsequent points
/// as [`LineTo`][super::enums::CommandType::LineTo] (the normal case for
/// starting a new cutting path).
///
/// When `move_first` is `false` every point is emitted as a `LineTo`,
/// useful for appending a polyline to an in-progress cut (the cutter
/// starts cutting from its current position with no preceding travel
/// move).
pub fn polyline_to_ops(polyline: &[Point3D], move_first: bool) -> Ops {
    let mut ops = Ops::new();

    if polyline.is_empty() {
        return ops;
    }

    if move_first {
        let first = polyline[0];
        ops.move_to(first.x, first.y, first.z, None);
        for pt in &polyline[1..] {
            ops.line_to(pt.x, pt.y, pt.z, None);
        }
    } else {
        for pt in polyline {
            ops.line_to(pt.x, pt.y, pt.z, None);
        }
    }

    ops
}

// ── Helpers to find pass boundaries ──

fn pass_entry(ops: &Ops) -> Option<Point3D> {
    for i in 0..ops.len() {
        if ops.is_travel(i) {
            return Some(ops.endpoint(i));
        }
    }
    for i in 0..ops.len() {
        if ops.category(i) == CommandCategory::Moving {
            return Some(ops.endpoint(i));
        }
    }
    None
}

fn pass_exit(ops: &Ops) -> Option<Point3D> {
    for i in (0..ops.len()).rev() {
        if ops.category(i) == CommandCategory::Moving {
            return Some(ops.endpoint(i));
        }
    }
    None
}

/// Join ordered machining passes into a single [`Ops`] sequence with
/// travel moves between them according to `strategy`.
///
/// * `Retract` — between passes the tool retracts to `safe_z`, moves
///   XY at that height, then plunges to the next pass start Z.
/// * `StayDown` — the tool moves directly from the previous pass end to
///   the next pass start without retracting.
///
/// The first pass is emitted as-is; subsequent passes are prefixed with
/// the appropriate travel move(s) and then appended.
///
/// Returns an empty `Ops` when `passes` is empty.
pub fn link_passes(passes: &[Ops], safe_z: f64, strategy: LinkStrategy) -> Ops {
    if passes.is_empty() {
        return Ops::new();
    }

    let mut result = Ops::new();
    result.extend(&passes[0]);

    for pass in &passes[1..] {
        let prev_exit = match pass_exit(&result) {
            Some(p) => p,
            None => continue,
        };
        let entry = match pass_entry(pass) {
            Some(p) => p,
            None => continue,
        };

        match strategy {
            LinkStrategy::Retract => {
                result.move_to(prev_exit.x, prev_exit.y, safe_z, None);
                if (entry.x - prev_exit.x).abs() > 1e-12
                    || (entry.y - prev_exit.y).abs() > 1e-12
                {
                    result.move_to(entry.x, entry.y, safe_z, None);
                }
                if (entry.z - safe_z).abs() > 1e-12 {
                    result.move_to(entry.x, entry.y, entry.z, None);
                }
            }
            LinkStrategy::StayDown => {
                result.move_to(entry.x, entry.y, entry.z, None);
            }
        }

        result.extend(pass);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64, z: f64) -> Point3D {
        Point3D::new(x, y, z)
    }

    // ── polyline_to_ops ──

    #[test]
    fn test_polyline_to_ops_empty() {
        let ops = polyline_to_ops(&[], true);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_polyline_to_ops_single_point_with_move() {
        let poly = [pt(10.0, 20.0, 0.0)];
        let ops = polyline_to_ops(&poly, true);
        assert_eq!(ops.len(), 1);
        assert!(ops.is_travel(0));
        let ep = ops.endpoint(0);
        assert!((ep.x - 10.0).abs() < 1e-12);
        assert!((ep.y - 20.0).abs() < 1e-12);
    }

    #[test]
    fn test_polyline_to_ops_single_point_no_move() {
        let poly = [pt(10.0, 20.0, 0.0)];
        let ops = polyline_to_ops(&poly, false);
        assert_eq!(ops.len(), 1);
        assert!(ops.is_cutting(0));
    }

    #[test]
    fn test_polyline_to_ops_with_move() {
        let poly = [
            pt(10.0, 10.0, 0.0),
            pt(20.0, 20.0, 0.0),
            pt(30.0, 10.0, 0.0),
        ];
        let ops = polyline_to_ops(&poly, true);
        assert_eq!(ops.len(), 3);
        assert!(ops.is_travel(0));
        assert!(ops.is_cutting(1));
        assert!(ops.is_cutting(2));
    }

    #[test]
    fn test_polyline_to_ops_no_move() {
        let poly = [pt(10.0, 10.0, 0.0), pt(20.0, 20.0, 0.0)];
        let ops = polyline_to_ops(&poly, false);
        assert_eq!(ops.len(), 2);
        assert!(ops.is_cutting(0));
        assert!(ops.is_cutting(1));
    }

    #[test]
    fn test_polyline_to_ops_3d_z_preserved() {
        let poly = [pt(0.0, 0.0, 5.0), pt(10.0, 0.0, 0.0), pt(20.0, 0.0, -5.0)];
        let ops = polyline_to_ops(&poly, true);
        assert!((ops.endpoint(0).z - 5.0).abs() < 1e-12);
        assert!((ops.endpoint(1).z - 0.0).abs() < 1e-12);
        assert!((ops.endpoint(2).z + 5.0).abs() < 1e-12);
    }

    // ── link_passes ──

    fn make_pass(start: (f64, f64), end: (f64, f64), z: f64) -> Ops {
        let mut ops = Ops::new();
        ops.move_to(start.0, start.1, z, None);
        ops.line_to(end.0, end.1, z, None);
        ops
    }

    #[test]
    fn test_link_passes_empty() {
        let result = link_passes(&[], 10.0, LinkStrategy::Retract);
        assert!(result.is_empty());
    }

    #[test]
    fn test_link_passes_single() {
        let pass = make_pass((0.0, 0.0), (10.0, 0.0), 0.0);
        let result = link_passes(&[pass.clone()], 10.0, LinkStrategy::Retract);
        assert_eq!(result.len(), 2); // MoveTo + LineTo
        assert!((result.endpoint(1).x - 10.0).abs() < 1e-12);
    }

    #[test]
    fn test_link_passes_stay_down() {
        let p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0);
        let p2 = make_pass((20.0, 0.0), (30.0, 0.0), 0.0);
        let result = link_passes(&[p1, p2], 10.0, LinkStrategy::StayDown);
        // pass1 (2 cmds) + travel MoveTo + pass2 (2 cmds) = 5
        assert_eq!(result.len(), 5);
        // The travel move (index 2) should go to (20, 0, 0)
        let travel_end = result.endpoint(2);
        assert!((travel_end.x - 20.0).abs() < 1e-12);
        assert!((travel_end.z - 0.0).abs() < 1e-12);
    }

    #[test]
    fn test_link_passes_retract() {
        let p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0);
        let p2 = make_pass((20.0, 0.0), (30.0, 0.0), -5.0);
        let result = link_passes(&[p1, p2], 10.0, LinkStrategy::Retract);
        // pass1 (2) + retract MoveTo to safe_z + XY MoveTo at safe_z +
        // descend MoveTo to -5 + pass2 (2) = 7
        assert_eq!(result.len(), 7);
        // index 2: retract to safe_z
        assert!((result.endpoint(2).z - 10.0).abs() < 1e-12);
        // index 3: move XY at safe_z
        assert!((result.endpoint(3).x - 20.0).abs() < 1e-12);
        assert!((result.endpoint(3).z - 10.0).abs() < 1e-12);
        // index 4: descend to -5
        assert!((result.endpoint(4).z + 5.0).abs() < 1e-12);
    }

    #[test]
    fn test_link_passes_retract_same_xy_same_z() {
        let p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0);
        let p2 = make_pass((10.0, 0.0), (20.0, 0.0), 0.0);
        let result = link_passes(&[p1, p2], 0.0, LinkStrategy::Retract);
        // safe_z == cutting z and XY matches prev_exit → only one
        // travel MoveTo (or possibly none when XY already matches)
        // The retract + XY move are collapsed when they're the same XY
        assert!(!result.is_empty());
        // Should have at least the two passes worth of commands
        assert!(result.len() >= 4);
    }

    #[test]
    fn test_link_passes_three_passes() {
        let p1 = make_pass((0.0, 0.0), (10.0, 0.0), 0.0);
        let p2 = make_pass((10.0, 10.0), (20.0, 10.0), 0.0);
        let p3 = make_pass((20.0, 0.0), (30.0, 0.0), 0.0);
        let result = link_passes(&[p1, p2, p3], 5.0, LinkStrategy::Retract);
        // 3 passes * 2 cmds + 2 links * 3 cmds (retract, XY, descend) = 6 + 6 = 12
        assert_eq!(result.len(), 12);
    }
}
