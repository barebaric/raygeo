//! Constraint dictionaries: parse the reference `.ryp` constraint
//! shapes (one dict per constraint, keyed by class name) into the
//! solver-math constraints, resolving entity references through the
//! registry exactly like the reference `to_solver_constraint`
//! methods.

use super::angle::AngleConstraint;
use serde_json::Value;

use super::aspect_ratio::AspectRatioConstraint;
use super::coincident::CoincidentConstraint;
use super::collinear::CollinearConstraint;
use super::diameter::DiameterConstraint;
use super::distance::DistanceConstraint;
use super::drag::DragConstraint;
use super::equal_distance::EqualDistanceConstraint;
use super::equal_length::EqualLengthConstraint;
use super::horizontal::HorizontalConstraint;
use super::parallelogram::ParallelogramConstraint;
use super::perpendicular::{PerpendicularConstraint, PerpendicularTarget};
use super::point_on_curve::PointOnCurveConstraint;
use super::point_on_line::{PointOnLineConstraint, PointOnTarget};
use super::radius::RadiusConstraint;
use super::rotational::RotationalConstraint;
use super::symmetry::{SymmetryConstraint, SymmetryTarget};
use super::tangent::TangentConstraint;
use super::vertical::VerticalConstraint;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::registry::EntityRegistry;

/// A stored constraint: the solver-math struct built from its
/// reference dictionary, the dictionary itself (data and
/// serialization source of truth), and its referenced point ids.
pub struct ConstraintEntry {
    pub constraint: Box<dyn SketchConstraint>,
    pub dict: Value,
    pub referenced_points: Vec<u64>,
}

impl ConstraintEntry {
    pub fn new(
        constraint: Box<dyn SketchConstraint>,
        dict: Value,
        referenced_points: Vec<u64>,
    ) -> Self {
        ConstraintEntry {
            constraint,
            dict,
            referenced_points,
        }
    }
}

fn u64_field(data: &Value, key: &str) -> Option<u64> {
    data.get(key).and_then(Value::as_u64)
}

fn f64_field(data: &Value, key: &str) -> Option<f64> {
    data.get(key).and_then(Value::as_f64)
}

fn opt_u64(data: &Value, key: &str) -> Option<u64> {
    data.get(key).and_then(Value::as_u64)
}

/// Resolves the radius-bearing point id of an arc/circle entity.
fn radius_point_id(model: &EntityRegistry, entity_id: u64) -> Option<u64> {
    let ids = model.entity_point_ids(entity_id)?;
    match model.entity_type(entity_id)? {
        "arc" => ids.first().copied(),
        "circle" => ids.get(1).copied(),
        _ => None,
    }
}

/// Resolves the center point id of an arc/circle entity.
fn center_point_id(model: &EntityRegistry, entity_id: u64) -> Option<u64> {
    let ids = model.entity_point_ids(entity_id)?;
    match model.entity_type(entity_id)? {
        "arc" => ids.get(2).copied(),
        "circle" => ids.first().copied(),
        _ => None,
    }
}

fn line_endpoints(
    model: &EntityRegistry,
    entity_id: u64,
) -> Option<(u64, u64)> {
    let ids = model.entity_point_ids(entity_id)?;
    Some((*ids.first()?, *ids.get(1)?))
}

