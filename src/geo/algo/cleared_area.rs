use prof_macros::prof;

use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::simplify::simplify_polyline_3d;
use crate::geo::algo::spatial_grid2d::SpatialGrid;
use crate::geo::shape::line::get_line_segment_closest_point;
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
    /// Flat list of all edges across all fragments, indexed by `edge_grid`.
    edge_pts: Vec<(Point, Point)>,
    /// Spatial grid over individual fragment edges for fast nearest-edge
    /// queries from arbitrary query points.
    edge_grid: SpatialGrid,
    /// Cell size for the edge-level spatial grid (finer than the polygon
    /// grid so that thin crescents are captured).
    edge_cell_size: f64,
}

impl ClearedArea {
    fn insert_poly_edges(
        edge_pts: &mut Vec<(Point, Point)>,
        edge_grid: &mut SpatialGrid,
        poly: &[Point],
    ) {
        let m = poly.len();
        for i in 0..m {
            let j = (i + 1) % m;
            let a = poly[i];
            let b = poly[j];
            let idx = edge_pts.len();
            edge_pts.push((a, b));
            edge_grid.insert(
                idx,
                Rect::new(
                    a.x.min(b.x),
                    a.y.min(b.y),
                    a.x.max(b.x),
                    a.y.max(b.y),
                ),
            );
        }
    }

    pub fn new() -> Self {
        let cell_size = 10.0;
        let edge_cell_size = 5.0;
        ClearedArea {
            fragments: Vec::new(),
            bboxes: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            edge_pts: Vec::new(),
            edge_grid: SpatialGrid::new(edge_cell_size),
            edge_cell_size,
        }
    }

