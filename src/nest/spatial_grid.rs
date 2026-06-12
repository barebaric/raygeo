use std::collections::{HashMap, HashSet};

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

    pub fn insert(&mut self, index: usize, bbox: (f64, f64, f64, f64)) {
        let (min_x, min_y, max_x, max_y) = bbox;
        let cell_min_x = (min_x / self.cell_size).floor() as i32;
        let cell_max_x = (max_x / self.cell_size).floor() as i32;
        let cell_min_y = (min_y / self.cell_size).floor() as i32;
        let cell_max_y = (max_y / self.cell_size).floor() as i32;
        for cx in cell_min_x..=cell_max_x {
            for cy in cell_min_y..=cell_max_y {
                self.cells.entry((cx, cy)).or_default().push(index);
            }
        }
    }

    pub fn query(&self, bbox: (f64, f64, f64, f64)) -> HashSet<usize> {
        let (min_x, min_y, max_x, max_y) = bbox;
        let cell_min_x = (min_x / self.cell_size).floor() as i32;
        let cell_max_x = (max_x / self.cell_size).floor() as i32;
        let cell_min_y = (min_y / self.cell_size).floor() as i32;
        let cell_max_y = (max_y / self.cell_size).floor() as i32;
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
