use std::collections::{HashMap, HashSet};

use crate::types::Rect;

/// A grid-based spatial index for fast overlap queries.
///
/// Divides the 2D plane into fixed-size cells and associates each
/// inserted item (identified by index) with the cells its bounding
/// box touches.  `query()` returns all item indices whose cells
/// overlap a given bounding box.
pub struct SpatialGrid {
    cell_size: f64,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f64) -> Self {
        SpatialGrid {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, index: usize, bbox: Rect) {
        let cell_min_x = (bbox.min.x / self.cell_size).floor() as i32;
        let cell_max_x = (bbox.max.x / self.cell_size).floor() as i32;
        let cell_min_y = (bbox.min.y / self.cell_size).floor() as i32;
        let cell_max_y = (bbox.max.y / self.cell_size).floor() as i32;
        for cx in cell_min_x..=cell_max_x {
            for cy in cell_min_y..=cell_max_y {
                self.cells.entry((cx, cy)).or_default().push(index);
            }
        }
    }

    /// Collect matching indices into a pre-allocated `Vec`.
    /// The caller is responsible for clearing the vec before passing it.
    pub fn query_into(&self, bbox: Rect, out: &mut Vec<usize>) {
        let cell_min_x = (bbox.min.x / self.cell_size).floor() as i32;
        let cell_max_x = (bbox.max.x / self.cell_size).floor() as i32;
        let cell_min_y = (bbox.min.y / self.cell_size).floor() as i32;
        let cell_max_y = (bbox.max.y / self.cell_size).floor() as i32;
        for cx in cell_min_x..=cell_max_x {
            for cy in cell_min_y..=cell_max_y {
                if let Some(indices) = self.cells.get(&(cx, cy)) {
                    out.extend(indices);
                }
            }
        }
    }

    pub fn query(&self, bbox: Rect) -> HashSet<usize> {
        let cell_min_x = (bbox.min.x / self.cell_size).floor() as i32;
        let cell_max_x = (bbox.max.x / self.cell_size).floor() as i32;
        let cell_min_y = (bbox.min.y / self.cell_size).floor() as i32;
        let cell_max_y = (bbox.max.y / self.cell_size).floor() as i32;
        let mut result = HashSet::new();
        for cx in cell_min_x..=cell_max_x {
            for cy in cell_min_y..=cell_max_y {
                if let Some(indices) = self.cells.get(&(cx, cy)) {
                    result.extend(indices);
                }
            }
        }
        result
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn cell_size(&self) -> f64 {
        self.cell_size
    }
}
