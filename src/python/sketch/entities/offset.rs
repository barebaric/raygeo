//! Offset planning for sketch entities.
//!
//! The reference sketcher keeps offset planning on the entity classes:
//! an entity either offsets in place (circle, arc, ellipse, closed
//! polygon) or is sampled into an outline that is replaced by new
//! polygon entities.  These bindings implement that contract in Rust.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::shape::polygon::get_circle_polygon;
use crate::geo::types::Point;
use crate::sketch::constraint::PointContext;

const MIN_OFFSET_RADIUS: f64 = 1e-6;

type Ring = Vec<(f64, f64)>;
type Solid = (Ring, Vec<Ring>);
type EdgeMidpoints = (f64, (f64, f64), (f64, f64));

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
    fn with_moves(
        py: Python<'_>,
        moves: Vec<(u64, (f64, f64))>,
    ) -> PyResult<Self> {
        let plan = Self::new(py);
        for (id, (x, y)) in moves {
            plan.point_moves.bind(py).set_item(id, (x, y))?;
        }
        Ok(plan)
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
}

fn min_radius_ok(radius: f64) -> bool {
    radius > MIN_OFFSET_RADIUS
}

/// Plan a circle offset: the radius point moves radially.
pub fn circle_plan(
    py: Python<'_>,
    circle: &crate::sketch::entities::circle::Circle,
    model: &dyn PointContext,
    offset: f64,
) -> Option<PyOffsetPlan> {
    let center = model.point_position(circle.center)?;
    let radius_pt = model.point_position(circle.radius_point)?;
    let dx = radius_pt.0 - center.0;
    let dy = radius_pt.1 - center.1;
    let radius = dx.hypot(dy);
    let new_radius = radius + offset;
    if !min_radius_ok(new_radius) {
        return None;
    }
    let (ux, uy) = if radius < 1e-9 {
        (1.0, 0.0)
    } else {
        (dx / radius, dy / radius)
    };
    let moved = (center.0 + ux * new_radius, center.1 + uy * new_radius);
    PyOffsetPlan::with_moves(py, vec![(circle.radius_point, moved)]).ok()
}

/// Preview a circle offset as a concentric ring.
pub fn circle_preview(
    circle: &crate::sketch::entities::circle::Circle,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    let (Some(center), Some(radius_pt)) = (
        model.point_position(circle.center),
        model.point_position(circle.radius_point),
    ) else {
        return Vec::new();
    };
    let radius = (radius_pt.0 - center.0).hypot(radius_pt.1 - center.1);
    if !min_radius_ok(radius + offset) {
        return Vec::new();
    }
    let ring =
        get_circle_polygon(Point::new(center.0, center.1), radius + offset, 64);
    let mut closed: Vec<(f64, f64)> =
        ring.into_iter().map(|p| (p.x, p.y)).collect();
    if let Some(first) = closed.first().copied() {
        closed.push(first);
    }
    vec![closed]
}

/// Plan an arc offset: both endpoints move radially onto the offset
/// ring while the center stays fixed.
pub fn arc_plan(
    py: Python<'_>,
    arc: &crate::sketch::entities::arc::Arc,
    model: &dyn PointContext,
    offset: f64,
) -> Option<PyOffsetPlan> {
    let center = model.point_position(arc.center)?;
    let start = model.point_position(arc.start)?;
    let end = model.point_position(arc.end)?;
    let radius = (start.0 - center.0).hypot(start.1 - center.1);
    let new_radius = radius + offset;
    if !min_radius_ok(new_radius) {
        return None;
    }
    let moved = |pt: (f64, f64)| {
        let dx = pt.0 - center.0;
        let dy = pt.1 - center.1;
        let length = dx.hypot(dy);
        let (ux, uy) = if length < 1e-9 {
            (1.0, 0.0)
        } else {
            (dx / length, dy / length)
        };
        (center.0 + ux * new_radius, center.1 + uy * new_radius)
    };
    PyOffsetPlan::with_moves(
        py,
        vec![(arc.start, moved(start)), (arc.end, moved(end))],
    )
    .ok()
}

