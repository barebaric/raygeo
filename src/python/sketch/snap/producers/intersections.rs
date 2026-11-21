//! Snap points at entity intersections.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::shape::circle::{
    get_circle_circle_intersections, get_line_circle_intersections,
};
use crate::geo::shape::line::get_line_segment_intersection;
use crate::geo::types::Point;
use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapLineType, PySnapPoint,
};
use crate::sketch::entities::{
    arc::Arc, circle::Circle, entity::Entity, line::Line,
};
use crate::sketch::registry::EntityRegistry;

#[gen_stub_pyclass]
#[pyclass(
    dict,
    skip_from_py_object,
    module = "raygeo.sketch.snap.producers",
    name = "IntersectionsProducer"
)]
pub struct PyIntersectionsProducer {
    include_construction: bool,
    cache_key: Option<String>,
    cached: Vec<(f64, f64)>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyIntersectionsProducer {
    #[new]
    #[pyo3(signature = (include_construction=true))]
    fn new(include_construction: bool) -> Self {
        Self {
            include_construction,
            cache_key: None,
            cached: Vec::new(),
        }
    }

    #[getter]
    fn _include_construction(&self) -> bool {
        self.include_construction
    }

    #[getter]
    fn _cache_key(&self) -> Option<String> {
        self.cache_key.clone()
    }

    #[getter]
    fn _cached_intersections(&self) -> Vec<(f64, f64)> {
        self.cached.clone()
    }

    fn _get_all_intersections(
        &self,
        registry: &Bound<'_, PyEntityRegistry>,
        drag_context: &PyDragContext,
    ) -> Vec<(f64, f64)> {
        let reg = registry.borrow();
        all_intersections(&reg.inner, drag_context, self.include_construction)
    }

    fn produce(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        _drag_position: (f64, f64),
        drag_context: &Bound<'_, PyAny>,
        _threshold: f64,
    ) -> PyResult<Vec<Py<PySnapLine>>> {
        let intersections = get_intersections(slf, registry, drag_context)?;
        let mut lines = Vec::new();
        for (ix, iy) in intersections {
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: false,
                    coordinate: ix,
                    line_type: PySnapLineType(
                        PySnapLineType::INTERSECTION_VALUE,
                    ),
                    source: None,
                },
            )?);
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: true,
                    coordinate: iy,
                    line_type: PySnapLineType(
                        PySnapLineType::INTERSECTION_VALUE,
                    ),
                    source: None,
                },
            )?);
        }
        Ok(lines)
    }

    fn produce_points(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        drag_position: (f64, f64),
        drag_context: &Bound<'_, PyAny>,
        threshold: f64,
    ) -> PyResult<Vec<Py<PySnapPoint>>> {
        let (x, y) = drag_position;
        let intersections = get_intersections(slf, registry, drag_context)?;
        let mut points = Vec::new();
        for (ix, iy) in intersections {
            let dist = ((ix - x).powi(2) + (iy - y).powi(2)).sqrt();
            if dist <= threshold {
                points.push(Py::new(
                    py,
                    PySnapPoint {
                        x: ix,
                        y: iy,
                        line_type: PySnapLineType(
                            PySnapLineType::INTERSECTION_VALUE,
                        ),
                        source: None,
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

fn get_intersections(
    slf: &Bound<'_, PyIntersectionsProducer>,
    registry: &Bound<'_, PyEntityRegistry>,
    context_obj: &Bound<'_, PyAny>,
) -> PyResult<Vec<(f64, f64)>> {
    let context = context_obj.extract::<PyRef<'_, PyDragContext>>()?;
    let key = {
        let this = slf.borrow();
        cache_key(registry, &context, this.include_construction)
    };
    {
        let this = slf.borrow();
        if this.cache_key.as_deref() == Some(key.as_str()) {
            return Ok(this.cached.clone());
        }
    }
    let dict = slf.getattr("__dict__")?;
    let dict = dict.cast::<pyo3::types::PyDict>()?;
    let computed: Vec<(f64, f64)> = match dict
        .get_item("_get_all_intersections")?
    {
        Some(callback) => callback.call1((registry, context_obj))?.extract()?,
        None => {
            let this = slf.borrow();
            let reg = registry.borrow();
            all_intersections(&reg.inner, &context, this.include_construction)
        }
    };
    let mut this = slf.borrow_mut();
    this.cached = computed.clone();
    this.cache_key = Some(key);
    Ok(computed)
}

fn cache_key(
    registry: &Bound<'_, PyEntityRegistry>,
    context: &PyDragContext,
    include_construction: bool,
) -> String {
    let reg = registry.borrow();
    let mut excluded_entities = context.entity_ids();
    excluded_entities.extend(context.preview_ids());
    let mut excluded_points = context.point_ids();
    for id in &excluded_entities {
        if let Some(ids) = reg.inner.entity_point_ids(*id) {
            excluded_points.extend(ids);
        }
    }
    let mut parts = Vec::new();
    for id in reg.inner.entity_ids() {
        if excluded_entities.contains(&id) {
            continue;
        }
        let Some(entity) = reg.inner.get_entity(id) else {
            continue;
        };
        if entity
            .get_point_ids()
            .iter()
            .any(|p| excluded_points.contains(p))
        {
            continue;
        }
        if !include_construction && entity.base().construction {
            continue;
        }
        parts.push(format!(
            "{id}:{}:{}",
            entity.type_name(),
            entity.geometry_signature(&reg.inner)
        ));
    }
    parts.join("|")
}

fn all_intersections(
    registry: &EntityRegistry,
    context: &PyDragContext,
    include_construction: bool,
) -> Vec<(f64, f64)> {
    let mut entities = Vec::new();
    for id in registry.entity_ids() {
        if context.is_entity_dragged(id) || context.is_preview_entity(id) {
            continue;
        }
        let Some(entity) = registry.get_entity(id) else {
            continue;
        };
        if entity
            .get_point_ids()
            .iter()
            .any(|pid| context.is_point_dragged(*pid))
        {
            continue;
        }
        if !include_construction && entity.base().construction {
            continue;
        }
        entities.push(entity);
    }
    let mut out = Vec::new();
    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            out.extend(pair_intersections(entities[i], entities[j], registry));
        }
    }
    out
}

fn pair_intersections(
    e1: &dyn Entity,
    e2: &dyn Entity,
    registry: &EntityRegistry,
) -> Vec<(f64, f64)> {
    let (l1, l2) = (
        e1.as_any().downcast_ref::<Line>(),
        e2.as_any().downcast_ref::<Line>(),
    );
    if let (Some(l1), Some(l2)) = (l1, l2) {
        return line_line(l1, l2, registry);
    }
    let (l1, a2) = (
        e1.as_any().downcast_ref::<Line>(),
        e2.as_any().downcast_ref::<Arc>(),
    );
    if let (Some(l1), Some(a2)) = (l1, a2) {
        return line_curve(l1, Curve::Arc(a2), registry);
    }
    let (a1, l2) = (
        e1.as_any().downcast_ref::<Arc>(),
        e2.as_any().downcast_ref::<Line>(),
    );
    if let (Some(a1), Some(l2)) = (a1, l2) {
        return line_curve(l2, Curve::Arc(a1), registry);
    }
    let (l1, c2) = (
        e1.as_any().downcast_ref::<Line>(),
        e2.as_any().downcast_ref::<Circle>(),
    );
    if let (Some(l1), Some(c2)) = (l1, c2) {
        return line_curve(l1, Curve::Circle(c2), registry);
    }
    let (c1, l2) = (
        e1.as_any().downcast_ref::<Circle>(),
        e2.as_any().downcast_ref::<Line>(),
    );
    if let (Some(c1), Some(l2)) = (c1, l2) {
        return line_curve(l2, Curve::Circle(c1), registry);
    }
    let (a1, a2) = (
        e1.as_any().downcast_ref::<Arc>(),
        e2.as_any().downcast_ref::<Arc>(),
    );
    if let (Some(a1), Some(a2)) = (a1, a2) {
        return arc_arc(a1, a2, registry);
    }
    let (c1, c2) = (
        e1.as_any().downcast_ref::<Circle>(),
        e2.as_any().downcast_ref::<Circle>(),
    );
    if let (Some(c1), Some(c2)) = (c1, c2) {
        return circle_circle(c1, c2, registry);
    }
    Vec::new()
}

fn line_line(
    l1: &Line,
    l2: &Line,
    registry: &EntityRegistry,
) -> Vec<(f64, f64)> {
    let (Some(p1), Some(p2), Some(p3), Some(p4)) = (
        registry.get_point(l1.p1),
        registry.get_point(l1.p2),
        registry.get_point(l2.p1),
        registry.get_point(l2.p2),
    ) else {
        return Vec::new();
    };
    get_line_segment_intersection(
        Point::new(p1.x, p1.y),
        Point::new(p2.x, p2.y),
        Point::new(p3.x, p3.y),
        Point::new(p4.x, p4.y),
    )
    .map(|p| vec![(p.x, p.y)])
    .unwrap_or_default()
}

enum Curve<'a> {
    Arc(&'a Arc),
    Circle(&'a Circle),
}

fn line_curve(
    line: &Line,
    curve: Curve<'_>,
    registry: &EntityRegistry,
) -> Vec<(f64, f64)> {
    let (Some(p1), Some(p2)) =
        (registry.get_point(line.p1), registry.get_point(line.p2))
    else {
        return Vec::new();
    };
    let (center, radius) = match curve {
        Curve::Arc(arc) => {
            let (Some(center), Some(start)) = (
                registry.get_point(arc.center),
                registry.get_point(arc.start),
            ) else {
                return Vec::new();
            };
            let radius = (start.x - center.x).hypot(start.y - center.y);
            (center, radius)
        }
        Curve::Circle(circle) => {
            let (Some(center), Some(radius_point)) = (
                registry.get_point(circle.center),
                registry.get_point(circle.radius_point),
            ) else {
                return Vec::new();
            };
            let radius =
                (radius_point.x - center.x).hypot(radius_point.y - center.y);
            (center, radius)
        }
    };
    if radius < 1e-10 {
        return Vec::new();
    }
    get_line_circle_intersections(
        Point::new(p1.x, p1.y),
        Point::new(p2.x, p2.y),
        Point::new(center.x, center.y),
        radius,
    )
    .into_iter()
    .filter(|p| match curve {
        Curve::Arc(arc) => {
            let angle = (p.y - center.y).atan2(p.x - center.x);
            arc.is_angle_within_sweep(angle, registry)
        }
        Curve::Circle(_) => true,
    })
    .map(|p| (p.x, p.y))
    .collect()
}

fn arc_arc(a1: &Arc, a2: &Arc, registry: &EntityRegistry) -> Vec<(f64, f64)> {
    let (Some(c1), Some(s1), Some(c2), Some(s2)) = (
        registry.get_point(a1.center),
        registry.get_point(a1.start),
        registry.get_point(a2.center),
        registry.get_point(a2.start),
    ) else {
        return Vec::new();
    };
    let r1 = (s1.x - c1.x).hypot(s1.y - c1.y);
    let r2 = (s2.x - c2.x).hypot(s2.y - c2.y);
    get_circle_circle_intersections(
        Point::new(c1.x, c1.y),
        r1,
        Point::new(c2.x, c2.y),
        r2,
    )
    .into_iter()
    .filter(|p| {
        let a1_angle = (p.y - c1.y).atan2(p.x - c1.x);
        let a2_angle = (p.y - c2.y).atan2(p.x - c2.x);
        a1.is_angle_within_sweep(a1_angle, registry)
            && a2.is_angle_within_sweep(a2_angle, registry)
    })
    .map(|p| (p.x, p.y))
    .collect()
}

fn circle_circle(
    c1: &Circle,
    c2: &Circle,
    registry: &EntityRegistry,
) -> Vec<(f64, f64)> {
    let (Some(center1), Some(r1p), Some(center2), Some(r2p)) = (
        registry.get_point(c1.center),
        registry.get_point(c1.radius_point),
        registry.get_point(c2.center),
        registry.get_point(c2.radius_point),
    ) else {
        return Vec::new();
    };
    let r1 = (r1p.x - center1.x).hypot(r1p.y - center1.y);
    let r2 = (r2p.x - center2.x).hypot(r2p.y - center2.y);
    get_circle_circle_intersections(
        Point::new(center1.x, center1.y),
        r1,
        Point::new(center2.x, center2.y),
        r2,
    )
    .into_iter()
    .map(|p| (p.x, p.y))
    .collect()
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyIntersectionsProducer>()?;
    Ok(())
}
