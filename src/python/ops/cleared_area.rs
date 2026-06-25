pyo3_stub_gen::module_doc!("raygeo.ops.cleared_area", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Incremental cleared-area tracker.

Maintains a union of swept-disk polygons and provides a spatial-indexed
windowed query for efficient engagement computation.
";

use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::cleared_area::ClearedArea as RustClearedArea;
use crate::ops::cleared_area::UpdateStrategy;
use crate::python::geo::algo::medial_axis::PyMedialAxis;
use crate::types::Point;
use crate::types::Rect;

// ── Stepper types ──

/// Options for the stepping solver.
///
/// Controls disk radius, step length, target engagement angle,
/// solver tolerance, max steering deflection, and iteration budget.
#[gen_stub_pyclass(module = "raygeo.ops.cleared_area")]
#[pyclass(name = "StepperOptions", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStepperOptions {
    pub inner: crate::ops::cleared_area::StepperOptions,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepperOptions {
    /// :param radius: Disk radius in mm (default 3.0).
    /// :param step_length: Forward step length in mm (default 0.6).
    /// :param target_engagement: Target engagement angle in radians (default π).
    /// :param engagement_tol: Engagement tolerance in radians (default 0.01).
    /// :param max_deflection: Maximum steering deflection per step in radians (default π/6).
    /// :param max_solver_iters: Maximum solver iterations per step (default 6).
    #[new]
    #[pyo3(signature = (
        radius = 3.0,
        step_length = 0.6,
        target_engagement = None,
        engagement_tol = 0.01,
        max_deflection = None,
        max_solver_iters = 6,
    ))]
    pub fn new(
        radius: f64,
        step_length: f64,
        target_engagement: Option<f64>,
        engagement_tol: f64,
        max_deflection: Option<f64>,
        max_solver_iters: usize,
    ) -> Self {
        let target = target_engagement.unwrap_or(std::f64::consts::PI);
        let max_def = max_deflection.unwrap_or(std::f64::consts::FRAC_PI_6);
        PyStepperOptions {
            inner: crate::ops::cleared_area::StepperOptions {
                radius,
                step_length,
                target_engagement: target,
                engagement_tol,
                max_deflection: max_def,
                max_solver_iters,
                valid_area: None,
            },
        }
    }

    /// Disk radius in mm.
    #[getter]
    pub fn get_radius(&self) -> f64 {
        self.inner.radius
    }
    #[setter]
    pub fn set_radius(&mut self, v: f64) {
        self.inner.radius = v;
    }
    /// Forward step length in mm.
    #[getter]
    pub fn get_step_length(&self) -> f64 {
        self.inner.step_length
    }
    #[setter]
    pub fn set_step_length(&mut self, v: f64) {
        self.inner.step_length = v;
    }
    /// Target engagement angle in radians.
    #[getter]
    pub fn get_target_engagement(&self) -> f64 {
        self.inner.target_engagement
    }
    #[setter]
    pub fn set_target_engagement(&mut self, v: f64) {
        self.inner.target_engagement = v;
    }
    /// Engagement tolerance in radians.
    #[getter]
    pub fn get_engagement_tol(&self) -> f64 {
        self.inner.engagement_tol
    }
    #[setter]
    pub fn set_engagement_tol(&mut self, v: f64) {
        self.inner.engagement_tol = v;
    }
    /// Maximum steering deflection per step in radians.
    #[getter]
    pub fn get_max_deflection(&self) -> f64 {
        self.inner.max_deflection
    }
    #[setter]
    pub fn set_max_deflection(&mut self, v: f64) {
        self.inner.max_deflection = v;
    }
    /// Maximum solver iterations per step.
    #[getter]
    pub fn get_max_solver_iters(&self) -> usize {
        self.inner.max_solver_iters
    }
    #[setter]
    pub fn set_max_solver_iters(&mut self, v: usize) {
        self.inner.max_solver_iters = v;
    }

    fn __repr__(&self) -> String {
        format!(
            "StepperOptions(R={}, step={}, target={:.3}, max_def={:.3}, iters={})",
            self.inner.radius,
            self.inner.step_length,
            self.inner.target_engagement,
            self.inner.max_deflection,
            self.inner.max_solver_iters,
        )
    }
}