/// Preview an arc offset as a concentric arc.
pub fn arc_preview(
    arc: &crate::sketch::entities::arc::Arc,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    let (Some(center), Some(start), Some(end)) = (
        model.point_position(arc.center),
        model.point_position(arc.start),
        model.point_position(arc.end),
    ) else {
        return Vec::new();
    };
    let radius = (start.0 - center.0).hypot(start.1 - center.1) + offset;
    if !min_radius_ok(radius) {
        return Vec::new();
    }
    let start_angle = (start.1 - center.1).atan2(start.0 - center.0);
    let end_angle = (end.1 - center.1).atan2(end.0 - center.0);
    let mut sweep = end_angle - start_angle;
    if arc.clockwise && sweep > 0.0 {
        sweep -= 2.0 * std::f64::consts::PI;
    } else if !arc.clockwise && sweep < 0.0 {
        sweep += 2.0 * std::f64::consts::PI;
    }
    let segments =
        (sweep.abs() / std::f64::consts::PI * 32.0).max(8.0) as usize;
    let points = (0..=segments)
        .map(|i| {
            let angle = start_angle + sweep * i as f64 / segments as f64;
            (
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            )
        })
        .collect();
    vec![points]
}

/// Plan an ellipse offset: both radius points move radially.
pub fn ellipse_plan(
    py: Python<'_>,
    ellipse: &crate::sketch::entities::ellipse::Ellipse,
    model: &dyn PointContext,
    offset: f64,
) -> Option<PyOffsetPlan> {
    let center = model.point_position(ellipse.center)?;
    let rx_pt = model.point_position(ellipse.radius_x_point)?;
    let ry_pt = model.point_position(ellipse.radius_y_point)?;
    let rx = (rx_pt.0 - center.0).hypot(rx_pt.1 - center.1);
    let ry = (ry_pt.0 - center.0).hypot(ry_pt.1 - center.1);
    if rx + offset <= MIN_OFFSET_RADIUS || ry + offset <= MIN_OFFSET_RADIUS {
        return None;
    }
    let moved = |pt: (f64, f64), radius: f64| {
        let scale = (radius + offset) / radius;
        (
            center.0 + (pt.0 - center.0) * scale,
            center.1 + (pt.1 - center.1) * scale,
        )
    };
    PyOffsetPlan::with_moves(
        py,
        vec![
            (ellipse.radius_x_point, moved(rx_pt, rx)),
            (ellipse.radius_y_point, moved(ry_pt, ry)),
        ],
    )
    .ok()
}

/// Preview an ellipse offset as a parametric sampling.
pub fn ellipse_preview(
    ellipse: &crate::sketch::entities::ellipse::Ellipse,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    let (Some(center), Some(rx_pt), Some(ry_pt)) = (
        model.point_position(ellipse.center),
        model.point_position(ellipse.radius_x_point),
        model.point_position(ellipse.radius_y_point),
    ) else {
        return Vec::new();
    };
    let rx = (rx_pt.0 - center.0).hypot(rx_pt.1 - center.1);
    let ry = (ry_pt.0 - center.0).hypot(ry_pt.1 - center.1);
    if rx + offset <= MIN_OFFSET_RADIUS || ry + offset <= MIN_OFFSET_RADIUS {
        return Vec::new();
    }
    let rotation = (rx_pt.1 - center.1).atan2(rx_pt.0 - center.0);
    let cos_a = rotation.cos();
    let sin_a = rotation.sin();
    let segments = 64usize;
    let points = (0..=segments)
        .map(|i| {
            let t = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
            (
                center.0 + (rx + offset) * t.cos() * cos_a
                    - (ry + offset) * t.sin() * sin_a,
                center.1
                    + (rx + offset) * t.cos() * sin_a
                    + (ry + offset) * t.sin() * cos_a,
            )
        })
        .collect();
    vec![points]
}

#[allow(dead_code)]
fn _unused(_: &Bound<'_, PyAny>) -> PyResult<()> {
    Err(PyValueError::new_err("unused"))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyOffsetPlan>()?;
    m.add_class::<PyPolygonOutline>()?;
    Ok(())
}

// ── polygon outline helpers ────────────────────────────────────────

use crate::geo::algo::offset::grow_geometry;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::topology::{
    normalize_winding_orders, split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_closest_point, get_polygon_convex_hull,
    offset_polyline, EndStyle, JoinStyle,
};
use crate::geo::types::Polygon;

fn to_polygon(vertices: &[(f64, f64)]) -> Polygon {
    vertices.iter().map(|(x, y)| Point::new(*x, *y)).collect()
}

fn to_tuples(poly: &Polygon) -> Vec<(f64, f64)> {
    poly.iter().map(|p| (p.x, p.y)).collect()
}

