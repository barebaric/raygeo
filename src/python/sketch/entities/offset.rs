//! Python bindings for the sketch offset engine.  The algorithms
//! live in [`crate::sketch::offset`]; this layer only converts
//! between the core value plans and the Python objects the host
//! manipulates.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraint::PointContext;
use crate::sketch::entities::entity::entity_from_json;
use crate::sketch::offset as core;
use crate::sketch::transform::PlacementTransform;

/// Description of the changes one offset operation makes.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch.entities", name = "OffsetPlan")]
pub struct PyOffsetPlan {
    #[pyo3(get, set)]
    pub point_moves: Py<PyDict>,
    #[pyo3(get, set)]
    pub points: Py<PyList>,
    #[pyo3(get, set)]
    pub entities: Py<PyList>,
    #[pyo3(get, set)]
    pub removed_entity_ids: Py<PyList>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyOffsetPlan {
    #[new]
    fn new(py: Python<'_>) -> Self {
        PyOffsetPlan {
            point_moves: PyDict::new(py).unbind(),
            points: PyList::empty(py).unbind(),
            entities: PyList::empty(py).unbind(),
            removed_entity_ids: PyList::empty(py).unbind(),
        }
    }
}

impl PyOffsetPlan {
    fn new_in(py: Python<'_>) -> Self {
        Self::new(py)
    }

    /// Materializes a core plan: moves into the dict, planned points
    /// into fresh `SketchPoint` objects, entity dicts into entity
    /// objects through `entity_from_json`.
    fn from_core(py: Python<'_>, plan: core::OffsetPlan) -> PyResult<Self> {
        let object = Self::new_in(py);
        for (id, (x, y)) in plan.point_moves {
            object.point_moves.bind(py).set_item(id, (x, y))?;
        }
        for point in plan.points {
            object
                .points
                .bind(py)
                .append(planned_point_to_py(py, point)?)?;
        }
        for value in plan.entities {
            let Some(entity) = entity_from_json(&value) else {
                return Err(PyValueError::new_err(format!(
                    "offset plan produced an unknown entity: {value}"
                )));
            };
            let entity_obj = super::entity_to_py(py, entity.as_ref(), None)?;
            object.entities.bind(py).append(entity_obj)?;
        }
        for id in plan.removed_entity_ids {
            object.removed_entity_ids.bind(py).append(id)?;
        }
        Ok(object)
    }
}

/// A sampled outline produced from a connected chain of entities.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch.entities", name = "PolygonOutline")]
pub struct PyPolygonOutline {
    #[pyo3(get, set)]
    pub vertices: Vec<(f64, f64)>,
    #[pyo3(get, set)]
    pub closed: bool,
    #[pyo3(get, set)]
    pub source_ids: Vec<u64>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPolygonOutline {
    #[new]
    #[pyo3(signature = (vertices, closed=false, source_ids=None))]
    fn new(
        vertices: Vec<(f64, f64)>,
        closed: bool,
        source_ids: Option<Vec<u64>>,
    ) -> Self {
        PyPolygonOutline {
            vertices,
            closed,
            source_ids: source_ids.unwrap_or_default(),
        }
    }

    /// Offsets the outline into `(vertices, closed)` pairs.
    fn offset(
        &self,
        distance: f64,
        tolerance: f64,
    ) -> Vec<(Vec<(f64, f64)>, bool)> {
        crate::sketch::contour::offset_outline(
            &self.vertices,
            self.closed,
            distance,
            tolerance,
        )
    }

    /// Plans the replacement geometry for this outline.
    fn plan_offset(
        &self,
        py: Python<'_>,
        _registry: crate::python::sketch::context::PointCtx<'_>,
        offset: f64,
        allocate_id: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyOffsetPlan>> {
        self.plan(py, offset, allocate_id)
    }

    /// Coarse polylines of the offset result for live preview.
    fn preview_polylines(
        &self,
        _registry: crate::python::sketch::context::PointCtx<'_>,
        offset: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        core::preview_of_outline(&self.as_core(), offset)
    }
}

impl PyPolygonOutline {
    fn as_core(&self) -> core::Outline {
        core::Outline {
            vertices: self.vertices.clone(),
            closed: self.closed,
            source_ids: self.source_ids.clone(),
        }
    }

    fn from_core(outline: core::Outline) -> Self {
        PyPolygonOutline {
            vertices: outline.vertices,
            closed: outline.closed,
            source_ids: outline.source_ids,
        }
    }

    fn plan(
        &self,
        py: Python<'_>,
        offset: f64,
        allocate_id: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyOffsetPlan>> {
        let outline = self.as_core();
        let allocator = PyAllocator::new(allocate_id);
        let plan = core::plan_of_outline(&outline, offset, &mut || {
            allocator.next_id()
        });
        allocator.finish()?;
        plan.map(|plan| PyOffsetPlan::from_core(py, plan))
            .transpose()
    }
}

/// Runs the host's Python id allocator behind the core allocator
/// closure, remembering the first call error.
struct PyAllocator<'a> {
    allocate_id: &'a Bound<'a, PyAny>,
    error: std::cell::RefCell<Option<PyErr>>,
}

impl<'a> PyAllocator<'a> {
    fn new(allocate_id: &'a Bound<'a, PyAny>) -> Self {
        PyAllocator {
            allocate_id,
            error: std::cell::RefCell::new(None),
        }
    }

    fn next_id(&self) -> u64 {
        if self.error.borrow().is_some() {
            return u64::MAX;
        }
        match self
            .allocate_id
            .call0()
            .and_then(|value| value.extract::<i64>())
        {
            Ok(id) => id as u64,
            Err(err) => {
                *self.error.borrow_mut() = Some(err);
                u64::MAX
            }
        }
    }

    fn finish(&self) -> PyResult<()> {
        match self.error.borrow_mut().take() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

/// Materializes a planned point as a detached `SketchPoint`.
fn planned_point_to_py<'py>(
    py: Python<'py>,
    point: core::PlannedPoint,
) -> PyResult<Bound<'py, PyAny>> {
    use crate::sketch::entities::point::{SketchPoint, WaypointType};

    Ok(Py::new(
        py,
        super::point::PySketchPoint {
            inner: SketchPoint {
                id: point.id,
                x: point.x,
                y: point.y,
                fixed: false,
                waypoint_type: WaypointType::Sharp,
                constrained: false,
            },
            registry: None,
        },
    )?
    .into_bound(py)
    .into_any())
}

/// Adapts the host's duck-typed placement object to the core
/// [`PlacementTransform`], remembering the first call error.
struct PyPlacement<'a> {
    placement: &'a Bound<'a, PyAny>,
    error: std::cell::RefCell<Option<PyErr>>,
}

impl<'a> PyPlacement<'a> {
    fn new(placement: &'a Bound<'a, PyAny>) -> Self {
        PyPlacement {
            placement,
            error: std::cell::RefCell::new(None),
        }
    }