/// Parses a reference constraint dictionary into the solver-math
/// constraint, resolving entity references through `model`.
pub fn constraint_from_dict(
    data: &Value,
    model: &EntityRegistry,
) -> Option<Box<dyn SketchConstraint>> {
    match data.get("type").and_then(Value::as_str)? {
        "HorizontalConstraint" => Some(Box::new(HorizontalConstraint::new(
            u64_field(data, "p1")?,
            u64_field(data, "p2")?,
        ))),
        "VerticalConstraint" => Some(Box::new(VerticalConstraint::new(
            u64_field(data, "p1")?,
            u64_field(data, "p2")?,
        ))),
        "CoincidentConstraint" => Some(Box::new(CoincidentConstraint::new(
            u64_field(data, "p1")?,
            u64_field(data, "p2")?,
        ))),
        "CollinearConstraint" => Some(Box::new(CollinearConstraint::new(
            u64_field(data, "p1")?,
            u64_field(data, "p2")?,
            u64_field(data, "p3")?,
        ))),
        "EqualDistanceConstraint" => {
            Some(Box::new(EqualDistanceConstraint::new(
                u64_field(data, "p1")?,
                u64_field(data, "p2")?,
                u64_field(data, "p3")?,
                u64_field(data, "p4")?,
            )))
        }
        "ParallelogramConstraint" => {
            Some(Box::new(ParallelogramConstraint::new(
                u64_field(data, "p_origin")?,
                u64_field(data, "p_width")?,
                u64_field(data, "p_height")?,
                u64_field(data, "p4")?,
            )))
        }
        "DistanceConstraint" => Some(Box::new(DistanceConstraint::new(
            u64_field(data, "p1")?,
            u64_field(data, "p2")?,
            f64_field(data, "value")?,
        ))),
        "DragConstraint" => Some(Box::new(DragConstraint::new(
            u64_field(data, "point")?,
            f64_field(data, "target_x")?,
            f64_field(data, "target_y")?,
            f64_field(data, "weight")?,
        ))),
        "RadiusConstraint" => {
            // Round trip uses the resolved keys from to_json; entity
            // references resolve through the registry as fallback.
            let center = match u64_field(data, "center_idx") {
                Some(center) => center,
                None => center_point_id(model, u64_field(data, "entity_id")?)?,
            };
            let radius_point = match u64_field(data, "radius_pt_idx") {
                Some(radius_point) => radius_point,
                None => radius_point_id(model, u64_field(data, "entity_id")?)?,
            };
            Some(Box::new(RadiusConstraint::new(
                center,
                radius_point,
                f64_field(data, "value")?,
            )))
        }
        "DiameterConstraint" => {
            let center = match u64_field(data, "center_idx") {
                Some(center) => center,
                None => center_point_id(model, u64_field(data, "circle_id")?)?,
            };
            let radius_point = match u64_field(data, "radius_pt_idx") {
                Some(radius_point) => radius_point,
                None => radius_point_id(model, u64_field(data, "circle_id")?)?,
            };
            Some(Box::new(DiameterConstraint::new(
                center,
                radius_point,
                f64_field(data, "value")?,
            )))
        }
        "PerpendicularConstraint" => {
            // Two dict shapes are accepted: resolved keys from the
            // round trip (a1_idx/l1_idx/c1_idx families), or entity
            // references (e1_id/e2_id) resolved through the registry
            // like the reference to_solver_constraint.
            let target = if let Some(a1) = u64_field(data, "a1_idx") {
                PerpendicularTarget::LineLine {
                    a1,
                    a2: u64_field(data, "a2_idx")?,
                    b1: u64_field(data, "b1_idx")?,
                    b2: u64_field(data, "b2_idx")?,
                }
            } else if let Some(l1) = u64_field(data, "l1_idx") {
                PerpendicularTarget::LineCenter {
                    l1,
                    l2: u64_field(data, "l2_idx")?,
                    center: u64_field(data, "center_idx")?,
                }
            } else if let Some(c1) = u64_field(data, "c1_idx") {
                PerpendicularTarget::Circles {
                    c1,
                    r1_point: u64_field(data, "r1_pt_idx")?,
                    c2: u64_field(data, "c2_idx")?,
                    r2_point: u64_field(data, "r2_pt_idx")?,
                }
            } else {
                let e1 = u64_field(data, "e1_id")?;
                let e2 = u64_field(data, "e2_id")?;
                let t1 = model.entity_type(e1)?;
                let t2 = model.entity_type(e2)?;
                match (t1, t2) {
                    ("line", "line") => {
                        let (a1, a2) = line_endpoints(model, e1)?;
                        let (b1, b2) = line_endpoints(model, e2)?;
                        PerpendicularTarget::LineLine { a1, a2, b1, b2 }
                    }
                    ("line", other) => {
                        let (l1, l2) = line_endpoints(model, e1)?;
                        let center = center_point_id(model, e2)?;
                        let _ = other;
                        PerpendicularTarget::LineCenter { l1, l2, center }
                    }
                    (_, "line") => {
                        let (l1, l2) = line_endpoints(model, e2)?;
                        let center = center_point_id(model, e1)?;
                        PerpendicularTarget::LineCenter { l1, l2, center }
                    }
                    _ => {
                        let c1 = center_point_id(model, e1)?;
                        let r1 = radius_point_id(model, e1)?;
                        let c2 = center_point_id(model, e2)?;
                        let r2 = radius_point_id(model, e2)?;
                        PerpendicularTarget::Circles {
                            c1,
                            r1_point: r1,
                            c2,
                            r2_point: r2,
                        }
                    }
                }
            };
            Some(Box::new(PerpendicularConstraint::new(target)))
        }
        "TangentConstraint" => {
            let (l1, l2) = line_endpoints(model, u64_field(data, "line_id")?)?;
            let shape = u64_field(data, "shape_id")?;
            let center = center_point_id(model, shape)?;
            let radius_point = radius_point_id(model, shape)?;
            Some(Box::new(TangentConstraint::new(
                l1,
                l2,
                center,
                radius_point,
            )))
        }
        "EqualLengthConstraint" => {
            let entity_ids = data.get("entity_ids")?.as_array()?;
            if entity_ids.len() < 2 {
                return None;
            }
            let ids: Vec<u64> =
                entity_ids.iter().map(|v| v.as_u64().unwrap_or(0)).collect();
            let base_pairs =
                model.get_entity(ids[0])?.characteristic_length_pairs();
            if base_pairs.is_empty() {
                return None;
            }
            let mut pairs = Vec::new();
            for id in ids.iter().skip(1) {
                let entity = model.get_entity(*id)?;
                let ent_pairs = entity.characteristic_length_pairs();
                let count = base_pairs.len().max(ent_pairs.len());
                for j in 0..count {
                    let bp = base_pairs.get(j).or(base_pairs.first())?;
                    let ep = ent_pairs.get(j).or(ent_pairs.first())?;
                    pairs.push([bp[0], bp[1], ep[0], ep[1]]);
                }
            }
            Some(Box::new(EqualLengthConstraint::new(pairs)))
        }
        "PointOnLineConstraint" => {
            let point = u64_field(data, "point_id")?;
            // Two dict shapes: resolved keys (l1_idx/l2_idx for a
            // line, center_idx/radius_pt_idx for a circle/arc), or
            // entity references (shape_id) resolved through the
            // registry.
            let target = if let (Some(l1), Some(l2)) =
                (u64_field(data, "l1_idx"), u64_field(data, "l2_idx"))
            {
                PointOnTarget::Line { l1, l2 }
            } else if let (Some(center), Some(radius_point)) = (
                u64_field(data, "center_idx"),
                u64_field(data, "radius_pt_idx"),
            ) {
                PointOnTarget::RadiusCurve {
                    center,
                    radius_point,
                }
            } else {
                let shape = u64_field(data, "shape_id")?;
                match model.entity_type(shape)? {
                    "line" => {
                        let ids = model.entity_point_ids(shape)?;
                        PointOnTarget::Line {
                            l1: *ids.first()?,
                            l2: *ids.get(1)?,
                        }
                    }
                    _ => PointOnTarget::RadiusCurve {
                        center: center_point_id(model, shape)?,
                        radius_point: radius_point_id(model, shape)?,
                    },
                }
            };
            Some(Box::new(PointOnLineConstraint::new(point, target)))
        }
        "PointOnCurveConstraint" => {
            let point = u64_field(data, "point_id")?;
            let shape = u64_field(data, "shape_id")?;
            if model.entity_type(shape)? != "bezier" {
                return None;
            }
            let bezier = model.get_entity(shape)?;
            let dict = bezier.to_json();
            let start = u64_field(&dict, "start_idx")?;
            let end = u64_field(&dict, "end_idx")?;
            let cp = |dx: &str, dy: &str| -> Option<[f64; 2]> {
                Some([f64_field(&dict, dx)?, f64_field(&dict, dy)?])
            };
            let cp1_offset = cp("cp1_dx", "cp1_dy").unwrap_or([0.0, 0.0]);
            let cp2_offset = cp("cp2_dx", "cp2_dy").unwrap_or([0.0, 0.0]);
            let is_line =
                dict.get("cp1_dx").is_none() && dict.get("cp2_dx").is_none();
            Some(Box::new(PointOnCurveConstraint::new(
                point, start, end, cp1_offset, cp2_offset, is_line,
            )))
        }
        "SymmetryConstraint" => {
            let p1 = u64_field(data, "p1")?;
            let p2 = u64_field(data, "p2")?;
            let target = if let Some(center) = opt_u64(data, "center") {
                SymmetryTarget::Point { center }
            } else {
                let axis = opt_u64(data, "axis")?;
                let (l1, l2) = line_endpoints(model, axis)?;
                SymmetryTarget::Line { l1, l2 }
            };
            Some(Box::new(SymmetryConstraint::new(p1, p2, target)))
        }
        "AspectRatioConstraint" => Some(Box::new(AspectRatioConstraint::new(
            u64_field(data, "p1")?,
            u64_field(data, "p2")?,
            u64_field(data, "p3")?,
            u64_field(data, "p4")?,
            f64_field(data, "ratio")?,
        ))),
        "AngleConstraint" => {
            let e1 = u64_field(data, "e1_id")?;
            let e2 = u64_field(data, "e2_id")?;
            let (p1, p2) = line_endpoints(model, e1)?;
            let (p3, p4) = line_endpoints(model, e2)?;
            let value_deg = f64_field(data, "value")?;
            let e1_far = opt_u64(data, "e1_far_idx");
            let e2_far = opt_u64(data, "e2_far_idx");
            Some(Box::new(AngleConstraint::new(
                p1, p2, p3, p4, e1_far, e2_far, value_deg,
            )))
        }
        "RotationalConstraint" => Some(Box::new(RotationalConstraint::new(
            u64_field(data, "center")?,
            u64_field(data, "p1")?,
            u64_field(data, "p2")?,
            f64_field(data, "value")?,
        ))),
        _ => None,
    }
}