/// Offsets a single connected outline. Closed outlines grow/shrink
/// through the geometry offsetter; open outlines become a closed
/// slot-like outline with round caps.
pub fn offset_outline(
    vertices: &[(f64, f64)],
    closed: bool,
    distance: f64,
    tolerance: f64,
) -> Vec<(Vec<(f64, f64)>, bool)> {
    if vertices.len() < 2 || distance.abs() < 1e-9 {
        return Vec::new();
    }
    if closed {
        let mut geo = Geometry::new();
        geo.move_to(vertices[0].0, vertices[0].1, 0.0);
        for (x, y) in &vertices[1..] {
            geo.line_to(*x, *y, 0.0);
        }
        geo.close_path();
        let grown = grow_geometry(&geo, distance);
        return grown
            .to_polygons(tolerance)
            .into_iter()
            .map(|p| (to_tuples(&p), true))
            .collect();
    }
    offset_polyline(
        &to_polygon(vertices),
        distance,
        JoinStyle::Miter,
        EndStyle::Round,
    )
    .into_iter()
    .map(|p| (to_tuples(&p), true))
    .collect()
}

/// Builds a multi-ring closed geometry from a flat ring list.
pub fn rings_to_geometry(rings: &[Vec<(f64, f64)>]) -> Geometry {
    let mut geo = Geometry::new();
    for ring in rings {
        if ring.len() < 3 {
            continue;
        }
        geo.move_to(ring[0].0, ring[0].1, 0.0);
        for (x, y) in &ring[1..] {
            geo.line_to(*x, *y, 0.0);
        }
        geo.close_path();
    }
    geo
}

