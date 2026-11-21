//! Offset planning for sketch entities: analytic plans for the
//! radius-bearing kinds, geometry-offsetter plans for outlines, and
//! the offset-state transforms applied to array copies.
//!
//! Plans are pure value descriptions ([`OffsetPlan`]); the host
//! materializes them into its own document objects.  Placement
//! transforms arrive through the structural
//! [`PlacementTransform`](crate::sketch::transform::PlacementTransform)
//! trait, mirroring how the solver reads coordinates through
//! `PointContext`.

use serde_json::Value;

use crate::geo::algo::offset::grow_geometry;
use crate::geo::shape::polygon::get_circle_polygon;
use crate::geo::types::Point;
use crate::sketch::constraint::PointContext;
use crate::sketch::entities::arc::Arc;
use crate::sketch::entities::bezier::Bezier;
use crate::sketch::entities::circle::Circle;
use crate::sketch::entities::ellipse::Ellipse;
use crate::sketch::entities::polygon::PolygonEntity;
use crate::sketch::entities::text_box::TextBoxEntity;
use crate::sketch::transform::{flips_chirality, PlacementTransform};

const MIN_OFFSET_RADIUS: f64 = 1e-6;

type Ring = Vec<(f64, f64)>;
pub(crate) type Solid = (Ring, Vec<Ring>);

/// Description of the changes one offset operation makes.
#[derive(Clone, Debug, Default)]
pub struct OffsetPlan {
    /// Point id → its new position.
    pub point_moves: Vec<(u64, (f64, f64))>,
    /// New points the plan introduces.
    pub points: Vec<PlannedPoint>,
    /// New entities the plan introduces, in the reference dictionary
    /// shape (`entity_from_json` consumes these).
    pub entities: Vec<Value>,
    /// Entities the plan removes.
    pub removed_entity_ids: Vec<u64>,
}

impl OffsetPlan {
    fn with_moves(moves: Vec<(u64, (f64, f64))>) -> Self {
        OffsetPlan {
            point_moves: moves,
            ..OffsetPlan::default()
        }
    }
}

/// A new point an offset plan introduces.
#[derive(Clone, Copy, Debug)]
pub struct PlannedPoint {
    pub id: u64,
    pub x: f64,
    pub y: f64,
}

/// A sampled outline produced from a connected chain of entities.
#[derive(Clone, Debug, Default)]
pub struct Outline {
    pub vertices: Vec<(f64, f64)>,
    pub closed: bool,
    pub source_ids: Vec<u64>,
}

fn min_radius_ok(radius: f64) -> bool {
    radius > MIN_OFFSET_RADIUS
}

/// Plan a circle offset: the radius point moves radially.
pub fn circle_plan(
    circle: &Circle,
    model: &dyn PointContext,
    offset: f64,
) -> Option<OffsetPlan> {
    let center = model.point_position(circle.center)?;
    let radius_pt = model.point_position(circle.radius_point)?;
    let dx = radius_pt.0 - center.0;
    let dy = radius_pt.1 - center.1;
    let radius = dx.hypot(dy);
    let new_radius = radius + offset;
    if !min_radius_ok(new_radius) {
        return None;
    }
    let (ux, uy) = unit_or(radius, (dx, dy));
    let moved = (center.0 + ux * new_radius, center.1 + uy * new_radius);
    Some(OffsetPlan::with_moves(vec![(circle.radius_point, moved)]))
}

/// Preview a circle offset as a concentric ring.
pub fn circle_preview(
    circle: &Circle,
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
    arc: &Arc,
    model: &dyn PointContext,
    offset: f64,
) -> Option<OffsetPlan> {
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
        let (ux, uy) = unit_or(dx.hypot(dy), (dx, dy));
        (center.0 + ux * new_radius, center.1 + uy * new_radius)
    };
    Some(OffsetPlan::with_moves(vec![
        (arc.start, moved(start)),
        (arc.end, moved(end)),
    ]))
}