    fn map(&self, method: &str, x: f64, y: f64) -> (f64, f64) {
        if self.error.borrow().is_some() {
            return (x, y);
        }
        match self
            .placement
            .call_method1(method, (x, y))
            .and_then(|value| value.extract::<(f64, f64)>())
        {
            Ok(mapped) => mapped,
            Err(err) => {
                *self.error.borrow_mut() = Some(err);
                (x, y)
            }
        }
    }

    fn finish(&self) -> PyResult<()> {
        match self.error.borrow_mut().take() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

impl PlacementTransform for PyPlacement<'_> {
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        self.map("transform_point", x, y)
    }

    fn transform_offset(&self, dx: f64, dy: f64) -> (f64, f64) {
        self.map("transform_offset", dx, dy)
    }
}

/// Plan a circle offset: the radius point moves radially.
pub fn circle_plan(
    py: Python<'_>,
    circle: &crate::sketch::entities::circle::Circle,
    model: &dyn PointContext,
    offset: f64,
) -> PyResult<Option<PyOffsetPlan>> {
    core::circle_plan(circle, model, offset)
        .map(|plan| PyOffsetPlan::from_core(py, plan))
        .transpose()
}

/// Preview a circle offset as a concentric ring.
pub fn circle_preview(
    circle: &crate::sketch::entities::circle::Circle,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    core::circle_preview(circle, model, offset)
}

/// Plan an arc offset: both endpoints move radially onto the offset
/// ring while the center stays fixed.
pub fn arc_plan(
    py: Python<'_>,
    arc: &crate::sketch::entities::arc::Arc,
    model: &dyn PointContext,
    offset: f64,
) -> PyResult<Option<PyOffsetPlan>> {
    core::arc_plan(arc, model, offset)
        .map(|plan| PyOffsetPlan::from_core(py, plan))
        .transpose()
}

/// Preview an arc offset as a concentric arc.
pub fn arc_preview(
    arc: &crate::sketch::entities::arc::Arc,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    core::arc_preview(arc, model, offset)
}

/// Plan an ellipse offset: both radius points move radially.
pub fn ellipse_plan(
    py: Python<'_>,
    ellipse: &crate::sketch::entities::ellipse::Ellipse,
    model: &dyn PointContext,
    offset: f64,
) -> PyResult<Option<PyOffsetPlan>> {
    core::ellipse_plan(ellipse, model, offset)
        .map(|plan| PyOffsetPlan::from_core(py, plan))
        .transpose()
}

/// Preview an ellipse offset as a parametric sampling.
pub fn ellipse_preview(
    ellipse: &crate::sketch::entities::ellipse::Ellipse,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    core::ellipse_preview(ellipse, model, offset)
}

/// Builds a frame-local polygon (plus its two frame points) from an
/// offset ring, with ids from the host allocator.
pub(crate) fn outline_item<'py>(
    py: Python<'py>,
    vertices: &[(f64, f64)],
    closed: bool,
    allocate_id: &Bound<'_, PyAny>,
    holes: &[Vec<(f64, f64)>],
) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>, Bound<'py, PyAny>)> {
    let allocator = PyAllocator::new(allocate_id);
    let item = core::outline_plan_item(
        &mut || allocator.next_id(),
        vertices,
        closed,
        holes,
    );
    allocator.finish()?;
    let Some((center, handle, entity_value)) = item else {
        return Err(PyValueError::new_err("outline produced no plan item"));
    };
    let center_pt = planned_point_to_py(py, center)?;
    let handle_pt = planned_point_to_py(py, handle)?;
    let Some(entity) = entity_from_json(&entity_value) else {
        return Err(PyValueError::new_err(
            "outline produced an unknown entity",
        ));
    };
    let entity_obj = super::entity_to_py(py, entity.as_ref(), None)?;
    Ok((center_pt, handle_pt, entity_obj))
}