/// Groups a flat ring list into (outer, holes) solids.
pub fn group_rings_into_solids(rings: &[Vec<(f64, f64)>]) -> Vec<Solid> {
    let geo = rings_to_geometry(rings);
    if geo.is_empty() {
        return Vec::new();
    }
    let contours = split_into_contours(&geo);
    let refs: Vec<&Geometry> = contours.iter().collect();
    let normalized = normalize_winding_orders(&refs);
    let mut merged = Geometry::new();
    for contour in &normalized {
        merged.extend(contour);
    }
    let (outers, holes) = merged.split_inner_and_outer_polygons();
    let mut solids: Vec<Solid> = outers
        .iter()
        .map(|outer| (to_tuples(outer), Vec::new()))
        .collect();
    solids.sort_by(|a, b| {
        get_polygon_area(&to_polygon(&a.0))
            .partial_cmp(&get_polygon_area(&to_polygon(&b.0)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for hole in holes {
        let hole_tuples = to_tuples(&hole);
        if hole_tuples.is_empty() {
            continue;
        }
        for (outer, assigned) in &mut solids {
            if crate::geo::shape::polygon::is_point_inside_polygon(
                Point::new(hole_tuples[0].0, hole_tuples[0].1),
                &to_polygon(outer),
            ) {
                assigned.push(hole_tuples);
                break;
            }
        }
    }
    solids
}

fn short_edge_midpoints(vertices: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let hull = get_polygon_convex_hull(&to_polygon(vertices));
    if hull.len() < 2 {
        return Vec::new();
    }
    let hull = to_tuples(&hull);
    let mut best: Option<EdgeMidpoints> = None;
    for i in 0..hull.len() {
        let p1 = hull[i];
        let p2 = hull[(i + 1) % hull.len()];
        let (ex, ey) = (p2.0 - p1.0, p2.1 - p1.1);
        let edge_len = ex.hypot(ey);
        if edge_len < 1e-12 {
            continue;
        }
        let (ux, uy) = (ex / edge_len, ey / edge_len);
        let mut us = Vec::new();
        let mut vs = Vec::new();
        for (px, py) in &hull {
            let (dx, dy) = (px - p1.0, py - p1.1);
            us.push(dx * ux + dy * uy);
            vs.push(-dx * uy + dy * ux);
        }
        let w = us.iter().cloned().fold(f64::MIN, f64::max)
            - us.iter().cloned().fold(f64::MAX, f64::min);
        let h = vs.iter().cloned().fold(f64::MIN, f64::max)
            - vs.iter().cloned().fold(f64::MAX, f64::min);
        let area = w * h;
        if let Some((best_area, _, _)) = best {
            if area >= best_area {
                continue;
            }
        }
        let u_mid = (us.iter().cloned().fold(f64::MIN, f64::max)
            + us.iter().cloned().fold(f64::MAX, f64::min))
            / 2.0;
        let v_mid = (vs.iter().cloned().fold(f64::MIN, f64::max)
            + vs.iter().cloned().fold(f64::MAX, f64::min))
            / 2.0;
        let center = (
            p1.0 + u_mid * ux - v_mid * uy,
            p1.1 + u_mid * uy + v_mid * ux,
        );
        let (long_dir, long_half) = if w >= h {
            ((ux, uy), w / 2.0)
        } else {
            ((-uy, ux), h / 2.0)
        };
        let end1 = (
            center.0 + long_dir.0 * long_half,
            center.1 + long_dir.1 * long_half,
        );
        let end2 = (
            center.0 - long_dir.0 * long_half,
            center.1 - long_dir.1 * long_half,
        );
        best = Some((area, end1, end2));
    }
    match best {
        Some((_, a, b)) => vec![a, b],
        None => Vec::new(),
    }
}

fn closest_outline_point(
    vertices: &[(f64, f64)],
    closed: bool,
    x: f64,
    y: f64,
) -> Option<((f64, f64), f64)> {
    if closed {
        let result = get_polygon_closest_point(&to_polygon(vertices), x, y)?;
        let (_t, point, dist_sq) = result;
        return Some(((point.x, point.y), dist_sq));
    }
    let mut best: Option<((f64, f64), f64)> = None;
    for pair in vertices.windows(2) {
        let (_, point, dist) =
            crate::geo::shape::line::get_line_segment_closest_point(
                Point::new(pair[0].0, pair[0].1),
                Point::new(pair[1].0, pair[1].1),
                x,
                y,
            );
        let dist_sq = dist * dist;
        if best.is_none() || dist_sq < best.unwrap().1 {
            best = Some(((point.x, point.y), dist_sq));
        }
    }
    best
}

fn frame_for_outline(
    vertices: &[(f64, f64)],
    closed: bool,
) -> ((f64, f64), (f64, f64)) {
    let xs: Vec<f64> = vertices.iter().map(|v| v.0).collect();
    let ys: Vec<f64> = vertices.iter().map(|v| v.1).collect();
    let (min_x, max_x) = (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
    );
    let (min_y, max_y) = (
        ys.iter().cloned().fold(f64::MAX, f64::min),
        ys.iter().cloned().fold(f64::MIN, f64::max),
    );
    let mut cx = (min_x + max_x) / 2.0;
    let mut cy = (min_y + max_y) / 2.0;
    if closed {
        if let Some((center, _)) =
            find_largest_circle(&to_polygon(vertices), &[], 0.5)
        {
            cx = center.x;
            cy = center.y;
        }
    }
    let mut handle: Option<(f64, f64)> = None;
    let mut best_dist_sq = 1e-12;
    for reference in short_edge_midpoints(vertices) {
        let Some((point, _)) =
            closest_outline_point(vertices, closed, reference.0, reference.1)
        else {
            continue;
        };
        let dx = point.0 - cx;
        let dy = point.1 - cy;
        let frame_dist_sq = dx * dx + dy * dy;
        if frame_dist_sq > best_dist_sq {
            best_dist_sq = frame_dist_sq;
            handle = Some(point);
        }
    }
    if let Some(handle) = handle {
        return ((cx, cy), handle);
    }
    let mut scale = (max_x - min_x) / 2.0;
    if scale <= 1e-9 {
        scale = ((max_y - min_y) / 2.0).max(1.0);
    }
    ((cx, cy), (cx + scale, cy))
}

pub(crate) fn normalize_vertices(
    center: (f64, f64),
    handle: (f64, f64),
    world_vertices: &[(f64, f64)],
) -> Vec<(f64, f64)> {
    let dx = handle.0 - center.0;
    let dy = handle.1 - center.1;
    let scale = dx.hypot(dy);
    if scale < 1e-9 {
        return Vec::new();
    }
    let (ux, uy) = (dx / scale, dy / scale);
    world_vertices
        .iter()
        .map(|(x, y)| {
            (
                ((x - center.0) * ux + (y - center.1) * uy) / scale,
                (-(x - center.0) * uy + (y - center.1) * ux) / scale,
            )
        })
        .collect()
}

// ── outline item construction ──────────────────────────────────────

pub(crate) fn outline_item<'py>(
    py: Python<'py>,
    vertices: &[(f64, f64)],
    closed: bool,
    allocate_id: &Bound<'_, PyAny>,
    holes: &[Vec<(f64, f64)>],
) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>, Bound<'py, PyAny>)> {
    use crate::sketch::entities::point::{SketchPoint, WaypointType};

    let (center, handle) = frame_for_outline(vertices, closed);
    let center_id: i64 = allocate_id.call0()?.extract()?;
    let handle_id: i64 = allocate_id.call0()?.extract()?;
    let entity_id: i64 = allocate_id.call0()?.extract()?;
    let local = normalize_vertices(center, handle, vertices);
    let local_holes: Vec<Vec<(f64, f64)>> = holes
        .iter()
        .map(|hole| normalize_vertices(center, handle, hole))
        .collect();

    let point = |id: i64, x: f64, y: f64| -> PyResult<Bound<'py, PyAny>> {
        Ok(Py::new(
            py,
            super::point::PySketchPoint {
                inner: SketchPoint {
                    id: id as u64,
                    x,
                    y,
                    fixed: false,
                    waypoint_type: WaypointType::Sharp,
                    constrained: false,
                },
                registry: None,
            },
        )?
        .into_bound(py)
        .into_any())
    };
    let center_pt = point(center_id, center.0, center.1)?;
    let handle_pt = point(handle_id, handle.0, handle.1)?;
    let entity = crate::sketch::entities::polygon::PolygonEntity::new(
        entity_id as u64,
        center_id as u64,
        handle_id as u64,
        local,
        closed,
        false,
        local_holes,
    );
    let entity_obj = super::entity_to_py(py, &entity, None)?;
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
        return Ok(polygon_as_offset_item(py, polygon, &ctx));
    }
    Ok(None)
}

