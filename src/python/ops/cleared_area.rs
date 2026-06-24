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
            },
        }
    }

    #[getter]
    pub fn get_radius(&self) -> f64 {
        self.inner.radius
    }
    #[setter]
    pub fn set_radius(&mut self, v: f64) {
        self.inner.radius = v;
    }
    #[getter]
    pub fn get_step_length(&self) -> f64 {
        self.inner.step_length
    }
    #[setter]
    pub fn set_step_length(&mut self, v: f64) {
        self.inner.step_length = v;
    }
    #[getter]
    pub fn get_target_engagement(&self) -> f64 {
        self.inner.target_engagement
    }
    #[setter]
    pub fn set_target_engagement(&mut self, v: f64) {
        self.inner.target_engagement = v;
    }
    #[getter]
    pub fn get_engagement_tol(&self) -> f64 {
        self.inner.engagement_tol
    }
    #[setter]
    pub fn set_engagement_tol(&mut self, v: f64) {
        self.inner.engagement_tol = v;
    }
    #[getter]
    pub fn get_max_deflection(&self) -> f64 {
        self.inner.max_deflection
    }
    #[setter]
    pub fn set_max_deflection(&mut self, v: f64) {
        self.inner.max_deflection = v;
    }
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
    #[classmethod]
    fn ok(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: crate::ops::cleared_area::StepStatus::Ok,
        }
    }
    #[classmethod]
    fn boundary_hit(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: crate::ops::cleared_area::StepStatus::BoundaryHit,
        }
    }
    #[classmethod]
    fn lost_engagement(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: crate::ops::cleared_area::StepStatus::LostEngagement,
        }
    }
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
    #[pyo3(get)]
    pub next: (f64, f64),
    #[pyo3(get)]
    pub heading: f64,
    #[pyo3(get)]
    pub iters: usize,
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
    /// Faster than add_cleared_polygons when inputs don't overlap
    /// existing fragments (skips the full union).
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
    /// :param simplify_tol: tolerance in mm for polyline simplification
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
    /// :param step_over: lateral step-over in mm
    /// :param valid_area: list of polygons defining the valid tool-centre region
    /// :param simplify_tol: tolerance in mm for frontier simplification
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
    /// :param step_over: lateral step-over in mm
    /// :param valid_area: list of polygons defining the valid tool-centre region
    /// :param simplify_tol: tolerance in mm for frontier simplification
    /// :param target: (x, y) target point to steer toward
    /// :param max_angle: maximum deviation from the target direction (radians)
    /// :complexity: O(n log n)
    pub fn bite_in_direction(
        &self,
        step_over: f64,
        valid_area: Vec<Vec<(f64, f64)>>,
        simplify_tol: f64,
        target: (f64, f64),
        max_angle: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let valid: Vec<crate::types::Polygon> = valid_area
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let bites = self.inner.bite_in_direction(
            step_over,
            &valid,
            simplify_tol,
            crate::types::Point::new(target.0, target.1),
            max_angle,
        );
        bites
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Iteratively call :py:meth:`bites` + :py:meth:`incorporate` until
    /// the valid area is fully cleared.
    ///
    /// Returns all passes, each pass being a list of bite polygons.
    /// The cleared area is fully cleared after this call.
    /// :param step_over: lateral step-over in mm
    /// :param valid_area: list of polygons defining the valid tool-centre region
    /// :param simplify_tol: tolerance in mm for frontier simplification
    /// :complexity: O(k n log n) where k = number of passes
    pub fn all_bites(
        &mut self,
        step_over: f64,
        valid_area: Vec<Vec<(f64, f64)>>,
        simplify_tol: f64,
    ) -> Vec<Vec<Vec<(f64, f64)>>> {
        let valid: Vec<crate::types::Polygon> = valid_area
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let passes = self.inner.all_bites(step_over, &valid, simplify_tol);
        passes
            .into_iter()
            .map(|pass| {
                pass.into_iter()
                    .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
                    .collect()
            })
            .collect()
    }

    /// :complexity: O(1)
    pub fn total_area(&self) -> f64 {
        self.inner.total_area()
    }

    /// Number of cleared fragments.
    pub fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// True when no fragments have been recorded.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Return the union of all polygons currently tracked as cleared.
    ///
    /// Each fragment is a closed polygon (list of ``(x, y)`` vertices)
    /// representing an area that has already been cut.  The fragment set
    /// grows as ``incorporate`` or ``add_cleared_polygons`` are called.
    ///
    /// This is useful for determining which parts of a bite polygon
    /// lie outside the cleared area (i.e. the cutting arc), for example
    /// when used with :py:func:`raygeo.ops.assembly.hsm.find_cutting_arc`.
    /// :complexity: O(m) where m = number of fragments
    pub fn fragments(&self) -> Vec<Vec<(f64, f64)>> {
        self.inner
            .fragments()
            .iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// Compute the inset region of *boundary* by *radius* (excluding
    /// *obstacles*), then return the portions of that region not covered
    /// by stored fragments, together with the original obstacle polygons.
    ///
    /// :param boundary: Outer boundary polygon.
    /// :param obstacles: Obstacle (hole) polygons to exclude.
    /// :param radius: Inset distance applied to *boundary* and *obstacles*.
    /// :returns: List of polygons — the obstacles plus the uncovered
    ///           portion of the inset region.
    /// :complexity: O(n log n) for the inset and difference operations.
    #[pyo3(signature = (boundary, obstacles = None, radius = 3.0))]
    pub fn remaining_in_inset(
        &self,
        boundary: Vec<(f64, f64)>,
        obstacles: Option<Vec<Vec<(f64, f64)>>>,
        radius: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        let boundary_poly: crate::types::Polygon = boundary
            .into_iter()
            .map(|(x, y)| crate::types::Point::new(x, y))
            .collect();
        let obstacles_polys: Vec<crate::types::Polygon> = obstacles
            .unwrap_or_default()
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let result = self.inner.remaining_in_inset(
            &boundary_poly,
            &obstacles_polys,
            radius,
        );
        result
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
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
