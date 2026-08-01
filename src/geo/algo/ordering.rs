//! Path ordering algorithms.
//!
//! Pure-geometry combinatorial optimizations for sequencing paths
//! (polylines, arcs) with no machining or CNC concepts.

use prof_macros::prof;

use crate::geo::types::Point;

/// Order paths by greedy nearest-neighbor starting from the longest path.
///
/// The algorithm starts with the longest input path (most vertices),
/// then repeatedly selects the next unvisited path whose first endpoint
/// is closest to the current path's last endpoint.  Returns the indices
/// into `paths` in visit order.
///
/// This is a pure geometric optimization — no CNC or machining concepts.
#[prof]
pub fn order_nearest_neighbor(paths: &[Vec<Point>]) -> Vec<usize> {
    if paths.is_empty() {
        return Vec::new();
    }

    let mut used = vec![false; paths.len()];
    let mut order = Vec::with_capacity(paths.len());
    let start_idx = (0..paths.len())
        .max_by(|&i, &j| paths[i].len().cmp(&paths[j].len()))
        .unwrap_or(0);
    order.push(start_idx);
    used[start_idx] = true;

    while order.len() < paths.len() {
        let last_end = *paths[*order.last().unwrap()].last().unwrap();
        let mut best = None;
        let mut best_d2 = f64::MAX;
        for (i, path) in paths.iter().enumerate() {
            if used[i] || path.len() < 2 {
                continue;
            }
            let d2 = (path[0] - last_end).length_squared();
            if d2 < best_d2 {
                best_d2 = d2;
                best = Some(i);
            }
        }
        if let Some(i) = best {
            order.push(i);
            used[i] = true;
        } else {
            break;
        }
    }

    order
}
