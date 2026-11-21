//! The snap producer algorithms: candidate snap lines and points
//! for the entity-point, center, midpoint, on-entity, equidistant
//! and intersection producers.
//!
//! Pure functions over the registry and a [`SnapContext`]; the
//! Python binding layer materializes these values into its snap
//! objects and fills in the host's entity objects for `source`.

use std::collections::HashSet;

use crate::geo::shape::circle::{
    get_circle_circle_intersections, get_line_circle_intersections,
};
use crate::geo::shape::line::get_line_segment_intersection;
use crate::geo::types::Point;
use crate::sketch::entities::arc::Arc;
use crate::sketch::entities::circle::Circle;
use crate::sketch::entities::entity::Entity;
use crate::sketch::entities::entity::EntityKind;
use crate::sketch::entities::line::Line;
use crate::sketch::registry::EntityRegistry;

use super::types::{SnapContext, SnapLine, SnapLineType, SnapPoint};

/// The center point of an entity, for the radius-bearing kinds.
fn center_of(
    entity: &dyn Entity,
    registry: &EntityRegistry,
) -> Option<(u64, (f64, f64))> {
    let index = entity.kind().center_id_index()?;
    let id = *entity.get_point_ids().get(index)?;
    let point = registry.get_point(id)?;
    Some((id, (point.x, point.y)))
}

/// Whether a candidate at `(x, y)` coincides with a dragged point.
fn coincides_with_drag(
    registry: &EntityRegistry,
    context: &SnapContext,
    x: f64,
    y: f64,
) -> bool {
    !context.suppress_coincident_points
        || context.dragged_point_ids.iter().any(|pid| {
            registry.get_point(*pid).is_some_and(|dragged| {
                (dragged.x - x).abs() <= 1e-6 && (dragged.y - y).abs() <= 1e-6
            })
        })
}

pub mod centers {
    //! Snap lines and points at circle, arc, and ellipse centers.

    use super::*;

    /// The `(entity_id, center)` candidates the producer offers.
    fn candidates(
        registry: &EntityRegistry,
        context: &SnapContext,
        include_construction: bool,
    ) -> Vec<(u64, (f64, f64))> {
        let mut out = Vec::new();
        for entity_id in registry.entity_ids() {
            if context.is_entity_dragged(entity_id) {
                continue;
            }
            let Some(entity) = registry.get_entity(entity_id) else {
                continue;
            };
            if !include_construction && entity.base().construction {
                continue;
            }
            let Some((center_id, center)) = center_of(entity, registry) else {
                continue;
            };
            if context.is_point_dragged(center_id) {
                continue;
            }
            if coincides_with_drag(registry, context, center.0, center.1) {
                continue;
            }
            out.push((entity_id, center));
        }
        out
    }

    pub fn produce_lines(
        registry: &EntityRegistry,
        context: &SnapContext,
        include_construction: bool,
    ) -> Vec<SnapLine> {
        let mut lines = Vec::new();
        for (entity_id, (cx, cy)) in
            candidates(registry, context, include_construction)
        {
            for (is_horizontal, coordinate) in [(false, cx), (true, cy)] {
                lines.push(SnapLine {
                    is_horizontal,
                    coordinate,
                    kind: SnapLineType::CENTER,
                    source: Some(entity_id),
                });
            }
        }
        lines
    }

    pub fn produce_points(
        registry: &EntityRegistry,
        context: &SnapContext,
        position: (f64, f64),
        threshold: f64,
        include_construction: bool,
    ) -> Vec<SnapPoint> {
        let (x, y) = position;
        let mut points = Vec::new();
        for (entity_id, (cx, cy)) in
            candidates(registry, context, include_construction)
        {
            let dist = ((cx - x).powi(2) + (cy - y).powi(2)).sqrt();
            if dist <= threshold {
                points.push(SnapPoint {
                    x: cx,
                    y: cy,
                    kind: SnapLineType::CENTER,
                    source: Some(entity_id),
                    spacing: None,
                    is_horizontal: false,
                    pattern_coords: None,
                    axis_coord: None,
                });
            }
        }
        points
    }
}

pub mod entity_points {
    //! Snap lines and points for every sketch point.

    use super::*;

    /// The `(point_id, position)` candidates the producer offers.
    fn candidates(
        registry: &EntityRegistry,
        context: &SnapContext,
    ) -> Vec<(u64, (f64, f64))> {
        let mut out = Vec::new();
        for point_id in registry.point_ids() {
            if context.is_point_dragged(point_id) {
                continue;
            }
            let Some(point) = registry.get_point(point_id) else {
                continue;
            };
            if coincides_with_drag(registry, context, point.x, point.y) {
                continue;
            }
            out.push((point_id, (point.x, point.y)));
        }
        out
    }