impl PyPolygonOutline {
    fn plan(
        &self,
        py: Python<'_>,
        offset: f64,
        allocate_id: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyOffsetPlan>> {
        let plan = PyOffsetPlan::new(py);
        for (vertices, closed) in
            offset_outline(&self.vertices, self.closed, offset, 0.02)
        {
            let (center, handle, entity) =
                outline_item(py, &vertices, closed, allocate_id, &[])?;
            plan.points.bind(py).append(center)?;
            plan.points.bind(py).append(handle)?;
            plan.entities.bind(py).append(entity)?;
        }
        if plan.entities.bind(py).len() == 0 {
            return Ok(None);
        }
        for id in &self.source_ids {
            plan.removed_entity_ids.bind(py).append(*id)?;
        }
        Ok(Some(plan))
    }

    fn preview(&self, offset: f64) -> Vec<Vec<(f64, f64)>> {
        offset_outline(&self.vertices, self.closed, offset, 0.02)
            .into_iter()
            .map(|(mut vertices, closed)| {
                if closed && !vertices.is_empty() {
                    let first = vertices[0];
                    vertices.push(first);
                }
                vertices
            })
            .collect()
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPolygonOutline {
    /// Offsets the outline into `(vertices, closed)` pairs.
    fn offset(
        &self,
        distance: f64,
        tolerance: f64,
    ) -> Vec<(Vec<(f64, f64)>, bool)> {
        offset_outline(&self.vertices, self.closed, distance, tolerance)
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
        self.preview(offset)
    }
}

// ── polygon entity offsets ─────────────────────────────────────────

fn polygon_outline(
    polygon: &crate::sketch::entities::polygon::PolygonEntity,
    model: &dyn PointContext,
) -> PyPolygonOutline {
    PyPolygonOutline {
        vertices: polygon.get_world_vertices(model),
        closed: polygon.closed,
        source_ids: vec![polygon.base.id],
    }
}

fn polygon_solids(
    polygon: &crate::sketch::entities::polygon::PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Solid> {
    let rings = polygon.get_world_rings(model);
    let geo = rings_to_geometry(&rings);
    let grown = grow_geometry(&geo, offset);
    let rings: Vec<Vec<(f64, f64)>> =
        grown.to_polygons(0.02).iter().map(to_tuples).collect();
    group_rings_into_solids(&rings)
}

pub fn polygon_as_offset_item<'py>(
    py: Python<'py>,
    polygon: &crate::sketch::entities::polygon::PolygonEntity,
    model: &dyn PointContext,
) -> Option<Bound<'py, PyAny>> {
    if polygon.closed {
        return None;
    }
    Py::new(py, polygon_outline(polygon, model))
        .ok()
        .map(|p| p.into_bound(py).into_any())
}

pub fn polygon_plan(
    py: Python<'_>,
    polygon: &crate::sketch::entities::polygon::PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
    allocate_id: &Bound<'_, PyAny>,
) -> PyResult<Option<PyOffsetPlan>> {
    if !polygon.closed {
        return polygon_outline(polygon, model).plan(py, offset, allocate_id);
    }
    let solids = polygon_solids(polygon, model, offset);
    if solids.is_empty() {
        return Ok(None);
    }
    let plan = PyOffsetPlan::new(py);
    for (outer, holes) in solids {
        let (center, handle, entity) =
            outline_item(py, &outer, true, allocate_id, &holes)?;
        plan.points.bind(py).append(center)?;
        plan.points.bind(py).append(handle)?;
        plan.entities.bind(py).append(entity)?;
    }
    plan.removed_entity_ids.bind(py).append(polygon.base.id)?;
    Ok(Some(plan))
}

pub fn polygon_preview(
    polygon: &crate::sketch::entities::polygon::PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Vec<(f64, f64)>> {
    if !polygon.closed {
        return polygon_outline(polygon, model).preview(offset);
    }
    let mut polylines = Vec::new();
    for (outer, holes) in polygon_solids(polygon, model, offset) {
        for ring in std::iter::once(outer).chain(holes) {
            if ring.is_empty() {
                continue;
            }
            let mut closed = ring;
            closed.push(closed[0]);
            polylines.push(closed);
        }
    }
    polylines
}

// ── placement / copy offset state ─────────────────────────────────

fn placement_point(
    placement: &Bound<'_, PyAny>,
    x: f64,
    y: f64,
) -> PyResult<(f64, f64)> {
    placement
        .call_method1("transform_offset", (x, y))?
        .extract()
}

fn placement_flips_chirality(placement: &Bound<'_, PyAny>) -> PyResult<bool> {
    let ex = placement_point(placement, 1.0, 0.0)?;
    let ey = placement_point(placement, 0.0, 1.0)?;
    Ok(ex.0 * ey.1 - ex.1 * ey.0 < 0.0)
}

pub fn bezier_transform_offsets(
    bezier: &mut crate::sketch::entities::bezier::Bezier,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    if let Some(cp) = bezier.cp1 {
        let (x, y) = placement_point(placement, cp[0], cp[1])?;
        bezier.cp1 = Some([x, y]);
    }
    if let Some(cp) = bezier.cp2 {
        let (x, y) = placement_point(placement, cp[0], cp[1])?;
        bezier.cp2 = Some([x, y]);
    }
    Ok(())
}

pub fn bezier_rewrite_offsets_from(
    bezier: &mut crate::sketch::entities::bezier::Bezier,
    template: &crate::sketch::entities::bezier::Bezier,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    if let Some(cp) = template.cp1 {
        let (x, y) = placement_point(placement, cp[0], cp[1])?;
        bezier.cp1 = Some([x, y]);
    }
    if let Some(cp) = template.cp2 {
        let (x, y) = placement_point(placement, cp[0], cp[1])?;
        bezier.cp2 = Some([x, y]);
    }
    Ok(())
}

pub fn polygon_transform_offsets(
    polygon: &mut crate::sketch::entities::polygon::PolygonEntity,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    if placement_flips_chirality(placement)? {
        for ring in &mut polygon.rings {
            for vertex in ring {
                vertex.1 = -vertex.1;
            }
        }
        polygon.normalize_ring_winding();
    }
    Ok(())
}

pub fn polygon_rewrite_offsets_from(
    polygon: &mut crate::sketch::entities::polygon::PolygonEntity,
    template: &crate::sketch::entities::polygon::PolygonEntity,
    placement: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let flips = placement_flips_chirality(placement)?;
    polygon.rings = template
        .rings
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|(u, v)| (*u, if flips { -v } else { *v }))
                .collect()
        })
        .collect();
    polygon.normalize_ring_winding();
    Ok(())
}

pub fn text_box_rewrite_offsets_from(
    text_box: &mut crate::sketch::entities::text_box::TextBoxEntity,
    template: &crate::sketch::entities::text_box::TextBoxEntity,
) {
    text_box.content = template.content.clone();
    text_box.font_config = template.font_config.clone();
    text_box.font_extra = template.font_extra.clone();
    text_box.base.fill_color = template.base.fill_color;
}