/// The offsettable item an entity forms on its own.
///
/// Circles, arcs and ellipses pass through as themselves; open
/// polygons become a sampled outline. Other entity kinds return
/// `None`.
pub fn offset_item_for_entity<'py>(
    py: Python<'py>,
    entity_obj: &Bound<'py, PyAny>,
    registry: &Bound<'_, PyAny>,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    let entity = super::extract_entity(entity_obj)?;
    let any = entity.as_any();
    if any
        .downcast_ref::<crate::sketch::entities::circle::Circle>()
        .is_some()
        || any
            .downcast_ref::<crate::sketch::entities::arc::Arc>()
            .is_some()
        || any
            .downcast_ref::<crate::sketch::entities::ellipse::Ellipse>()
            .is_some()
    {
        return Ok(Some(entity_obj.clone()));
    }
    if let Some(polygon) =
        any.downcast_ref::<crate::sketch::entities::polygon::PolygonEntity>()
    {
        if polygon.closed {
            return Ok(Some(entity_obj.clone()));
        }
        let ctx = crate::python::sketch::context::PointCtx(registry.clone());
        return core::outline_of_open_polygon(polygon, &ctx)
            .map(|outline| {
                Py::new(py, PyPolygonOutline::from_core(outline))
                    .map(|p| p.into_bound(py).into_any())
            })
            .transpose();
    }
    Ok(None)
}

pub fn polygon_plan(
    py: Python<'_>,
    polygon: &crate::sketch::entities::polygon::PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
    allocate_id: &Bound<'_, PyAny>,
) -> PyResult<Option<PyOffsetPlan>> {
    let allocator = PyAllocator::new(allocate_id);
    let plan =
        core::polygon_plan(polygon, model, offset, &mut || allocator.next_id());
    allocator.finish()?;
    plan.map(|plan| PyOffsetPlan::from_core(py, plan))
        .transpose()
}

pub fn polygon_preview(
    polygon: &crate::sketch::entities::polygon::PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    core::polygon_preview(polygon, model, offset)
}

pub fn bezier_transform_offsets(
    bezier: &mut crate::sketch::entities::bezier::Bezier,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let adapter = PyPlacement::new(placement);
    core::bezier_transform_offsets(bezier, &adapter);
    adapter.finish()
}

pub fn bezier_rewrite_offsets_from(
    bezier: &mut crate::sketch::entities::bezier::Bezier,
    template: &crate::sketch::entities::bezier::Bezier,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let adapter = PyPlacement::new(placement);
    core::bezier_rewrite_offsets_from(bezier, template, &adapter);
    adapter.finish()
}

pub fn polygon_transform_offsets(
    polygon: &mut crate::sketch::entities::polygon::PolygonEntity,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let adapter = PyPlacement::new(placement);
    core::polygon_transform_offsets(polygon, &adapter);
    adapter.finish()
}

pub fn polygon_rewrite_offsets_from(
    polygon: &mut crate::sketch::entities::polygon::PolygonEntity,
    template: &crate::sketch::entities::polygon::PolygonEntity,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let adapter = PyPlacement::new(placement);
    core::polygon_rewrite_offsets_from(polygon, template, &adapter);
    adapter.finish()
}

pub fn text_box_rewrite_offsets_from(
    text_box: &mut crate::sketch::entities::text_box::TextBoxEntity,
    template: &crate::sketch::entities::text_box::TextBoxEntity,
) {
    core::text_box_rewrite_offsets_from(text_box, template);
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyOffsetPlan>()?;
    m.add_class::<PyPolygonOutline>()?;
    Ok(())
}