    pub fn produce_lines(
        registry: &EntityRegistry,
        context: &SnapContext,
    ) -> Vec<SnapLine> {
        let mut lines = Vec::new();
        for (point_id, (px, py)) in candidates(registry, context) {
            for (is_horizontal, coordinate) in [(false, px), (true, py)] {
                lines.push(SnapLine {
                    is_horizontal,
                    coordinate,
                    kind: SnapLineType::ENTITY_POINT,
                    source: Some(point_id),
                });
            }
        }
        lines
    }

    pub fn produce_points(
        registry: &EntityRegistry,
        context: &SnapContext,
        position: (f64, f64),
        threshold: f64,
    ) -> Vec<SnapPoint> {
        let (x, y) = position;
        let mut points = Vec::new();
        for (point_id, (px, py)) in candidates(registry, context) {
            let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if dist <= threshold {
                points.push(SnapPoint {
                    x: px,
                    y: py,
                    kind: SnapLineType::ENTITY_POINT,
                    source: Some(point_id),
                    spacing: None,
                    is_horizontal: false,
                    pattern_coords: None,
                    axis_coord: None,
                });
            }
        }
        points
    }
}

pub mod midpoints {
    //! Snap points at line and arc midpoints.

    use super::*;

    pub fn produce_points(
        registry: &EntityRegistry,
        context: &SnapContext,
        position: (f64, f64),
        threshold: f64,
    ) -> Vec<SnapPoint> {
        let (x, y) = position;
        let mut points = Vec::new();
        for entity_id in registry.entity_ids() {
            if context.is_entity_dragged(entity_id) {
                continue;
            }
            let Some(entity) = registry.get_entity(entity_id) else {
                continue;
            };
            if entity
                .get_point_ids()
                .iter()
                .any(|pid| context.is_point_dragged(*pid))
            {
                continue;
            }
            let midpoint = match entity.kind() {
                EntityKind::Line => {
                    let ids = entity.get_point_ids();
                    match (
                        registry.get_point(ids[0]),
                        registry.get_point(ids[1]),
                    ) {
                        (Some(p1), Some(p2)) => {
                            Some(((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0))
                        }
                        _ => None,
                    }
                }
                EntityKind::Arc => entity
                    .as_any()
                    .downcast_ref::<Arc>()
                    .and_then(|arc| arc.get_midpoint(registry)),
                _ => None,
            };
            let Some((mx, my)) = midpoint else {
                continue;
            };
            let dist = ((mx - x).powi(2) + (my - y).powi(2)).sqrt();
            if dist <= threshold {
                points.push(SnapPoint {
                    x: mx,
                    y: my,
                    kind: SnapLineType::MIDPOINT,
                    source: Some(entity_id),
                    spacing: None,
                    is_horizontal: false,
                    pattern_coords: None,
                    axis_coord: None,
                });
            }
        }
        points
    }
}

pub mod on_entity {
    //! Snap points on lines, arcs, and circles.

    use super::*;

    fn nearest_on_line(
        line: &Line,
        registry: &EntityRegistry,
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
        registry: &EntityRegistry,
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
        registry: &EntityRegistry,
        x: f64,
        y: f64,
        threshold: f64,
    ) -> Option<(f64, f64)> {
        let center = registry.get_point(circle.center)?;
        let radius_point = registry.get_point(circle.radius_point)?;
        let radius =
            (radius_point.x - center.x).hypot(radius_point.y - center.y);
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

    pub fn produce_points(
        registry: &EntityRegistry,
        context: &SnapContext,
        position: (f64, f64),
        threshold: f64,
    ) -> Vec<SnapPoint> {
        let (x, y) = position;
        let mut points = Vec::new();
        for entity_id in registry.entity_ids() {
            if context.is_entity_dragged(entity_id) {
                continue;
            }
            let Some(entity) = registry.get_entity(entity_id) else {
                continue;
            };
            if entity
                .get_point_ids()
                .iter()
                .any(|pid| context.is_point_dragged(*pid))
            {
                continue;
            }
            let any = entity.as_any();
            let nearest = if let Some(line) = any.downcast_ref::<Line>() {
                nearest_on_line(line, registry, x, y, threshold)
            } else if let Some(arc) = any.downcast_ref::<Arc>() {
                nearest_on_arc(arc, registry, x, y, threshold)
            } else if let Some(circle) = any.downcast_ref::<Circle>() {
                nearest_on_circle(circle, registry, x, y, threshold)
            } else {
                None
            };
            if let Some((nx, ny)) = nearest {
                points.push(SnapPoint {
                    x: nx,
                    y: ny,
                    kind: SnapLineType::ON_ENTITY,
                    source: Some(entity_id),
                    spacing: None,
                    is_horizontal: false,
                    pattern_coords: None,
                    axis_coord: None,
                });
            }
        }
        points
    }
}

/// Parameters of the equidistant snap producer.
#[derive(Clone, Copy, Debug)]
pub struct EquidistantParams {
    pub spacing_tolerance: f64,
    pub max_spacing: f64,
}

pub mod equidistant {
    //! Snap points at equally spaced positions.

