use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::cut::ClearedArea;
use crate::types::Point;
use crate::types::Rect;

use super::stock_region::PyStockRegion;

#[gen_stub_pyclass(module = "raygeo.ops.cut.cleared_area")]
#[pyclass(name = "ClearedArea", from_py_object)]
pub struct PyClearedArea {
    pub(crate) inner: ClearedArea,
}

impl Clone for PyClearedArea {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClearedArea {
    /// Create an empty ClearedArea.
    ///
    /// :param initial: Optional pre-seeded cleared polygons.
    #[new]
    #[pyo3(signature = (initial=None))]
    pub fn new(initial: Option<Vec<Vec<(f64, f64)>>>) -> Self {
        match initial {
            Some(polys) => {
                let polygons: Vec<crate::types::Polygon> = polys
                    .into_iter()
                    .map(|v| {
                        v.into_iter().map(|(x, y)| Point::new(x, y)).collect()
                    })
                    .collect();
                PyClearedArea {
                    inner: ClearedArea::with_fragments(&polygons),
                }
            }
            None => PyClearedArea {
                inner: ClearedArea::new(),
            },
        }
    }

    /// Sweep a disk along a polyline, adding the swept area to the
    /// cleared set.
    ///
    /// :param path: List of ``(x, y)`` points forming the polyline.
    /// :param radius: Disk radius (mm).
    /// :complexity: O(n) where n = number of path points
    pub fn expand(&mut self, path: Vec<(f64, f64)>, radius: f64) {
        let path: Vec<Point> =
            path.into_iter().map(|(x, y)| Point::new(x, y)).collect();
        self.inner.expand(&path, radius);
    }

    /// Expand the cleared area by sweeping a disk of *radius* along a
    /// single segment from *prev* to *next*.
    ///
    /// :param prev: Start point ``(x, y)`` of the segment.
    /// :param next: End point ``(x, y)`` of the segment.
    /// :param radius: Disk radius (mm).
    pub fn expand_step(
        &mut self,
        prev: (f64, f64),
        next: (f64, f64),
        radius: f64,
    ) {
        self.inner.expand_step(
            Point::new(prev.0, prev.1),
            Point::new(next.0, next.1),
            radius,
        );
    }

    /// Signed perpendicular distance to the nearest cleared boundary.
    ///
    /// Returns positive when the point is outside the cleared area
    /// (in uncut material), negative when inside.
    ///
    /// :param x: X coordinate of the query point.
    /// :param y: Y coordinate of the query point.
    /// :returns: Signed distance in mm.  ``0.0`` means exactly on
    ///           the boundary.
    pub fn signed_boundary_distance(&self, x: f64, y: f64) -> f64 {
        self.inner.signed_boundary_distance(x, y)
    }

