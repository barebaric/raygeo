use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::shape::polygon::get_polygons_closest_point;
use crate::ops::area::ClearedArea;
use crate::types::Point;

/// A resume point found on the cleared-area frontier.
#[derive(Debug, Clone)]
pub struct ResumePoint {
    /// Position on the frontier.
    pub pos: Point,
    /// Outward-normal heading at the resume position (radians).
    pub heading: f64,
    /// Travel polyline through cleared territory, routed by the MAT.
    pub link_path: Vec<Point>,
}

/// Internal: position + heading pair found on the frontier.
struct FrontierCandidate {
    pos: Point,
    heading: f64,
}

impl ClearedArea {
    /// Walk the cleared-area frontier forward from a point near `end_pos`
    /// and return the first position where engagement ≥ `min_engagement`.
    ///
    /// The link path between `end_pos` and the resume position is routed
    /// through the cleared area via MAT for collision‑free travel.
    ///
    /// Returns `None` when no valid resume point is found (fully cleared
    /// or no MAT available).
    pub fn find_next_resume(
        &self,
        mat: &MedialAxis,
        end_pos: Point,
        radius: f64,
        min_engagement: f64,
    ) -> Option<ResumePoint> {
        if self.is_empty() {
            return None;
        }

        let frontier = self.frontier(0.5);
        if frontier.is_empty() {
            return None;
        }

        // Find the closest point on the frontier to end_pos
        // using the general polygon query.
        let (closest_poly_idx, _t, closest_pt, _d2) =
            get_polygons_closest_point(&frontier, end_pos)?;

        // Build a FrontierCandidate from the closest point.
        let poly = &frontier[closest_poly_idx];
        let heading = frontier_heading_at(closest_pt, poly);
        let start_candidate = FrontierCandidate {
            pos: closest_pt,
            heading,
        };

        // Walk the frontier forward, checking engagement.
        let walk_candidates = walk_frontier_forward(
            &start_candidate,
            &frontier,
            radius,
            min_engagement,
            self,
        );

        let resume_pt = walk_candidates.into_iter().next()?;

        let link = mat
            .path_between(end_pos, resume_pt.pos)
            .unwrap_or_else(|| vec![end_pos, resume_pt.pos]);

        Some(ResumePoint {
            pos: resume_pt.pos,
            heading: resume_pt.heading,
            link_path: link,
        })
    }
}

/// Walk the frontier polygon forward from `start`, checking engagement at
/// each vertex.  Returns candidates with engagement ≥ min_engagement.
fn walk_frontier_forward(
    start: &FrontierCandidate,
    frontier: &[Vec<Point>],
    radius: f64,
    min_engagement: f64,
    cleared: &ClearedArea,
) -> Vec<FrontierCandidate> {
    let mut candidates = Vec::new();

    for poly in frontier {
        if poly.len() < 3 {
            continue;
        }

        // Find the nearest vertex index in this polygon.
        let start_idx = poly.iter().enumerate().min_by(|(_, a), (_, b)| {
            a.distance_squared(start.pos)
                .partial_cmp(&b.distance_squared(start.pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let start_idx = match start_idx {
            Some((i, _)) => i,
            None => continue,
        };

        // Walk forward from start_idx (wrap around).
        let n = poly.len();
        for offset in 0..n {
            let idx = (start_idx + offset) % n;
            let pt = poly[idx];
            let eng = cleared.point_engagement(pt, radius);
            if eng.angle >= min_engagement {
                let heading = frontier_heading_at(pt, poly);
                candidates.push(FrontierCandidate { pos: pt, heading });
                break;
            }
        }
    }

    candidates
}

/// Estimate the outward-normal heading at a vertex of a frontier polygon.
fn frontier_heading_at(v: Point, poly: &[Point]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let n = poly.len();
    // Find the edge whose perpendicular projection best matches v.
    let mut best_edge = (0usize, 1usize);
    let mut best_d2 = f64::MAX;
    for i in 0..n {
        let j = (i + 1) % n;
        let a = poly[i];
        let b = poly[j];
        let (_, _, d2) = get_line_segment_closest_point(a, b, v.x, v.y);
        if d2 < best_d2 {
            best_d2 = d2;
            best_edge = (i, j);
        }
    }
    let (ei, ej) = best_edge;
    let edge_dir = poly[ej] - poly[ei];
    if edge_dir.length_squared() < 1e-12 {
        return 0.0;
    }
    // Right normal: for a CCW outer polygon this points outward.
    let outward = Point::new(edge_dir.y, -edge_dir.x);
    outward.y.atan2(outward.x)
}