    use super::*;

    /// Collects the coordinates aligned with the drag axis and the
    /// mean of the perpendicular values.
    fn collect_aligned(
        points: &[(f64, f64)],
        fixed_y: Option<f64>,
        fixed_x: Option<f64>,
        threshold: f64,
    ) -> (Vec<f64>, f64) {
        let mut coords = Vec::new();
        let mut axis_values = Vec::new();
        for point in points {
            if let Some(fx) = fixed_x {
                if (point.0 - fx).abs() > threshold {
                    continue;
                }
                coords.push(point.1);
                axis_values.push(point.0);
            } else if let Some(fy) = fixed_y {
                if (point.1 - fy).abs() > threshold {
                    continue;
                }
                coords.push(point.0);
                axis_values.push(point.1);
            }
        }
        coords.sort_by(f64::total_cmp);
        coords.dedup();
        let axis = if axis_values.is_empty() {
            0.0
        } else {
            axis_values.iter().sum::<f64>() / axis_values.len() as f64
        };
        (coords, axis)
    }

    fn find_snaps(
        params: &EquidistantParams,
        aligned: &[f64],
        drag_coord: f64,
        threshold: f64,
    ) -> Vec<(f64, f64, Vec<f64>)> {
        if aligned.len() < 2 {
            return Vec::new();
        }
        let mut spacings: HashSet<u64> = HashSet::new();
        for pair in aligned.windows(2) {
            let spacing = pair[1] - pair[0];
            if spacing > 1e-6 && spacing <= params.max_spacing {
                spacings.insert(spacing.to_bits());
            }
        }
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for spacing in spacings.into_iter().map(f64::from_bits) {
            for base in aligned {
                let n = ((drag_coord - base) / spacing).round();
                let snap = base + n * spacing;
                if seen.contains(&snap.to_bits()) {
                    continue;
                }
                if (snap - drag_coord).abs() > threshold {
                    continue;
                }
                let pattern = build_pattern(params, aligned, snap, spacing);
                if pattern.len() >= 3 {
                    seen.insert(snap.to_bits());
                    out.push((snap, spacing, pattern));
                }
            }
        }
        out
    }

    fn build_pattern(
        params: &EquidistantParams,
        aligned: &[f64],
        snap_coord: f64,
        spacing: f64,
    ) -> Vec<f64> {
        let mut all_coords: Vec<f64> = aligned.to_vec();
        all_coords.push(snap_coord);
        let mut pattern = Vec::new();
        let near = |coord: f64| {
            all_coords
                .iter()
                .any(|c| (coord - c).abs() < params.spacing_tolerance)
        };
        let min = all_coords.iter().copied().fold(f64::INFINITY, f64::min);
        let max = all_coords.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut coord = snap_coord;
        while near(coord) {
            pattern.push(coord);
            coord -= spacing;
            if coord < min - spacing {
                break;
            }
        }
        coord = snap_coord + spacing;
        while near(coord) {
            pattern.push(coord);
            coord += spacing;
            if coord > max + spacing {
                break;
            }
        }
        pattern.sort_by(f64::total_cmp);
        pattern
    }