    /// Add pre-computed polygons to the cleared set.
    ///
    /// :param polygons: List of polygons (each a list of ``(x, y)``
    ///                  vertices) to add.
    /// :complexity: O(n) where n = total vertices across all polygons
    pub fn cut(&mut self, polygons: Vec<Vec<(f64, f64)>>) {
        let polys: Vec<crate::types::Polygon> = polygons
            .into_iter()
            .map(|v| v.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        self.inner.cut(&polys);
    }

    /// Return fragments whose bounding box overlaps the query window.
    ///
    /// :param bbox: Bounding box ``(x_min, y_min, x_max, y_max)``.
    /// :returns: Fragments intersecting the bounding box.
    /// :complexity: O(m + k) where m = number of fragments,
    ///              k = output vertices
    pub fn query_window(
        &self,
        bbox: (f64, f64, f64, f64),
    ) -> Vec<Vec<(f64, f64)>> {
        let rect = Rect::new(bbox.0, bbox.1, bbox.2, bbox.3);
        let frags = self.inner.query_window(rect);
        frags
            .into_iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Subtract cleared fragments from the stock, returning the uncut
    /// portion.
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :returns: List of polygons representing the uncut portion.
    /// :complexity: O(n * m) where n = stock vertices, m = fragments
    pub fn remaining(&self, region: &PyStockRegion) -> Vec<Vec<(f64, f64)>> {
        self.inner
            .remaining(&region.inner)
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Add polygons, returning only the newly-added portion.
    /// Faster than ``cut`` when inputs don't overlap existing fragments
    /// (skips the full union).
    ///
    /// :param polygons: List of polygons to add.
    /// :returns: List of polygons representing the newly-added portion.
    /// :complexity: O(n log n) worst case when union required,
    ///              O(n) when inputs are disjoint from existing fragments
    pub fn cut_fast(
        &mut self,
        polygons: Vec<Vec<(f64, f64)>>,
    ) -> Vec<Vec<(f64, f64)>> {
        let polys: Vec<crate::types::Polygon> = polygons
            .into_iter()
            .map(|v| v.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        let new = self.inner.cut_fast(&polys);
        new.into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Return a unioned, simplified snapshot of the current outer
    /// boundary, clipped to the stock.
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :param simplify_tol: Tolerance in mm for polyline simplification.
    /// :returns: List of polygons representing the outer boundary.
    /// :complexity: O(n log n)
    pub fn frontier(
        &self,
        region: &PyStockRegion,
        simplify_tol: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let f = self.inner.frontier(&region.inner, simplify_tol);
        f.into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Compute the "bites" — new material reachable by expanding the
    /// current frontier outward by *step_over*, clipping to the
    /// tool-centre envelope, and subtracting already-cleared portions.
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :param step_over: Lateral step-over in mm.
    /// :param tool_radius: Tool radius (mm) for computing the envelope.
    /// :param simplify_tol: Tolerance in mm for frontier simplification.
    /// :returns: List of polygons representing the bite regions.
    /// :complexity: O(n log n)
    pub fn bites(
        &self,
        region: &PyStockRegion,
        step_over: f64,
        tool_radius: f64,
        simplify_tol: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let bites = self.inner.bites(
            &region.inner,
            step_over,
            tool_radius,
            simplify_tol,
        );
        bites
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Begin buffering single-segment expansions.
    ///
    /// Subsequent calls to ``expand_batched`` are queued without a
    /// union.  Call ``commit_batch`` to union all queued sweeps with
    /// the stored fragments in a single pass.
    ///
    /// Calling this while a batch is already active is a no-op.
    pub fn begin_batch(&mut self) {
        self.inner.begin_batch();
    }

    /// Queue a single-segment expansion into the current batch.
    ///
    /// The segment swept polygon is stored in the internal buffer.
    /// Does **not** perform a union until ``commit_batch`` is called.
    ///
    /// :param prev: Start point ``(x, y)`` of the segment.
    /// :param next: End point ``(x, y)`` of the segment.
    /// :param radius: Disk radius (mm).
    ///
    /// .. warning::
    ///     Panics if ``begin_batch`` was not called first.
    pub fn expand_batched(
        &mut self,
        prev: (f64, f64),
        next: (f64, f64),
        radius: f64,
    ) {
        self.inner.expand_batched(
            Point::new(prev.0, prev.1),
            Point::new(next.0, next.1),
            radius,
        );
    }

    /// Union all buffered sweeps with stored fragments in a single pass,
    /// then rebuild the spatial grid.
    ///
    /// After this call the batch is closed (the caller may start a new
    /// one).
    pub fn commit_batch(&mut self) {
        self.inner.commit_batch();
    }

    /// Union only the buffered sweeps with nearby overlapping fragments,
    /// using the spatial grid to avoid touching distant fragments.
    ///
    /// After this call the batch is closed (the caller may start a new
    /// one).
    pub fn commit_batch_local(&mut self) {
        self.inner.commit_batch_local();
    }

    /// Evaluate engagement at a point using the signed distance to this
    /// cleared area's boundary.
    ///
    /// :param center: Query point ``(x, y)``.
    /// :param radius: Disk radius (mm).
    /// :returns: ``(angle_rad, area, chord_depth)``.
    pub fn get_point_engagement(
        &self,
        center: (f64, f64),
        radius: f64,
    ) -> (f64, f64, f64) {
        let e = self
            .inner
            .get_point_engagement(Point::new(center.0, center.1), radius);
        (e.angle, e.area, e.chord_depth)
    }

    /// Compute angular engagement by exact circle-polygon intersection.
    ///
    /// Creates a disk polygon at *center* with *radius*, intersects it
    /// with all nearby cleared fragments, and returns the uncleared
    /// angular extent in ``[0, 2*pi]``.
    ///
    /// :param center: Query point ``(x, y)``.
    /// :param radius: Disk radius (mm).
    /// :returns: Uncleared angular extent (radians).
    pub fn get_angular_engagement(
        &self,
        center: (f64, f64),
        radius: f64,
    ) -> f64 {
        self.inner
            .get_angular_engagement(Point::new(center.0, center.1), radius)
    }

    /// Incremental cut area when the tool moves from *c1* to *c2*.
    ///
    /// Computes the fresh material area inside the disk at *c2* that is
    /// not already cleared (crescent area).
    ///
    /// :param c1: Previous centre ``(x, y)``.
    /// :param c2: Next centre ``(x, y)``.
    /// :param radius: Disk radius (mm).
    /// :returns: Fresh cut area (mm2).
    pub fn cut_area(&self, c1: (f64, f64), c2: (f64, f64), radius: f64) -> f64 {
        self.inner.cut_area(
            Point::new(c1.0, c1.1),
            Point::new(c2.0, c2.1),
            radius,
        )
    }

    /// Evaluate engagement along a polyline.
    ///
    /// :param path: List of ``(x, y)`` points.
    /// :param radius: Disk radius (mm).
    /// :returns: List of ``(angle, area, chord_depth)`` tuples.
    pub fn path_engagement(
        &self,
        path: Vec<(f64, f64)>,
        radius: f64,
    ) -> Vec<(f64, f64, f64)> {
        let pts: Vec<Point> =
            path.iter().map(|&(x, y)| Point::new(x, y)).collect();
        self.inner
            .path_engagement(&pts, radius)
            .into_iter()
            .map(|e| (e.angle, e.area, e.chord_depth))
            .collect()
    }

    /// Compact fragments if total vertex count exceeds the default
    /// threshold.
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :param tol: Vertex simplification tolerance in mm.
    pub fn compact_if_needed(&mut self, region: &PyStockRegion, tol: f64) {
        self.inner.compact_if_needed(&region.inner, tol);
    }

    /// Compact with an explicit vertex-count threshold.
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :param tol: Vertex simplification tolerance in mm.
    /// :param threshold: Vertex count threshold above which compaction
    ///                   is triggered.
    pub fn compact_if_needed_threshold(
        &mut self,
        region: &PyStockRegion,
        tol: f64,
        threshold: usize,
    ) {
        self.inner
            .compact_if_needed_threshold(&region.inner, tol, threshold);
    }

    /// Total cleared area.
    ///
    /// :returns: Total cleared area in mm2.
    /// :complexity: O(1)
    pub fn total_area(&self) -> f64 {
        self.inner.total_area()
    }

    /// Remaining uncut area (boundary minus islands minus cleared
    /// fragments).  Only positive-area (CCW) polygons are counted,
    /// so island holes do not inflate the result.
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :returns: Remaining uncut area in mm2.
    pub fn remaining_area(&self, region: &PyStockRegion) -> f64 {
        self.inner.remaining_area(&region.inner)
    }

    /// Uncleared area **inside the actionable zone** of the pocket.
    ///
    /// The actionable zone is the boundary inset by
    /// ``inset_distance``, with islands buffered by the same amount.
    /// Material outside this zone -- wall-band slivers thinner than
    /// ``inset_distance`` -- is excluded, so this metric can gate
    /// convergence on whether any *actionable* material remains.
    ///
    /// ``inset_distance`` is typically ``step_length``: slivers
    /// thinner than the per-step advance get skipped by the
    /// stepper, so they should not gate convergence.
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :param inset_distance: Inset distance (mm) defining the
    ///                        actionable zone (boundary inset,
    ///                        islands buffered).
    /// :returns: Actionable remaining area in mm2.
    pub fn actionable_remaining(
        &self,
        region: &PyStockRegion,
        inset_distance: f64,
    ) -> f64 {
        self.inner
            .actionable_remaining(&region.inner, inset_distance)
    }

    /// Return the union of all polygons currently tracked as cleared.
    ///
    /// Each fragment is a closed polygon (list of ``(x, y)`` vertices)
    /// representing an area that has already been cut.  The fragment set
    /// grows as ``cut_fast`` or ``cut`` are called.
    ///
    /// This is useful for inspecting which areas have been cleared.
    ///
    /// :returns: List of polygons representing the cleared fragments.
    /// :complexity: O(m) where m = number of fragments
    pub fn fragments(&self) -> Vec<Vec<(f64, f64)>> {
        self.inner
            .fragments()
            .iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Number of cleared fragments.
    ///
    /// :returns: Fragment count.
    pub fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// The tool-centre envelope (inset of boundary by ``tool_radius``,
    /// minus islands).
    ///
    /// :param region: StockRegion defining the boundary and islands.
    /// :param tool_radius: Tool radius (mm).
    /// :returns: List of polygons representing the tool-centre envelope.
    pub fn envelope(
        &self,
        region: &PyStockRegion,
        tool_radius: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        self.inner
            .envelope(&region.inner, tool_radius)
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// True when no fragments have been recorded.
    ///
    /// :returns: ``True`` if no fragments have been recorded.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("ClearedArea({} fragments)", self.inner.len())
    }
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = cut_mod.py();
    let m = PyModule::new(py, "cleared_area")?;

    m.add_class::<PyClearedArea>()?;

    cut_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.cut.cleared_area", &m)?;

    Ok(())
}
