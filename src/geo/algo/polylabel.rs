//! Polylabel: find the pole of inaccessibility (most distant interior point)
//! of a polygon using a grid-based priority-queue refinement algorithm.
//!
//! The algorithm starts by covering the polygon's bounding box with cells,
//! then repeatedly subdivides the most promising cell (highest upper bound
//! on distance-to-boundary) until the cell radius drops below `precision`.

use std::collections::BinaryHeap;

use crate::geo::shape::polygon::{
    get_point_line_distance, get_polygon_closest_point, is_point_in_polygon,
};
use crate::types::{Point, Polygon};

/// A square cell used during the priority-queue search.
#[derive(Clone, Copy, Debug)]
struct Cell {
    x: f64,
    y: f64,
    half_size: f64,
    /// Signed distance from the cell centre to the polygon boundary
    /// (positive = inside).
    dist: f64,
}

impl Cell {
    fn potential(&self) -> f64 {
        self.dist + self.half_size
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.potential() == other.potential()
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap, so higher potential → Greater → pops first.
        self.potential().total_cmp(&other.potential())
    }
}

/// Signed distance from `pt` to the boundary of the compound polygon
/// formed by `shell` (outer boundary) minus `holes` (inner boundaries).
/// Positive when the point is inside the shell and outside all holes.
fn signed_distance(pt: Point, shell: &Polygon, holes: &[Polygon]) -> f64 {
    // Must be inside the shell.
    if !is_point_in_polygon(pt, shell) {
        return -get_point_line_distance(pt, shell[0], shell[1]).max(1.0);
    }
    // Must be outside every hole.
    for hole in holes {
        if is_point_in_polygon(pt, hole) {
            // Inside a hole — return negative distance to the nearest hole edge.
            let n = hole.len();
            let mut min_d = f64::MAX;
            for i in 0..n {
                let j = (i + 1) % n;
                let d = get_point_line_distance(pt, hole[i], hole[j]);
                if d < min_d {
                    min_d = d;
                }
            }
            return -min_d;
        }
    }

    // Find the minimum absolute distance to any edge (shell or holes).
    let mut min_d = f64::MAX;

    let add_edges = |poly: &Polygon, md: &mut f64| {
        let n = poly.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let d = get_point_line_distance(pt, poly[i], poly[j]);
            if d < *md {
                *md = d;
            }
        }
    };

    add_edges(shell, &mut min_d);
    for hole in holes {
        add_edges(hole, &mut min_d);
    }

    min_d
}

/// Find the pole of inaccessibility — the point inside a polygon (with
/// optional holes) that is farthest from any boundary.
///
/// Uses the Polylabel algorithm (Mapbox):
///  1. Cover the shell's bounding box with a grid of cells.
///  2. Push all cells onto a max-heap keyed by `dist + half_size` (the
///     upper bound on distance inside the cell).
///  3. Pop the best cell.  If `half_size < precision` return its centre.
///  4. Otherwise split the cell into four quadrants and push them back.
///
/// `shell` is the outer boundary; `holes` are interior exclusions
/// (e.g. islands).  When there are no holes pass `&[]`.
pub fn get_polylabel(
    shell: &Polygon,
    holes: &[Polygon],
    precision: f64,
) -> Option<Point> {
    if shell.len() < 3 {
        return None;
    }

    // Bounding box from the shell.
    let (mut x_min, mut x_max) = (f64::MAX, f64::MIN);
    let (mut y_min, mut y_max) = (f64::MAX, f64::MIN);
    for p in shell {
        if p.x < x_min {
            x_min = p.x;
        }
        if p.x > x_max {
            x_max = p.x;
        }
        if p.y < y_min {
            y_min = p.y;
        }
        if p.y > y_max {
            y_max = p.y;
        }
    }
    let w = x_max - x_min;
    let h = y_max - y_min;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }

    let cell_size = w.max(h) / 16.0;
    let cell_radius = cell_size / 2.0_f64.sqrt();

    let mut heap = BinaryHeap::new();

    let mut y = y_min + cell_size * 0.5;
    while y < y_max {
        let mut x = x_min + cell_size * 0.5;
        while x < x_max {
            let dist = signed_distance(Point::new(x, y), shell, holes);
            if dist >= 0.0 {
                heap.push(Cell {
                    x,
                    y,
                    half_size: cell_radius,
                    dist,
                });
            }
            x += cell_size;
        }
        y += cell_size;
    }

    if heap.is_empty() {
        return None;
    }

    while let Some(cell) = heap.pop() {
        if cell.half_size < precision {
            return Some(Point::new(cell.x, cell.y));
        }

        let h2 = cell.half_size * 0.5;
        let off = cell.half_size * 0.5;

        for (dx, dy) in &[(-off, -off), (-off, off), (off, -off), (off, off)] {
            let cx = cell.x + dx;
            let cy = cell.y + dy;
            let dist = signed_distance(Point::new(cx, cy), shell, holes);
            let potential = dist + h2;
            if potential >= 0.0 {
                heap.push(Cell {
                    x: cx,
                    y: cy,
                    half_size: h2,
                    dist,
                });
            }
        }
    }

    None
}

/// Convenience wrapper: find the centre **and** radius of the largest
/// inscribed circle of a polygon (with optional holes).
///
/// The radius is the minimum distance from the pole to any boundary
/// edge (shell or holes).  Returns `None` when the polygon is
/// degenerate or no interior point exists above `precision`.
pub fn find_largest_circle(
    shell: &Polygon,
    holes: &[Polygon],
    precision: f64,
) -> Option<(Point, f64)> {
    let centre = get_polylabel(shell, holes, precision)?;

    let mut radius = f64::MAX;
    let consider = |poly: &Polygon, r: &mut f64| {
        if let Some((_, _, d2)) =
            get_polygon_closest_point(poly, centre.x, centre.y)
        {
            if d2 < *r {
                *r = d2;
            }
        }
    };
    consider(shell, &mut radius);
    for h in holes {
        consider(h, &mut radius);
    }

    Some((centre, radius.sqrt()))
}