    pub fn produce_points(
        registry: &EntityRegistry,
        context: &SnapContext,
        params: &EquidistantParams,
        position: (f64, f64),
        threshold: f64,
    ) -> Vec<SnapPoint> {
        let (x, y) = position;
        let mut points = Vec::new();
        let candidates: Vec<(f64, f64)> = registry
            .point_ids()
            .into_iter()
            .filter(|id| !context.is_point_dragged(*id))
            .filter_map(|id| registry.get_point(id))
            .map(|p| (p.x, p.y))
            .collect();

        let (v_coords, axis_x) =
            collect_aligned(&candidates, None, Some(x), threshold);
        let (h_coords, axis_y) =
            collect_aligned(&candidates, Some(y), None, threshold);

        for (snap_coord, spacing, pattern) in
            find_snaps(params, &v_coords, y, threshold)
        {
            points.push(SnapPoint {
                x: axis_x,
                y: snap_coord,
                kind: SnapLineType::EQUIDISTANT,
                source: None,
                spacing: Some(spacing),
                is_horizontal: true,
                pattern_coords: Some(pattern),
                axis_coord: Some(axis_x),
            });
        }
        for (snap_coord, spacing, pattern) in
            find_snaps(params, &h_coords, x, threshold)
        {
            points.push(SnapPoint {
                x: snap_coord,
                y: axis_y,
                kind: SnapLineType::EQUIDISTANT,
                source: None,
                spacing: Some(spacing),
                is_horizontal: false,
                pattern_coords: Some(pattern),
                axis_coord: Some(axis_y),
            });
        }
        points
    }
}

pub mod intersections {
    //! Snap points at entity intersections.

    use super::*;

    /// The entities that participate in intersection snapping.
    fn participating(
        registry: &EntityRegistry,
        context: &SnapContext,
        include_construction: bool,
    ) -> Vec<Box<dyn Entity>> {
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
            entities.push(entity.clone_box());
        }
        entities
    }

    pub fn all_intersections(
        registry: &EntityRegistry,
        context: &SnapContext,
        include_construction: bool,
    ) -> Vec<(f64, f64)> {
        let entities = participating(registry, context, include_construction);
        let mut out = Vec::new();
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                out.extend(pair_intersections(
                    entities[i].as_ref(),
                    entities[j].as_ref(),
                    registry,
                ));
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
                let radius = (radius_point.x - center.x)
                    .hypot(radius_point.y - center.y);
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

    fn arc_arc(
        a1: &Arc,
        a2: &Arc,
        registry: &EntityRegistry,
    ) -> Vec<(f64, f64)> {
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
}

/// Selects the best snap point: highest priority first, then the
/// nearest; falls back to the primary-exit hysteresis (same kind and
/// source as the last primary within the exit radius).
///
/// Returns the index into `points` and the distance.
pub fn select_snap_point(
    points: &[SnapPoint],
    x: f64,
    y: f64,
    threshold: f64,
    last_primary: Option<&SnapPoint>,
) -> Option<(usize, f64)> {
    let mut best: Option<usize> = None;
    let mut best_dist = threshold;
    let mut best_priority = -1;
    for (index, point) in points.iter().enumerate() {
        let dist = ((x - point.x).powi(2) + (y - point.y).powi(2)).sqrt();
        let priority = point.kind.priority_value();
        if dist > threshold {
            continue;
        }
        if priority > best_priority
            || (priority == best_priority && dist < best_dist)
        {
            best_dist = dist;
            best = Some(index);
            best_priority = priority;
        }
    }
    if best.is_some() {
        return best.map(|index| (index, best_dist));
    }
    if let Some(last) = last_primary {
        let exit_radius = threshold * 1.5;
        for (index, point) in points.iter().enumerate() {
            if point.kind.0 != last.kind.0 {
                continue;
            }
            if source_key(point.source) != source_key(last.source) {
                continue;
            }
            let dist = ((x - point.x).powi(2) + (y - point.y).powi(2)).sqrt();
            if dist <= exit_radius {
                return Some((index, dist));
            }
        }
    }
    None
}

/// The hysteresis key of a snap source: `Some` ids compare by id,
/// `None` compares as its own group.
fn source_key(source: Option<u64>) -> (u8, u64) {
    match source {
        Some(id) => (0, id),
        None => (2, 0),
    }
}

/// The axis guides the dragged preview entities imply: one vertical
/// and one horizontal line per non-dragged point of every preview
/// entity.
pub fn dynamic_preview_lines(
    registry: &EntityRegistry,
    context: &SnapContext,
) -> Vec<SnapLine> {
    let mut lines = Vec::new();
    for entity_id in &context.preview_entity_ids {
        let Some(entity) = registry.get_entity(*entity_id) else {
            continue;
        };
        for point_id in entity.get_point_ids() {
            if context.is_point_dragged(point_id) {
                continue;
            }
            let Some(point) = registry.get_point(point_id) else {
                continue;
            };
            for (is_horizontal, coordinate) in
                [(false, point.x), (true, point.y)]
            {
                lines.push(SnapLine {
                    is_horizontal,
                    coordinate,
                    kind: SnapLineType::ENTITY_POINT,
                    source: None,
                });
            }
        }
    }
    lines
}
