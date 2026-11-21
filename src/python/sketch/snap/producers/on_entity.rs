//! Snap points on lines, arcs, and circles.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapLineType, PySnapPoint,
};
use crate::sketch::entities::{arc::Arc, circle::Circle, line::Line};

#[gen_stub_pyclass]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.sketch.snap.producers",
    name = "OnEntityProducer"
)]
#[derive(Default)]
pub struct PyOnEntityProducer;

#[gen_stub_pymethods]
#[pymethods]
impl PyOnEntityProducer {
    #[new]
    fn new() -> Self {
        Self
    }

    fn produce(
        &self,
        _py: Python<'_>,
        _registry: &Bound<'_, PyEntityRegistry>,
        _drag_position: (f64, f64),
        _drag_context: &PyDragContext,
        _threshold: f64,
    ) -> Vec<Py<PySnapLine>> {
        Vec::new()
    }

    fn produce_points(
        &self,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        drag_position: (f64, f64),
        drag_context: &PyDragContext,
        threshold: f64,
    ) -> PyResult<Vec<Py<PySnapPoint>>> {
        let reg = registry.borrow();
        let (x, y) = drag_position;
        let mut points = Vec::new();
        for entity_id in reg.inner.entity_ids() {
            if drag_context.is_entity_dragged(entity_id) {
                continue;
            }
            let Some(entity) = reg.inner.get_entity(entity_id) else {
                continue;
            };
            if entity
                .get_point_ids()
                .iter()
                .any(|pid| drag_context.is_point_dragged(*pid))
            {
                continue;
            }
            let nearest = if let Some(line) =
                entity.as_any().downcast_ref::<Line>()
            {
                nearest_on_line(line, &reg.inner, x, y, threshold)
            } else if let Some(arc) = entity.as_any().downcast_ref::<Arc>() {
                nearest_on_arc(arc, &reg.inner, x, y, threshold)
            } else if let Some(circle) =
                entity.as_any().downcast_ref::<Circle>()
            {
                nearest_on_circle(circle, &reg.inner, x, y, threshold)
            } else {
                None
            };
            if let Some((nx, ny)) = nearest {
                let source = registry
                    .call_method1("get_entity_object", (entity_id,))?
                    .extract::<Option<Py<PyAny>>>()?;
                points.push(Py::new(
                    py,
                    PySnapPoint {
                        x: nx,
                        y: ny,
                        line_type: PySnapLineType(
                            PySnapLineType::ON_ENTITY_VALUE,
                        ),
                        source,
                        spacing: None,
                        is_horizontal: false,
                        pattern_coords: None,
                        axis_coord: None,
                    },
                )?);
            }
        }
        Ok(points)
    }
}

fn nearest_on_line(
    line: &Line,
    registry: &crate::sketch::registry::EntityRegistry,
    x: f64,
    y: f64,
    threshold: f64,
) -> Option<(f64, f64)> {
    let p1 = registry.get_point(line.p1)?;
    let p2 = registry.get_point(line.p2)?;
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-10 {
        return ((x - p1.x).hypot(y - p1.y) <= threshold)
            .then_some((p1.x, p1.y));
    }
    let t = (((x - p1.x) * dx + (y - p1.y) * dy) / len_sq).clamp(0.0, 1.0);
    let nx = p1.x + t * dx;
    let ny = p1.y + t * dy;
    ((x - nx).hypot(y - ny) <= threshold).then_some((nx, ny))
}

fn nearest_on_arc(
    arc: &Arc,
    registry: &crate::sketch::registry::EntityRegistry,
    x: f64,
    y: f64,
    threshold: f64,
) -> Option<(f64, f64)> {
    let center = registry.get_point(arc.center)?;
    let start = registry.get_point(arc.start)?;
    let radius = (start.x - center.x).hypot(start.y - center.y);
    if radius < 1e-10 {
        return None;
    }
    let angle = if (x - center.x).hypot(y - center.y) < 1e-10 {
        0.0
    } else {
        (y - center.y).atan2(x - center.x)
    };
    if !arc.is_angle_within_sweep(angle, registry) {
        return None;
    }
    let nx = center.x + radius * angle.cos();
    let ny = center.y + radius * angle.sin();
    ((x - nx).hypot(y - ny) <= threshold).then_some((nx, ny))
}

fn nearest_on_circle(
    circle: &Circle,
    registry: &crate::sketch::registry::EntityRegistry,
    x: f64,
    y: f64,
    threshold: f64,
) -> Option<(f64, f64)> {
    let center = registry.get_point(circle.center)?;
    let radius_point = registry.get_point(circle.radius_point)?;
    let radius = (radius_point.x - center.x).hypot(radius_point.y - center.y);
    if radius < 1e-10 {
        return None;
    }
    let angle = if (x - center.x).hypot(y - center.y) < 1e-10 {
        0.0
    } else {
        (y - center.y).atan2(x - center.x)
    };
    let nx = center.x + radius * angle.cos();
    let ny = center.y + radius * angle.sin();
    ((x - nx).hypot(y - ny) <= threshold).then_some((nx, ny))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyOnEntityProducer>()?;
    Ok(())
}
