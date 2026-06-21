use crate::geo::algo::simplify::simplify_polyline;
use crate::geo::algo::spatial_grid2d::SpatialGrid;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::geo::shape::polygon::get_polygons_group_difference;
use crate::geo::shape::polygon::get_polygons_group_intersection;
use crate::geo::shape::polygon::get_polygons_union;
use crate::geo::shape::polygon::get_segment_swept_polygon;
use crate::geo::shape::polygon::offset_polygon;
use crate::geo::shape::polygon::JoinStyle;
use crate::types::{Point, Point3D, Polygon, Rect};

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
            swept_polys.extend(get_segment_swept_polygon(
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
        let swept = get_segment_swept_polygon(prev, next, tool_radius);
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
        get_polygons_group_difference(bounds, &self.fragments)
    }

    pub fn total_area(&self) -> f64 {
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

    /// Add polygons and return only the newly-added portion (input minus
    /// already-cleared area).  When none of the inputs overlap any existing
    /// fragment the union is skipped and they are appended directly for
    /// better performance.
    pub fn incorporate(&mut self, polys: &[Polygon]) -> Vec<Polygon> {
        if polys.is_empty() {
            return vec![];
        }
        let new = self.remaining(polys);
        if new.is_empty() {
            return new;
        }
        if self.any_overlap(&new) {
            let mut all = self.fragments.clone();
            all.extend(new.iter().cloned());
            self.fragments = get_polygons_union(&all);
            self.rebuild_grid();
        } else {
            for poly in &new {
                if poly.len() >= 3 {
                    let idx = self.fragments.len();
                    let bb = poly_bbox(poly);
                    self.fragments.push(poly.clone());
                    self.bboxes.push(bb);
                    self.grid.insert(idx, bb);
                }
            }
        }
        new
    }

    /// Return a unioned, simplified snapshot of the current outer boundary.
    pub fn frontier(&self, simplify_tol: f64) -> Vec<Polygon> {
        let unioned = get_polygons_union(&self.fragments);
        unioned
            .into_iter()
            .filter_map(|p| {
                let pts3d: Vec<Point3D> =
                    p.iter().map(|q| Point3D::new(q.x, q.y, 0.0)).collect();
                let simplified = simplify_polyline(&pts3d, simplify_tol);
                if simplified.len() < 3 {
                    None
                } else {
                    Some(
                        simplified
                            .iter()
                            .map(|q| Point::new(q.x, q.y))
                            .collect(),
                    )
                }
            })
            .collect()
    }

    /// Expand the current frontier by `step_over`, clip to `valid_area`,
    /// subtract already-cleared space, and return the resulting "bites" of
    /// material to be machined.
    pub fn bites(
        &self,
        step_over: f64,
        valid_area: &[Polygon],
        simplify_tol: f64,
    ) -> Vec<Polygon> {
        let f = self.frontier(simplify_tol);
        if f.is_empty() {
            return vec![];
        }
        let expanded: Vec<Polygon> = f
            .iter()
            .flat_map(|p| offset_polygon(p, step_over, JoinStyle::Round))
            .collect();
        if expanded.is_empty() {
            return vec![];
        }
        let material = self.remaining(&expanded);
        get_polygons_group_intersection(&material, valid_area)
    }

    /// Like [`bites`](Self::bites) but only returns the bites whose
    /// centroid lies within `max_angle` radians of the direction from
    /// this area's centre toward `target`.
    ///
    /// When the cleared area is empty the direction-filter is skipped
    /// and all bites are returned.
    pub fn bite_in_direction(
        &self,
        step_over: f64,
        valid_area: &[Polygon],
        simplify_tol: f64,
        target: Point,
        max_angle: f64,
    ) -> Vec<Polygon> {
        let all = self.bites(step_over, valid_area, simplify_tol);
        if all.is_empty()
            || self.fragments.is_empty()
            || max_angle >= std::f64::consts::PI
        {
            return all;
        }

        // Compute centre of current cleared area.
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut count = 0usize;
        for frag in &self.fragments {
            for p in frag {
                cx += p.x;
                cy += p.y;
                count += 1;
            }
        }
        let centre = Point::new(cx / count as f64, cy / count as f64);
        let dir = target - centre;
        let dir_len = dir.length();
        if dir_len < 1e-12 {
            return all;
        }
        let dir = dir / dir_len;

        let cos_max = max_angle.cos();

        all.into_iter()
            .filter(|bite| {
                let bc = get_polygon_centroid(bite);
                let to_bite = bc - centre;
                let len = to_bite.length();
                len > 1e-12 && dir.dot(to_bite / len) >= cos_max
            })
            .collect()
    }

    /// True when any polygon in `polys` overlaps an existing fragment.
    fn any_overlap(&self, polys: &[Polygon]) -> bool {
        for poly in polys {
            if poly.len() < 3 {
                continue;
            }
            let bb = poly_bbox(poly);
            if !self.grid.query(bb).is_empty() {
                return true;
            }
        }
        false
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
    Rect::new(x_min, y_min, x_max, y_max)
}