/// Status of a single step or cut segment.
///
/// One of ``Ok`` (normal), ``BoundaryHit`` (hit pocket boundary),
/// ``LostEngagement`` (no uncut material), or ``NoConvergence``
/// (solver failed to converge).
#[gen_stub_pyclass(module = "raygeo.ops.cleared_area")]
#[pyclass(name = "StepStatus", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStepStatus {
    pub inner: crate::ops::cleared_area::StepStatus,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepStatus {
    /// Normal step completion.
    /// :returns: ``StepStatus.ok``
    #[classmethod]
    fn ok(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: crate::ops::cleared_area::StepStatus::Ok,
        }
    }
    /// Hit pocket boundary.
    /// :returns: ``StepStatus.boundary_hit``
    #[classmethod]
    fn boundary_hit(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: crate::ops::cleared_area::StepStatus::BoundaryHit,
        }
    }
    /// No uncut material found.
    /// :returns: ``StepStatus.lost_engagement``
    #[classmethod]
    fn lost_engagement(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: crate::ops::cleared_area::StepStatus::LostEngagement,
        }
    }
    /// Solver failed to converge.
    /// :returns: ``StepStatus.no_convergence``
    #[classmethod]
    fn no_convergence(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: crate::ops::cleared_area::StepStatus::NoConvergence,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Result of a single forward step.
///
/// Contains the next centre position, updated heading,
/// solver iteration count, and the final status.
#[gen_stub_pyclass(module = "raygeo.ops.cleared_area")]
#[pyclass(name = "StepResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStepResult {
    /// Next centre position ``(x, y)``.
    #[pyo3(get)]
    pub next: (f64, f64),
    /// Updated heading angle in radians.
    #[pyo3(get)]
    pub heading: f64,
    /// Number of solver iterations used.
    #[pyo3(get)]
    pub iters: usize,
    /// Step completion status.
    #[pyo3(get)]
    pub status: PyStepStatus,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepResult {
    fn __repr__(&self) -> String {
        format!(
            "StepResult(next=({:.3},{:.3}), heading={:.3}, status={:?})",
            self.next.0, self.next.1, self.heading, self.status.inner,
        )
    }
}

// ── ClearedArea class ──

#[gen_stub_pyclass(module = "raygeo.ops.cleared_area")]
#[pyclass]
pub struct ClearedArea {
    pub(crate) inner: RustClearedArea,
}

#[gen_stub_pymethods]
#[pymethods]
impl ClearedArea {
    #[new]
    #[pyo3(signature = (initial = None))]
    pub fn new(initial: Option<Vec<Vec<(f64, f64)>>>) -> Self {
        match initial {
            Some(polys) => {
                let polygons: Vec<crate::types::Polygon> = polys
                    .into_iter()
                    .map(|v| {
                        v.into_iter()
                            .map(|(x, y)| crate::types::Point::new(x, y))
                            .collect()
                    })
                    .collect();
                ClearedArea {
                    inner: RustClearedArea::from_polygons(&polygons),
                }
            }
            None => ClearedArea {
                inner: RustClearedArea::new(),
            },
        }
    }

    /// Sweep a disk along a polyline, adding the swept area to the cleared set.
    ///
    /// :param path: List of ``(x, y)`` points forming the polyline.
    /// :param radius: Disk radius (mm).
    /// :complexity: O(n) where n = number of path points
    pub fn expand(&mut self, path: Vec<(f64, f64)>, radius: f64) {
        let path: Vec<crate::types::Point> = path
            .into_iter()
            .map(|(x, y)| crate::types::Point::new(x, y))
            .collect();
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
            crate::types::Point::new(prev.0, prev.1),
            crate::types::Point::new(next.0, next.1),
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
    /// :returns: Signed distance in mm.  ``0.0`` means exactly on the boundary.
    pub fn signed_boundary_distance(&self, x: f64, y: f64) -> f64 {
        self.inner.signed_boundary_distance(x, y)
    }

    /// Add pre‑computed polygons to the cleared set.
    ///
    /// :param polygons: List of polygons (each a list of ``(x, y)`` vertices) to add.
    /// :complexity: O(n) where n = total vertices across all polygons
    pub fn add_cleared_polygons(&mut self, polygons: Vec<Vec<(f64, f64)>>) {
        let polys: Vec<crate::types::Polygon> = polygons
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        self.inner.add_cleared_polygons(&polys);
    }

    /// Return fragments whose bounding box overlaps the query window.
    ///
    /// :param bbox: Bounding box ``(x_min, y_min, x_max, y_max)``.
    /// :returns: Fragments intersecting the bounding box.
    /// :complexity: O(m + k) where m = number of fragments, k = output vertices
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

    /// Subtract cleared fragments from the boundary polygons, returning the uncut region.
    ///
    /// :param bounds: Boundary polygons defining the region of interest.
    /// :returns: List of polygons representing the uncut portion.
    /// :complexity: O(n * m) where n = bounds vertices, m = fragments
    pub fn remaining(
        &self,
        bounds: Vec<Vec<(f64, f64)>>,
    ) -> Vec<Vec<(f64, f64)>> {
        let bounds_polys: Vec<crate::types::Polygon> = bounds
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let remaining = self.inner.remaining(&bounds_polys);
        remaining
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Add polygons, returning only the newly-added portion.
    /// Faster than ``add_cleared_polygons`` when inputs don't overlap
    /// existing fragments (skips the full union).
    ///
    /// :param polygons: List of polygons to add.
    /// :returns: List of polygons representing the newly-added portion.
    /// :complexity: O(n log n) worst case when union required,
    ///              O(n) when inputs are disjoint from existing fragments
    pub fn incorporate(
        &mut self,
        polygons: Vec<Vec<(f64, f64)>>,
    ) -> Vec<Vec<(f64, f64)>> {
        let polys: Vec<crate::types::Polygon> = polygons
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let new = self.inner.incorporate(&polys);
        new.into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Return a unioned, simplified snapshot of the current outer boundary.
    ///
    /// :param simplify_tol: Tolerance in mm for polyline simplification.
    /// :returns: List of polygons representing the outer boundary.
    /// :complexity: O(n log n)
    pub fn frontier(&self, simplify_tol: f64) -> Vec<Vec<(f64, f64)>> {
        let f = self.inner.frontier(simplify_tol);
        f.into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Compute the "bites" — new material reachable by expanding the
    /// current frontier outward by step_over, clipping to valid_area,
    /// and subtracting already-cleared portions.
    ///
    /// :param step_over: Lateral step-over in mm.
    /// :param valid_area: List of polygons defining the valid tool-centre region.
    /// :param simplify_tol: Tolerance in mm for frontier simplification.
    /// :returns: List of polygons representing the bite regions.
    /// :complexity: O(n log n)
    pub fn bites(
        &self,
        step_over: f64,
        valid_area: Vec<Vec<(f64, f64)>>,
        simplify_tol: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let valid: Vec<crate::types::Polygon> = valid_area
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let bites = self.inner.bites(step_over, &valid, simplify_tol);
        bites
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Like :py:meth:`bites` but filters to only the bites whose centroid
    /// lies within *max_angle* radians of the direction from the current
    /// cleared region's centre toward *target*.
    /// useful for steering the clearing direction along a MAT branch.
    ///
    /// :param step_over: Lateral step-over in mm.
    /// :param valid_area: List of polygons defining the valid tool-centre region.
    /// :param simplify_tol: Tolerance in mm for frontier simplification.
    /// Total cleared area.
    ///
    /// :returns: Total cleared area in mm².
    /// :complexity: O(1)
    pub fn total_area(&self) -> f64 {
        self.inner.total_area()
    }

    /// Number of cleared fragments.
    ///
    /// :returns: Fragment count.
    pub fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// True when no fragments have been recorded.
    ///
    /// :returns: ``True`` if no fragments have been recorded.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Return the union of all polygons currently tracked as cleared.
    ///
    /// Each fragment is a closed polygon (list of ``(x, y)`` vertices)
    /// representing an area that has already been cut.  The fragment set
    /// grows as ``incorporate`` or ``add_cleared_polygons`` are called.
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

    fn __repr__(&self) -> String {
        format!("ClearedArea({} fragments)", self.inner.len())
    }

    // ── Stepper methods ──

    /// Perform one forward step.
    ///
    /// Starting from *pos* with the given *heading* (radians), proposes
    /// candidate positions and solves for the heading that maintains the
    /// target engagement.
    ///
    /// :param pos: Current centre position ``(x, y)``.
    /// :param heading: Current heading angle in radians.
    /// :param opts: ``StepperOptions`` controlling the solver.
    /// :returns: ``StepResult`` with the next position and updated heading.
    pub fn step(
        &self,
        pos: (f64, f64),
        heading: f64,
        opts: &PyStepperOptions,
    ) -> PyStepResult {
        let r = self
            .inner
            .step(Point::new(pos.0, pos.1), heading, &opts.inner);
        PyStepResult {
            next: (r.next.x, r.next.y),
            heading: r.heading,
            iters: r.iters,
            status: PyStepStatus { inner: r.status },
        }
    }

    /// Drive the disk forward until a non-Ok status or *max_steps*.
    ///
    /// Does **not** modify the ClearedArea — the caller is responsible for
    /// committing swept polygons.
    ///
    /// :param start: Starting position ``(x, y)``.
    /// :param initial_heading: Initial heading angle (radians).
    /// :param opts: ``StepperOptions`` controlling the solver.
    /// :param max_steps: Maximum number of steps.
    /// :returns: ``(path, status_string)``.
    pub fn run_segment(
        &self,
        start: (f64, f64),
        initial_heading: f64,
        opts: &PyStepperOptions,
        max_steps: usize,
    ) -> (Vec<(f64, f64)>, String) {
        let (path, status) = self.inner.run_segment(
            Point::new(start.0, start.1),
            initial_heading,
            &opts.inner,
            max_steps,
        );
        let path_out: Vec<(f64, f64)> =
            path.into_iter().map(|p| (p.x, p.y)).collect();
        (path_out, format!("{status:?}"))
    }

    // ── Batched step expansion ──

    /// Begin buffering single‑segment expansions.
    ///
    /// Subsequent calls to ``expand_step_batched`` are queued without a
    /// union.  Call ``commit_step_batch`` to union all queued sweeps with
    /// the stored fragments in a single pass.
    ///
    /// Calling this while a batch is already active is a no‑op.
    pub fn begin_step_batch(&mut self) {
        self.inner.begin_step_batch();
    }

    /// Queue a single‑segment expansion into the current batch.
    ///
    /// The segment swept polygon is stored in the internal buffer.
    /// Does **not** perform a union until ``commit_step_batch`` is called.
    ///
    /// :param prev: Start point ``(x, y)`` of the segment.
    /// :param next: End point ``(x, y)`` of the segment.
    /// :param radius: Disk radius (mm).
    ///
    /// .. warning::
    ///     Panics if ``begin_step_batch`` was not called first.
    pub fn expand_step_batched(
        &mut self,
        prev: (f64, f64),
        next: (f64, f64),
        radius: f64,
    ) {
        self.inner.expand_step_batched(
            crate::types::Point::new(prev.0, prev.1),
            crate::types::Point::new(next.0, next.1),
            radius,
        );
    }

    /// Union all buffered sweeps with stored fragments in a single pass,
    /// then rebuild the spatial grid.
    ///
    /// After this call the batch is closed (the caller may start a new one).
    pub fn commit_step_batch(&mut self) {
        self.inner.commit_step_batch();
    }

    /// Evaluate engagement at a point using the signed distance to this
    /// cleared area's boundary.
    ///
    /// :param center: Query point ``(x, y)``.
    /// :param radius: Disk radius (mm).
    /// :returns: ``(angle_rad, area, chord_depth)``.
    pub fn point_engagement(
        &self,
        center: (f64, f64),
        radius: f64,
    ) -> (f64, f64, f64) {
        let e = self
            .inner
            .point_engagement(Point::new(center.0, center.1), radius);
        (e.angle, e.area, e.chord_depth)
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

    /// Walk the cleared-area frontier forward from a point near `end_pos`
    /// and return the first position where engagement ≥ `min_engagement`.
    ///
    /// :param mat: Medial Axis of the domain (computed once per level).
    /// :param end_pos: Current position where the path ended.
    /// :param radius: Disk radius (mm).
    /// :param min_engagement: Minimum engagement angle (radians) required.
    /// :returns: ``ResumePoint`` or ``None``.
    pub fn find_next_resume(
        &self,
        mat: &PyMedialAxis,
        end_pos: (f64, f64),
        radius: f64,
        min_engagement: f64,
    ) -> Option<PyResumePoint> {
        let r = self.inner.find_next_resume(
            &mat.inner,
            Point::new(end_pos.0, end_pos.1),
            radius,
            min_engagement,
        )?;
        Some(PyResumePoint {
            pos: (r.pos.x, r.pos.y),
            heading: r.heading,
            link_path: r.link_path.into_iter().map(|p| (p.x, p.y)).collect(),
        })
    }

    /// Switch between global and local fragment-merging strategies.
    ///
    /// :param strategy: Either ``"global"`` or ``"local"``.
    pub fn set_update_strategy(&mut self, strategy: &str) {
        match strategy {
            "local" => self.inner.set_update_strategy(UpdateStrategy::Local),
            _ => self.inner.set_update_strategy(UpdateStrategy::Global),
        }
    }

    /// Compact fragments if total vertex count exceeds the default threshold.
    ///
    /// :param tol: Vertex simplification tolerance in mm.
    pub fn compact_if_needed(&mut self, tol: f64) {
        self.inner.compact_if_needed(tol);
    }

    /// Compact with an explicit vertex-count threshold.
    ///
    /// :param tol: Vertex simplification tolerance in mm.
    /// :param threshold: Vertex count threshold above which compaction is triggered.
    pub fn compact_if_needed_threshold(&mut self, tol: f64, threshold: usize) {
        self.inner.compact_if_needed_threshold(tol, threshold);
    }
}

/// A resume point found on the cleared-area frontier.
#[gen_stub_pyclass(module = "raygeo.ops.cleared_area")]
#[pyclass(name = "ResumePoint", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyResumePoint {
    /// Position on the frontier ``(x, y)``.
    #[pyo3(get)]
    pub pos: (f64, f64),
    /// Outward-normal heading (radians).
    #[pyo3(get)]
    pub heading: f64,
    /// Travel polyline through cleared territory.
    #[pyo3(get)]
    pub link_path: Vec<(f64, f64)>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyResumePoint {
    /// :param pos: Position on the frontier ``(x, y)``.
    /// :param heading: Outward-normal heading in radians.
    /// :param link_path: Travel polyline through cleared territory.
    #[new]
    #[pyo3(signature = (pos, heading, link_path))]
    pub fn new(
        pos: (f64, f64),
        heading: f64,
        link_path: Vec<(f64, f64)>,
    ) -> Self {
        PyResumePoint {
            pos,
            heading,
            link_path,
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "ResumePoint(pos=({:.3},{:.3}), heading={:.3}, link_len={})",
            self.pos.0,
            self.pos.1,
            self.heading,
            self.link_path.len(),
        )
    }
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "cleared_area")?;
    m.setattr("__doc__", MODULE_DOC)?;

    m.add_class::<ClearedArea>()?;
    m.add_class::<PyStepperOptions>()?;
    m.add_class::<PyStepStatus>()?;
    m.add_class::<PyStepResult>()?;
    m.add_class::<PyResumePoint>()?;
    m.add_function(wrap_pyfunction!(target_engagement_from_advance_py, &m)?)?;

    algo_mod.add_submodule(&m)?;
    Ok(())
}

// ── Module-level functions ──

/// Derive the target engagement angle from the advance ratio.
///
/// :param advance: Per-step forward distance (mm).
/// :param radius: Disk radius (mm).
/// :returns: Engagement angle in radians.
#[gen_stub_pyfunction(module = "raygeo.ops.cleared_area")]
#[pyfunction(name = "target_engagement_from_advance")]
fn target_engagement_from_advance_py(advance: f64, radius: f64) -> f64 {
    crate::ops::cleared_area::target_engagement_from_advance(advance, radius)
}