    pub fn from_polygons(initial: &[Polygon]) -> Self {
        let cell_size = 10.0;
        let edge_cell_size = 5.0;
        let mut ca = ClearedArea {
            fragments: Vec::new(),
            bboxes: Vec::new(),
            grid: SpatialGrid::new(cell_size),
            cell_size,
            edge_pts: Vec::new(),
            edge_grid: SpatialGrid::new(edge_cell_size),
            edge_cell_size,
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

    /// Replace all stored fragments with a new set (e.g., the new frontier
    /// after a wavefront advance).  This is O(m) in the new fragment count
    /// and avoids the O(n·m) accumulation of
    /// [`add_cleared_polygons`](Self::add_cleared_polygons).
    pub fn replace_fragments(&mut self, fragments: Vec<Polygon>) {
        self.fragments = fragments;
        self.rebuild_grid();
    }

    /// Return the squared distance from `(x, y)` to the nearest edge of
    /// any cleared fragment.
    ///
    /// Uses the edge-level spatial grid so that only edges in nearby
    /// grid cells are checked.  Returns `f64::MAX` when there are no
    /// fragments.
    pub fn closest_boundary_distance_sq(&self, x: f64, y: f64) -> f64 {
        let cell_size = self.edge_cell_size;
        let mut qbox = Rect::new(x, y, x, y);
        qbox.min.x -= cell_size;
        qbox.min.y -= cell_size;
        qbox.max.x += cell_size;
        qbox.max.y += cell_size;
        let mut candidates = Vec::new();
        self.edge_grid.query_into(qbox, &mut candidates);
        if candidates.is_empty() {
            return f64::MAX;
        }
        let mut best_d2 = f64::MAX;
        for &ei in &candidates {
            let (a, b) = self.edge_pts[ei];
            let (_, _, d2) = get_line_segment_closest_point(a, b, x, y);
            if d2 < best_d2 {
                best_d2 = d2;
            }
        }
        best_d2
    }

    pub fn expand(&mut self, path: &[Point], radius: f64) {
        if path.len() < 2 || radius < 1e-12 {
            return;
        }
        let mut swept_polys: Vec<Polygon> = Vec::new();
        for window in path.windows(2) {
            swept_polys.extend(get_segment_swept_polygon(
                window[0], window[1], radius,
            ));
        }

        let mut all_polys: Vec<Polygon> = self.fragments.clone();
        all_polys.extend(swept_polys);
        let merged = get_polygons_union(&all_polys);

        self.fragments = merged;
        self.rebuild_grid();
    }

    pub fn expand_step(&mut self, prev: Point, next: Point, radius: f64) {
        let swept = get_segment_swept_polygon(prev, next, radius);
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

    /// Compute the inset region of `boundary` by `radius` (excluding
    /// `obstacles`), then return the portions of that region not covered
    /// by stored fragments, together with the original obstacle polygons.
    pub fn remaining_in_inset(
        &self,
        boundary: &Polygon,
        obstacles: &[Polygon],
        radius: f64,
    ) -> Vec<Polygon> {
        let (inset_region, _) =
            compute_inset_region(boundary, radius, obstacles);
        let mut result = obstacles.to_vec();
        result.extend(get_polygons_group_difference(
            &inset_region,
            self.fragments(),
        ));
        result
    }

    #[prof]
    pub fn total_area(&self) -> f64 {
        self.fragments.iter().map(get_polygon_area).sum()
    }

    #[prof]
    pub fn fragments(&self) -> &[Polygon] {
        &self.fragments
    }

    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Directly insert known cleared polygons. This avoids the overhead
    /// of sweeping thousands of individual line segments.
    #[prof]
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
    #[prof]
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
                    Self::insert_poly_edges(
                        &mut self.edge_pts,
                        &mut self.edge_grid,
                        poly,
                    );
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
                let simplified = simplify_polyline_3d(&pts3d, simplify_tol);
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

    /// Expand the current frontier outward by `step_over`, clip to
    /// `valid_area`, and return the newly-exposed region (the
    /// difference between the expanded frontier and the stored
    /// fragments).  Does NOT modify the stored fragments — the caller
    /// is responsible for calling
    /// [`absorb_frontier`](Self::absorb_frontier) afterwards.
    #[prof]
    pub fn compute_bites(
        &self,
        step_over: f64,
        valid_area: &[Polygon],
        simplify_tol: f64,
    ) -> Vec<Polygon> {
        if self.fragments.is_empty() {
            return vec![];
        }
        let offset_src: Vec<Polygon> = get_polygons_union(&self.fragments)
            .into_iter()
            .filter_map(|p| {
                if p.len() < 3 {
                    return None;
                }
                let pts3d: Vec<Point3D> =
                    p.iter().map(|q| Point3D::new(q.x, q.y, 0.0)).collect();
                let simplified = simplify_polyline_3d(&pts3d, simplify_tol);
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
            .collect();
        if offset_src.is_empty() {
            return vec![];
        }
        let expanded: Vec<Polygon> = offset_src
            .iter()
            .flat_map(|p| offset_polygon(p, step_over, JoinStyle::Round))
            .collect();
        if expanded.is_empty() {
            return vec![];
        }
        let mut region =
            get_polygons_group_difference(&expanded, &self.fragments);
        region = get_polygons_group_intersection(&region, valid_area);
        region
    }

    /// Absorb a set of polygons (e.g. the output of
    /// [`compute_bites`](Self::compute_bites)) into the stored
    /// fragments by unioning them, without accumulating historical
    /// fragments.
    #[prof]
    pub fn absorb_frontier(&mut self, region: &[Polygon]) {
        if region.is_empty() {
            return;
        }
        let mut both = self.fragments.clone();
        both.extend(region.iter().cloned());
        self.replace_fragments(get_polygons_union(&both));
    }

    /// Expand the current frontier by `step_over`, clip to `valid_area`,
    /// subtract already-cleared space, and return the resulting "bites".
    #[prof]
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
                len <= step_over || dir.dot(to_bite / len) >= cos_max
            })
            .collect()
    }

    /// Return all passes of isotropic bites needed to fully clear the
    /// valid area.
    ///
    /// Each inner `Vec` is one pass (all bites generated from the same
    /// frontier).  Passes are ordered from the centre of the cleared area
    /// outward.  The cleared area is fully cleared after this call.
    ///
    /// Note: this method uses the older accumulation-based approach
    /// ([`bites`] + [`incorporate`]).  For better performance on large
    /// pockets, use [`compute_bites`](Self::compute_bites) /
    /// [`absorb_frontier`](Self::absorb_frontier) in a
    /// caller-managed loop.
    pub fn all_bites(
        &mut self,
        step_over: f64,
        valid_area: &[Polygon],
        simplify_tol: f64,
    ) -> Vec<Vec<Polygon>> {
        let mut passes = Vec::new();
        loop {
            let bites = self.bites(step_over, valid_area, simplify_tol);
            if bites.is_empty() {
                break;
            }
            passes.push(bites);
            self.incorporate(passes.last().unwrap());
        }
        passes
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

        self.edge_grid = SpatialGrid::new(self.edge_cell_size);
        self.edge_pts.clear();
        for poly in &self.fragments {
            if poly.len() >= 2 {
                Self::insert_poly_edges(
                    &mut self.edge_pts,
                    &mut self.edge_grid,
                    poly,
                );
            }
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
