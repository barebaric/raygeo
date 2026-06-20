use clipper2::FillRule;

use crate::geo::shape::polygon::{
    get_polygon_bounds, get_polygon_convex_hull, polygons_to_paths,
};
use crate::types::{Point, Polygon};

/// Compute the Inner-Fit Polygon (IFP) for placing a part inside a bin.
///
/// The IFP is the set of all valid positions where `part` can be placed
/// entirely inside `bin`. It is computed as `bin - no_go_zones` where
/// no-go zones represent positions where the part would intersect the
/// bin boundary.
///
/// Input polygons should be normalized (at origin). Output IFPs are also
/// in normalized space — the caller translates to world coordinates.
pub fn inner_fit_polygon(bin: &Polygon, part: &Polygon) -> Vec<Polygon> {
    if bin.len() < 3 || part.len() < 3 {
        return vec![];
    }

    // Quick bounding-box reject
    let bin_bounds = get_polygon_bounds(bin);
    let part_bounds = get_polygon_bounds(part);
    let bin_width = bin_bounds.max.x - bin_bounds.min.x;
    let bin_height = bin_bounds.max.y - bin_bounds.min.y;
    let part_width = part_bounds.max.x - part_bounds.min.x;
    let part_height = part_bounds.max.y - part_bounds.min.y;
    if part_width > bin_width + 1e-4 || part_height > bin_height + 1e-4 {
        return vec![];
    }

    let part_neg: Polygon =
        part.iter().map(|p| Point::new(-p.x, -p.y)).collect();
    let no_go_zones = build_no_go_zones(bin, &part_neg);

    if no_go_zones.is_empty() {
        return vec![bin.to_vec()];
    }

    // Use clipper2 builder: bin - (no_go_zone_1 ∪ no_go_zone_2 ∪ ...)
    // This matches the Python approach of a single CT_DIFFERENCE with
    // multiple clip paths, which correctly handles outer/hole contours.
    let bin_paths = polygons_to_paths(std::slice::from_ref(bin));
    let no_go_paths = polygons_to_paths(&no_go_zones);

    let result: Vec<Polygon> = match bin_paths
        .to_clipper_subject()
        .add_clip(no_go_paths)
        .difference(FillRule::NonZero)
    {
        Ok(paths) => {
            let tuples: Vec<Vec<(f64, f64)>> = Vec::from(paths);
            tuples
                .iter()
                .map(|path| {
                    path.iter().map(|(x, y)| Point::new(*x, *y)).collect()
                })
                .collect()
        }
        Err(_) => return vec![],
    };

    result.into_iter().filter(|p| p.len() >= 3).collect()
}

/// Build the no-go zones for a bin-part pair.
///
/// `part_neg` is the orbiting polygon negated (each (x,y) -> (-x,-y)).
/// Returns corner caps at every bin vertex and convex-hull sweeps
/// along every bin edge.
pub fn build_no_go_zones(bin: &Polygon, part_neg: &Polygon) -> Vec<Polygon> {
    let mut subjects: Vec<Polygon> = Vec::new();

    // Corner caps: part_neg placed at each bin vertex
    for &v in bin {
        let translated: Polygon = part_neg
            .iter()
            .map(|p| Point::new(p.x + v.x, p.y + v.y))
            .collect();
        if translated.len() >= 3 {
            subjects.push(translated);
        }
    }

    // Edge sweeps: convex hull of part_neg at both edge endpoints
    let n = bin.len();
    for i in 0..n {
        let p1 = bin[(i + n - 1) % n];
        let p2 = bin[i];
        let hull = sweep_hull_for_edge(p1, p2, part_neg);
        if hull.len() >= 3 {
            subjects.push(hull);
        }
    }

    subjects
}

/// Compute the convex hull sweep of `part_neg` along the edge p1→p2.
///
/// The result is the convex hull of `part_neg` translated to p1 and to p2.
pub fn sweep_hull_for_edge(
    p1: Point,
    p2: Point,
    part_neg: &Polygon,
) -> Polygon {
    let mut points: Vec<Point> = Vec::with_capacity(part_neg.len() * 2);
    for p in part_neg {
        points.push(Point::new(p.x + p1.x, p.y + p1.y));
        points.push(Point::new(p.x + p2.x, p.y + p2.y));
    }
    get_polygon_convex_hull(&points)
}
