use crate::geo::algo::spatial_grid2d::SpatialGrid;
use crate::geo::shape::polygon::get_polygons_union;
use crate::types::{Point, Polygon, Rect};

const TOOL_CIRCLE_SEGMENTS: usize = 64;

fn tool_disk_polygon(center: Point, radius: f64, n: usize) -> Polygon {
    let mut poly = Vec::with_capacity(n);
    for i in 0..n {
        let a = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        poly.push(Point::new(
            center.x + radius * a.cos(),
            center.y + radius * a.sin(),
        ));
    }
    poly
}

fn tool_segment_swept(a: Point, b: Point, radius: f64) -> Vec<Polygon> {
    let dir = b - a;
    let len = dir.length();
    if len < 1e-12 {
        return vec![tool_disk_polygon(a, radius, TOOL_CIRCLE_SEGMENTS)];
    }
    let dir = dir / len;
    let perp = Point::new(-dir.y, dir.x);
    let rp = perp * radius;

    vec![
        vec![a - rp, b - rp, b + rp, a + rp],
        tool_disk_polygon(a, radius, TOOL_CIRCLE_SEGMENTS),
        tool_disk_polygon(b, radius, TOOL_CIRCLE_SEGMENTS),
    ]
}

pub struct ClearedArea {
    fragments: Vec<Polygon>,
    bboxes: Vec<Rect>,
    grid: SpatialGrid,
    cell_size: f64,
}

impl ClearedArea {
    pub fn new() -> Self {
        let cell_size = 10.0;
        ClearedArea {
            fragments: Vec::new(),
            bboxes: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
        }
    }

    pub fn from_polygons(initial: &[Polygon]) -> Self {
        let cell_size = 10.0;
        let mut ca = ClearedArea {
            fragments: Vec::new(),
            bboxes: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
        };
        for poly in initial {
            if poly.len() >= 3 {
                let idx = ca.fragments.len();
                ca.fragments.push(poly.clone());
                ca.bboxes.push(poly_bbox(poly));
                ca.grid.insert(idx, poly_bbox(poly));
            }
        }
        ca
    }

    pub fn expand(&mut self, tool_path: &[Point], tool_radius: f64) {
        if tool_path.len() < 2 || tool_radius < 1e-12 {
            return;
        }
        let mut swept_polys: Vec<Polygon> = Vec::new();
        for window in tool_path.windows(2) {
            swept_polys.extend(tool_segment_swept(
                window[0],
                window[1],
                tool_radius,
            ));
        }

        let mut all_polys: Vec<Polygon> = self.fragments.clone();
        all_polys.extend(swept_polys);
        let merged = get_polygons_union(&all_polys);

        self.fragments = merged;
        self.rebuild_grid();
    }

    pub fn expand_step(&mut self, prev: Point, next: Point, tool_radius: f64) {
        let swept = tool_segment_swept(prev, next, tool_radius);
        let mut all_polys = self.fragments.clone();
        all_polys.extend(swept);
        let merged = get_polygons_union(&all_polys);
        self.fragments = merged;
        self.rebuild_grid();
    }

    pub fn query_window(&self, bbox: Rect) -> Vec<&Polygon> {
        let indices = self.grid.query(bbox);
        let mut result: Vec<&Polygon> = indices
            .iter()
            .filter_map(|&idx| self.fragments.get(idx))
            .collect();
        result.sort_by(|a, b| {
            a.first()
                .and_then(|pa| {
                    b.first().map(|pb| {
                        pa.x.partial_cmp(&pb.x)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                })
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    pub fn remaining(&self, bounds: &[Polygon]) -> Vec<Polygon> {
        if self.fragments.is_empty() {
            return bounds.to_vec();
        }
        if bounds.is_empty() {
            return vec![];
        }
        use crate::geo::shape::polygon::get_polygons_group_difference;
        get_polygons_group_difference(bounds, &self.fragments)
    }

    pub fn total_area(&self) -> f64 {
        use crate::geo::shape::polygon::get_polygon_area;
        self.fragments.iter().map(get_polygon_area).sum()
    }

    pub fn fragments(&self) -> &[Polygon] {
        &self.fragments
    }

    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Directly insert known cleared polygons (e.g., the swept footprint
    /// of a bulk spiral). This avoids the overhead of sweeping thousands
    /// of individual line segments.
    pub fn add_cleared_polygons(&mut self, polygons: &[Polygon]) {
        if polygons.is_empty() {
            return;
        }
        let mut all_polys = self.fragments.clone();
        all_polys.extend(polygons.iter().cloned());
        self.fragments = get_polygons_union(&all_polys);
        self.rebuild_grid();
    }

    fn rebuild_grid(&mut self) {
        self.grid = SpatialGrid::new(self.cell_size);
        self.bboxes.clear();
        for (idx, poly) in self.fragments.iter().enumerate() {
            let bbox = poly_bbox(poly);
            self.bboxes.push(bbox);
            self.grid.insert(idx, bbox);
        }
    }
}

impl Default for ClearedArea {
    fn default() -> Self {
        Self::new()
    }
}

fn poly_bbox(poly: &Polygon) -> Rect {
    let mut x_min = f64::MAX;
    let mut y_min = f64::MAX;
    let mut x_max = f64::MIN;
    let mut y_max = f64::MIN;
    for p in poly {
        if p.x < x_min {
            x_min = p.x;
        }
        if p.y < y_min {
            y_min = p.y;
        }
        if p.x > x_max {
            x_max = p.x;
        }
        if p.y > y_max {
            y_max = p.y;
        }
    }
    Rect(x_min, y_min, x_max, y_max)
}
