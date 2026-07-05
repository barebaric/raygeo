//! A* path finding on a rasterised grid.
//!
//! The walkable area is rasterised at a given resolution.
//! Obstacles are dilated by a safety margin. A* then finds a
//! shortest path between two points that avoids all obstacles.

use std::collections::BinaryHeap;

use prof_macros::prof;

use crate::geo::shape::polygon::{
    get_polygon_group_bounds, is_point_inside_polygon, offset_polygon,
    JoinStyle,
};
use crate::types::{Point, Polygon};

/// Wrapper for `f64` that implements `Ord` via `total_cmp` (never NaN).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cost(f64);
impl Eq for Cost {}
impl PartialOrd for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Raster cell size (default 1.0).
pub const DEFAULT_CELL_SIZE: f64 = 1.0;

/// Path finding result.
#[derive(Debug, Clone)]
pub struct AStarPath {
    /// Waypoints in world coordinates (grid centres).
    pub waypoints: Vec<Point>,
    /// Cells visited during the search (for diagnostics).
    pub visited: usize,
    /// Length of the path in world units.
    pub length: f64,
}

/// Find a path from `from` to `to` inside the walkable area,
/// avoiding obstacles dilated by `obstacle_margin`.
///
/// * `free_space` — the area where pathfinding is allowed (polygons).
/// * `obstacles` — forbidden zones to avoid.
/// * `obstacle_margin` — radius by which obstacles are expanded.
/// * `cell_size` — raster grid resolution (smaller = finer but slower).
#[prof]
#[allow(clippy::too_many_arguments)]
pub fn find_path(
    from: Point,
    to: Point,
    free_space: &[Polygon],
    obstacles: &[Polygon],
    obstacle_margin: f64,
    cell_size: f64,
) -> Option<AStarPath> {
    if cell_size <= 0.0 || free_space.is_empty() {
        return None;
    }

    let margin = obstacle_margin + cell_size * 0.5;

    // Compute bounding box of free space.
    let bounds = get_polygon_group_bounds(free_space);
    let origin = Point::new(bounds.min.x - cell_size, bounds.min.y - cell_size);
    let w = bounds.max.x - bounds.min.x + 2.0 * cell_size;
    let h = bounds.max.y - bounds.min.y + 2.0 * cell_size;
    let cols = (w / cell_size).ceil() as usize;
    let rows = (h / cell_size).ceil() as usize;

    if cols < 2 || rows < 2 {
        return None;
    }

    // Dilate obstacles by the safety margin.
    let dilated: Vec<Polygon> = obstacles
        .iter()
        .filter_map(|poly| {
            let result = offset_polygon(poly, margin, JoinStyle::Round);
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        })
        .flatten()
        .collect();

    // Rasterise: mark cells that are inside free_space but outside
    // dilated obstacles.
    let cell_center = |col: usize, row: usize| -> Point {
        Point::new(
            origin.x + (col as f64 + 0.5) * cell_size,
            origin.y + (row as f64 + 0.5) * cell_size,
        )
    };

    let mut blocked = vec![false; cols * rows];
    for row in 0..rows {
        for col in 0..cols {
            let p = cell_center(col, row);
            let inside_free = free_space
                .iter()
                .any(|poly| is_point_inside_polygon(p, poly));
            if !inside_free {
                blocked[row * cols + col] = true;
                continue;
            }
            let inside_obstacle =
                dilated.iter().any(|poly| is_point_inside_polygon(p, poly));
            if inside_obstacle {
                blocked[row * cols + col] = true;
            }
        }
    }

    // Snap start/goal to nearest passable cell.
    let snap = |world: Point| -> Option<(usize, usize)> {
        let c = ((world.x - origin.x) / cell_size) as isize;
        let r = ((world.y - origin.y) / cell_size) as isize;
        // Search outward in a small spiral for a passable cell.
        for d in 0isize..10 {
            for dr in -d..=d {
                for dc in -d..=d {
                    if dr.abs() != d && dc.abs() != d {
                        continue;
                    }
                    let nc = c + dc;
                    let nr = r + dr;
                    if nc >= 0
                        && nc < cols as isize
                        && nr >= 0
                        && nr < rows as isize
                        && !blocked[nr as usize * cols + nc as usize]
                    {
                        return Some((nc as usize, nr as usize));
                    }
                }
            }
        }
        None
    };

    let (sc, sr) = snap(from)?;
    let (ec, er) = snap(to)?;

    if sc == ec && sr == er {
        let p = cell_center(sc, sr);
        return Some(AStarPath {
            waypoints: vec![p],
            visited: 1,
            length: 0.0,
        });
    }

    // A*.
    let index = |col: usize, row: usize| -> usize { row * cols + col };
    let heuristic = |col: usize, row: usize| -> f64 {
        let p = cell_center(col, row);
        p.distance(to)
    };

    let start_idx = index(sc, sr);
    let goal_idx = index(ec, er);

    let mut g_score = vec![f64::MAX; cols * rows];
    let mut f_score = vec![f64::MAX; cols * rows];
    let mut came_from = vec![None::<usize>; cols * rows];
    let mut closed = vec![false; cols * rows];

    g_score[start_idx] = 0.0;
    f_score[start_idx] = heuristic(sc, sr);

    let mut open = BinaryHeap::new();
    open.push(std::cmp::Reverse((Cost(f_score[start_idx]), start_idx)));

    let mut visited_count = 0;

    const NEIGHBOURS: &[(isize, isize)] = &[
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    while let Some(std::cmp::Reverse((_cost, current))) = open.pop() {
        if closed[current] {
            continue;
        }
        closed[current] = true;
        visited_count += 1;

        if current == goal_idx {
            // Trace back.
            let mut path = Vec::new();
            let mut idx = current;
            loop {
                let col = idx % cols;
                let row = idx / cols;
                path.push(cell_center(col, row));
                if let Some(prev) = came_from[idx] {
                    idx = prev;
                } else {
                    break;
                }
            }
            path.reverse();
            let length: f64 =
                path.windows(2).map(|w| w[0].distance(w[1])).sum();
            return Some(AStarPath {
                waypoints: path,
                visited: visited_count,
                length,
            });
        }

        let cur_col = (current % cols) as isize;
        let cur_row = (current / cols) as isize;
        let cur_g = g_score[current];

        for &(dc, dr) in NEIGHBOURS {
            let nc = cur_col + dc;
            let nr = cur_row + dr;
            if nc < 0 || nc >= cols as isize || nr < 0 || nr >= rows as isize {
                continue;
            }
            let n_idx = index(nc as usize, nr as usize);
            if blocked[n_idx] || closed[n_idx] {
                continue;
            }

            let step = if dc != 0 && dr != 0 {
                std::f64::consts::SQRT_2
            } else {
                1.0
            };
            let tentative = cur_g + step * cell_size;
            if tentative < g_score[n_idx] {
                came_from[n_idx] = Some(current);
                g_score[n_idx] = tentative;
                let h = heuristic(nc as usize, nr as usize);
                f_score[n_idx] = tentative + h;
                open.push(std::cmp::Reverse((Cost(f_score[n_idx]), n_idx)));
            }
        }
    }

    None
}