/// Preview an arc offset as a concentric arc.
pub fn arc_preview(
    arc: &Arc,
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
    ellipse: &Ellipse,
    model: &dyn PointContext,
    offset: f64,
) -> Option<OffsetPlan> {
    let center = model.point_position(ellipse.center)?;
    let rx_pt = model.point_position(ellipse.radius_x_point)?;
    let ry_pt = model.point_position(ellipse.radius_y_point)?;
    let rx = (rx_pt.0 - center.0).hypot(rx_pt.1 - center.1);
    let ry = (ry_pt.0 - center.0).hypot(ry_pt.1 - center.1);
    if !min_radius_ok(rx + offset) || !min_radius_ok(ry + offset) {
        return None;
    }
    let moved = |pt: (f64, f64), radius: f64| {
        let scale = (radius + offset) / radius;
        (
            center.0 + (pt.0 - center.0) * scale,
            center.1 + (pt.1 - center.1) * scale,
        )
    };
    Some(OffsetPlan::with_moves(vec![
        (ellipse.radius_x_point, moved(rx_pt, rx)),
        (ellipse.radius_y_point, moved(ry_pt, ry)),
    ]))
}

/// Preview an ellipse offset as a parametric sampling.
pub fn ellipse_preview(
    ellipse: &Ellipse,
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
    if !min_radius_ok(rx + offset) || !min_radius_ok(ry + offset) {
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

/// Unit vector of `(dx, dy)`, `(1, 0)` for a degenerate segment.
fn unit_or(length: f64, (dx, dy): (f64, f64)) -> (f64, f64) {
    if length < 1e-9 {
        (1.0, 0.0)
    } else {
        (dx / length, dy / length)
    }
}

// ── outline plans ──────────────────────────────────────────────────

/// The `(center, handle, entity-dict)` triple one outline contributes
/// to a plan.  The ids come from the caller's allocator.
pub fn outline_plan_item(
    allocate_id: &mut dyn FnMut() -> u64,
    vertices: &[(f64, f64)],
    closed: bool,
    holes: &[Vec<(f64, f64)>],
) -> Option<(PlannedPoint, PlannedPoint, Value)> {
    use crate::sketch::entities::polygon::PolygonEntity;

    let (center, handle) =
        crate::sketch::contour::frame_for_outline(vertices, closed);
    let center_id = allocate_id();
    let handle_id = allocate_id();
    let entity_id = allocate_id();
    let local =
        crate::sketch::contour::normalize_vertices(center, handle, vertices);
    let local_holes: Vec<Vec<(f64, f64)>> = holes
        .iter()
        .map(|hole| {
            crate::sketch::contour::normalize_vertices(center, handle, hole)
        })
        .collect();

    let entity = PolygonEntity::new(
        entity_id,
        center_id,
        handle_id,
        local,
        closed,
        false,
        local_holes,
    );
    Some((
        PlannedPoint {
            id: center_id,
            x: center.0,
            y: center.1,
        },
        PlannedPoint {
            id: handle_id,
            x: handle.0,
            y: handle.1,
        },
        entity.to_json(),
    ))
}

/// Plans the replacement geometry for an outline: every offset ring
/// becomes a frame-local polygon entity with two new frame points.
pub fn plan_of_outline(
    outline: &Outline,
    offset: f64,
    allocate_id: &mut dyn FnMut() -> u64,
) -> Option<OffsetPlan> {
    let mut plan = OffsetPlan::default();
    for (vertices, closed) in crate::sketch::contour::offset_outline(
        &outline.vertices,
        outline.closed,
        offset,
        0.02,
    ) {
        let (center, handle, entity) =
            outline_plan_item(allocate_id, &vertices, closed, &[])?;
        plan.points.push(center);
        plan.points.push(handle);
        plan.entities.push(entity);
    }
    if plan.entities.is_empty() {
        return None;
    }
    plan.removed_entity_ids = outline.source_ids.clone();
    Some(plan)
}

/// Coarse polylines of an outline's offset for live preview.
pub fn preview_of_outline(outline: &Outline, offset: f64) -> Vec<Ring> {
    crate::sketch::contour::offset_outline(
        &outline.vertices,
        outline.closed,
        offset,
        0.02,
    )
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

fn polygon_outline(
    polygon: &PolygonEntity,
    model: &dyn PointContext,
) -> Outline {
    Outline {
        vertices: polygon.get_world_vertices(model),
        closed: polygon.closed,
        source_ids: vec![polygon.base.id],
    }
}

/// The outline an open polygon forms on its own; `None` for closed
/// polygons (they offset through [`polygon_plan`]).
pub fn outline_of_open_polygon(
    polygon: &PolygonEntity,
    model: &dyn PointContext,
) -> Option<Outline> {
    if polygon.closed {
        return None;
    }
    Some(polygon_outline(polygon, model))
}

fn polygon_solids(
    polygon: &PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Solid> {
    let rings = polygon.get_world_rings(model);
    let geo = crate::sketch::contour::rings_to_geometry(&rings);
    let grown = grow_geometry(&geo, offset);
    let rings: Vec<Vec<(f64, f64)>> =
        grown.to_polygons(0.02).iter().map(to_tuples).collect();
    crate::sketch::contour::group_rings_into_solids(&rings)
}

/// Plans a polygon offset.  Open polygons become outlines; closed
/// polygons grow/shrink into one frame-local polygon per solid.
pub fn polygon_plan(
    polygon: &PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
    allocate_id: &mut dyn FnMut() -> u64,
) -> Option<OffsetPlan> {
    if !polygon.closed {
        return plan_of_outline(
            &polygon_outline(polygon, model),
            offset,
            allocate_id,
        );
    }
    let solids = polygon_solids(polygon, model, offset);
    if solids.is_empty() {
        return None;
    }
    let mut plan = OffsetPlan::default();
    for (outer, holes) in solids {
        let (center, handle, entity) =
            outline_plan_item(allocate_id, &outer, true, &holes)?;
        plan.points.push(center);
        plan.points.push(handle);
        plan.entities.push(entity);
    }
    plan.removed_entity_ids.push(polygon.base.id);
    Some(plan)
}

/// Coarse polylines of a polygon's offset for live preview.
pub fn polygon_preview(
    polygon: &PolygonEntity,
    model: &dyn PointContext,
    offset: f64,
) -> Vec<Ring> {
    if !polygon.closed {
        return preview_of_outline(&polygon_outline(polygon, model), offset);
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

// ── placement / copy offset state ──────────────────────────────────

/// Transforms a bezier's control-point offsets by the placement.
pub fn bezier_transform_offsets(
    bezier: &mut Bezier,
    placement: &dyn PlacementTransform,
) {
    if let Some(cp) = bezier.cp1 {
        let (x, y) = placement.transform_offset(cp[0], cp[1]);
        bezier.cp1 = Some([x, y]);
    }
    if let Some(cp) = bezier.cp2 {
        let (x, y) = placement.transform_offset(cp[0], cp[1]);
        bezier.cp2 = Some([x, y]);
    }
}

/// Re-derives a bezier's copy state from a template through the
/// placement.
pub fn bezier_rewrite_offsets_from(
    bezier: &mut Bezier,
    template: &Bezier,
    placement: &dyn PlacementTransform,
) {
    if let Some(cp) = template.cp1 {
        let (x, y) = placement.transform_offset(cp[0], cp[1]);
        bezier.cp1 = Some([x, y]);
    }
    if let Some(cp) = template.cp2 {
        let (x, y) = placement.transform_offset(cp[0], cp[1]);
        bezier.cp2 = Some([x, y]);
    }
}

/// Transforms a polygon's offset state for a placement: mirrored
/// placements flip the local rings.
pub fn polygon_transform_offsets(
    polygon: &mut PolygonEntity,
    placement: &dyn PlacementTransform,
) {
    if flips_chirality(placement) {
        for ring in &mut polygon.rings {
            for vertex in ring {
                vertex.1 = -vertex.1;
            }
        }
        polygon.normalize_ring_winding();
    }
}

/// Re-derives a polygon's copy state from a template through the
/// placement.
pub fn polygon_rewrite_offsets_from(
    polygon: &mut PolygonEntity,
    template: &PolygonEntity,
    placement: &dyn PlacementTransform,
) {
    let flips = flips_chirality(placement);
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
}

/// Re-derives a text box's copy state from a template.
pub fn text_box_rewrite_offsets_from(
    text_box: &mut TextBoxEntity,
    template: &TextBoxEntity,
) {
    text_box.content = template.content.clone();
    text_box.font_config = template.font_config.clone();
    text_box.font_extra = template.font_extra.clone();
    text_box.base.fill_color = template.base.fill_color;
}

fn to_tuples(poly: &crate::geo::types::Polygon) -> Vec<(f64, f64)> {
    poly.iter().map(|p| (p.x, p.y)).collect()
}
