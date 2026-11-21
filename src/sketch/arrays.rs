//! Persistent definitions of sketch arrays.

use serde_json::{json, Value};

use crate::sketch::entities::arc::Arc;
use crate::sketch::entities::bezier::Bezier;
use crate::sketch::entities::circle::Circle;
use crate::sketch::entities::entity::Entity;
use crate::sketch::entities::line::Line;
use crate::sketch::entity_group::EntityGroup;
use crate::sketch::registry::EntityRegistry;
use crate::sketch::transform::PlacementTransform;

/// The array kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayMode {
    Circular,
    CurveAlong,
}

impl ArrayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ArrayMode::Circular => "circular",
            ArrayMode::CurveAlong => "curve_along",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "circular" => Some(ArrayMode::Circular),
            "curve_along" => Some(ArrayMode::CurveAlong),
            _ => None,
        }
    }
}

/// Fields shared by every array definition.
#[derive(Clone, Debug)]
pub struct ArrayCommon {
    pub uid: String,
    pub guide_circle_id: u64,
    pub members: Vec<(i64, Vec<u64>)>,
    pub standalone_pids: Vec<(i64, Vec<u64>)>,
    pub count: u32,
    pub rotate_copies: bool,
}

impl ArrayCommon {
    pub fn new(
        uid: String,
        guide_circle_id: u64,
        members: Vec<(i64, Vec<u64>)>,
        count: u32,
        rotate_copies: bool,
    ) -> Self {
        ArrayCommon {
            uid,
            guide_circle_id,
            members,
            standalone_pids: Vec::new(),
            count,
            rotate_copies,
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "uid": self.uid,
            "guide_circle_id": self.guide_circle_id,
            "members": self
                .members
                .iter()
                .map(|(slot, eids)| json!([slot, eids]))
                .collect::<Vec<_>>(),
            "standalone_pids": self
                .standalone_pids
                .iter()
                .map(|(slot, pids)| json!([slot, pids]))
                .collect::<Vec<_>>(),
            "count": self.count,
            "rotate_copies": self.rotate_copies,
        })
    }

    fn snapshot(&self) -> serde_json::Map<String, Value> {
        let mut map = serde_json::Map::new();
        map.insert(
            "members".to_string(),
            json!(self
                .members
                .iter()
                .map(|(slot, eids)| json!([slot, eids]))
                .collect::<Vec<_>>()),
        );
        map.insert(
            "standalone_pids".to_string(),
            json!(self
                .standalone_pids
                .iter()
                .map(|(slot, pids)| json!([slot, pids]))
                .collect::<Vec<_>>()),
        );
        map.insert("count".to_string(), json!(self.count));
        map
    }

    fn restore(&mut self, data: &Value) {
        self.members = read_pairs(data.get("members"));
        self.standalone_pids = read_pairs(data.get("standalone_pids"));
        self.count = data
            .get("count")
            .and_then(Value::as_u64)
            .unwrap_or(self.count as u64) as u32;
    }
}

/// A circular array definition.
#[derive(Clone, Debug)]
pub struct CircularArray {
    pub common: ArrayCommon,
    pub total_angle_deg: f64,
}

impl CircularArray {
    pub fn new(
        uid: String,
        guide_circle_id: u64,
        members: Vec<(i64, Vec<u64>)>,
        count: u32,
        total_angle_deg: f64,
        rotate_copies: bool,
    ) -> Self {
        CircularArray {
            common: ArrayCommon::new(
                uid,
                guide_circle_id,
                members,
                count,
                rotate_copies,
            ),
            total_angle_deg,
        }
    }
}

/// An array along a curve definition.
#[derive(Clone, Debug)]
pub struct CurveAlongArray {
    pub common: ArrayCommon,
    pub path_entity_id: i64,
    pub align_to_tangent: bool,
    pub offset_to_start: f64,
    pub spacing: f64,
    pub template_anchor: Option<((f64, f64), f64)>,
}

impl CurveAlongArray {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uid: String,
        guide_circle_id: u64,
        members: Vec<(i64, Vec<u64>)>,
        count: u32,
        rotate_copies: bool,
        path_entity_id: i64,
        align_to_tangent: bool,
        offset_to_start: f64,
        spacing: f64,
        template_anchor: Option<((f64, f64), f64)>,
    ) -> Self {
        CurveAlongArray {
            common: ArrayCommon::new(
                uid,
                guide_circle_id,
                members,
                count,
                rotate_copies,
            ),
            path_entity_id,
            align_to_tangent,
            offset_to_start,
            spacing,
            template_anchor,
        }
    }
}

/// A concrete array definition.
#[derive(Clone, Debug)]
pub enum ArrayDefinition {
    Circular(CircularArray),
    CurveAlong(CurveAlongArray),
}

impl ArrayDefinition {
    /// Living members, pruned to surviving entities.
    pub fn living_members(
        &self,
        registry: &EntityRegistry,
    ) -> Vec<(i64, Vec<u64>)> {
        let mut living = Vec::new();
        for (slot, eids) in &self.common().members {
            let alive: Vec<u64> = eids
                .iter()
                .copied()
                .filter(|eid| registry.get_entity(*eid).is_some())
                .collect();
            if !alive.is_empty() {
                living.push((*slot, alive));
            }
        }
        living.sort_by_key(|(slot, _)| *slot);
        living
    }

    pub fn living_entity_ids(&self, registry: &EntityRegistry) -> Vec<u64> {
        self.living_members(registry)
            .into_iter()
            .flat_map(|(_, eids)| eids)
            .collect()
    }

    pub fn occupied_slots(
        &self,
        registry: &EntityRegistry,
    ) -> std::collections::HashSet<i64> {
        self.living_members(registry)
            .into_iter()
            .map(|(slot, _)| slot)
            .collect()
    }

    pub fn guide_signature(&self, registry: &EntityRegistry) -> Value {
        registry
            .get_entity(self.common().guide_circle_id)
            .map(|entity| entity.geometry_signature(registry))
            .unwrap_or(Value::Null)
    }

    pub fn template_signature(&self, registry: &EntityRegistry) -> Value {
        let living = self.living_members(registry);
        let Some((_, eids)) = living.first() else {
            return Value::Array(Vec::new());
        };
        Value::Array(
            eids.iter()
                .filter_map(|eid| registry.get_entity(*eid))
                .map(|entity| entity.geometry_signature(registry))
                .collect(),
        )
    }

    pub fn is_guide_radius_point(
        &self,
        registry: &EntityRegistry,
        pid: u64,
    ) -> bool {
        if !matches!(self, ArrayDefinition::Circular(_)) {
            return false;
        }
        registry
            .get_entity(self.common().guide_circle_id)
            .and_then(|entity| entity.as_any().downcast_ref::<Circle>())
            .map(|circle| circle.radius_point == pid)
            .unwrap_or(false)
    }

    /// Drops deleted entities from member groups.
    pub fn prune(&mut self, registry: &EntityRegistry) -> bool {
        if registry.get_entity(self.common().guide_circle_id).is_none() {
            return false;
        }
        let living = self.living_members(registry);
        let common = self.common_mut();
        common.members = living;
        common.standalone_pids.retain(|(_, pids)| {
            for pid in pids {
                if registry.get_point(*pid).is_none() {
                    return false;
                }
            }
            true
        });
        true
    }

    pub fn make_circular_strategy(
        &self,
        registry: &EntityRegistry,
    ) -> Option<CircularArrayStrategy> {
        let ArrayDefinition::Circular(array) = self else {
            return None;
        };
        let circle = registry
            .get_entity(array.common.guide_circle_id)?
            .as_any()
            .downcast_ref::<Circle>()?;
        let center = registry.point_position(circle.center)?;
        let radius_point = registry.point_position(circle.radius_point)?;
        Some(CircularArrayStrategy {
            count: array.common.count,
            rotate_copies: array.common.rotate_copies,
            total_angle_deg: array.total_angle_deg,
            center,
            radius: (radius_point.0 - center.0)
                .hypot(radius_point.1 - center.1),
        })
    }

    pub fn make_curve_strategy(&self) -> Option<CurveAlongArrayStrategy> {
        let ArrayDefinition::CurveAlong(array) = self else {
            return None;
        };
        Some(CurveAlongArrayStrategy {
            count: array.common.count,
            rotate_copies: array.common.rotate_copies,
            path_entity_id: array.path_entity_id,
            align_to_tangent: array.align_to_tangent,
            offset_to_start: array.offset_to_start,
            spacing: array.spacing,
        })
    }

    pub fn commit_circular(&mut self, strategy: &CircularArrayStrategy) {
        if let ArrayDefinition::Circular(array) = self {
            array.common.count = strategy.count;
            array.common.rotate_copies = strategy.rotate_copies;
            array.total_angle_deg = strategy.total_angle_deg;
        }
    }

    pub fn commit_curve(&mut self, strategy: &CurveAlongArrayStrategy) {
        if let ArrayDefinition::CurveAlong(array) = self {
            array.common.count = strategy.count;
            array.common.rotate_copies = strategy.rotate_copies;
            array.path_entity_id = strategy.path_entity_id;
            array.align_to_tangent = strategy.align_to_tangent;
            array.offset_to_start = strategy.offset_to_start;
            array.spacing = strategy.spacing;
        }
    }

    pub fn params_changed_circular(
        &self,
        strategy: &CircularArrayStrategy,
    ) -> bool {
        match self {
            ArrayDefinition::Circular(array) => {
                array.common.count != strategy.count
                    || array.common.rotate_copies != strategy.rotate_copies
                    || array.total_angle_deg != strategy.total_angle_deg
            }
            ArrayDefinition::CurveAlong(_) => true,
        }
    }

    pub fn params_changed_curve(
        &self,
        strategy: &CurveAlongArrayStrategy,
    ) -> bool {
        match self {
            ArrayDefinition::CurveAlong(array) => {
                array.common.count != strategy.count
                    || array.path_entity_id != strategy.path_entity_id
                    || array.align_to_tangent != strategy.align_to_tangent
                    || array.offset_to_start != strategy.offset_to_start
                    || array.spacing != strategy.spacing
            }
            ArrayDefinition::Circular(_) => true,
        }
    }

    pub fn mode(&self) -> ArrayMode {
        match self {
            ArrayDefinition::Circular(_) => ArrayMode::Circular,
            ArrayDefinition::CurveAlong(_) => ArrayMode::CurveAlong,
        }
    }

    pub fn common(&self) -> &ArrayCommon {
        match self {
            ArrayDefinition::Circular(a) => &a.common,
            ArrayDefinition::CurveAlong(a) => &a.common,
        }
    }

    pub fn common_mut(&mut self) -> &mut ArrayCommon {
        match self {
            ArrayDefinition::Circular(a) => &mut a.common,
            ArrayDefinition::CurveAlong(a) => &mut a.common,
        }
    }

    pub fn to_json(&self) -> Value {
        let mut data = self.common().to_json();
        data["mode"] = json!(self.mode().as_str());
        match self {
            ArrayDefinition::Circular(a) => {
                data["total_angle_deg"] = json!(a.total_angle_deg);
            }
            ArrayDefinition::CurveAlong(a) => {
                data["path_entity_id"] = json!(a.path_entity_id);
                data["align_to_tangent"] = json!(a.align_to_tangent);
                data["offset_to_start"] = json!(a.offset_to_start);
                data["spacing"] = json!(a.spacing);
                if let Some(((x, y), angle)) = a.template_anchor {
                    data["template_anchor"] = json!([[x, y], angle]);
                }
            }
        }
        data
    }

    pub fn from_json(data: &Value) -> Option<Self> {
        let mode = ArrayMode::parse(
            data.get("mode")
                .and_then(Value::as_str)
                .unwrap_or("circular"),
        )?;
        let common = ArrayCommon {
            uid: data.get("uid")?.as_str()?.to_string(),
            guide_circle_id: read_u64(data.get("guide_circle_id")?)?,
            members: read_pairs(data.get("members")),
            standalone_pids: read_pairs(data.get("standalone_pids")),
            count: data.get("count").and_then(read_u64).unwrap_or(6) as u32,
            rotate_copies: data
                .get("rotate_copies")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        };
        match mode {
            ArrayMode::Circular => {
                Some(ArrayDefinition::Circular(CircularArray {
                    common,
                    total_angle_deg: data
                        .get("total_angle_deg")
                        .and_then(Value::as_f64)
                        .unwrap_or(360.0),
                }))
            }
            ArrayMode::CurveAlong => {
                Some(ArrayDefinition::CurveAlong(CurveAlongArray {
                    common,
                    path_entity_id: data
                        .get("path_entity_id")
                        .and_then(read_i64)
                        .unwrap_or(-1),
                    align_to_tangent: data
                        .get("align_to_tangent")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    offset_to_start: data
                        .get("offset_to_start")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    spacing: data
                        .get("spacing")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    template_anchor: read_anchor(data.get("template_anchor")),
                }))
            }
        }
    }

    pub fn snapshot(&self) -> Value {
        let mut data = self.common().snapshot();
        match self {
            ArrayDefinition::Circular(a) => {
                data.insert(
                    "total_angle_deg".to_string(),
                    json!(a.total_angle_deg),
                );
                data.insert(
                    "rotate_copies".to_string(),
                    json!(a.common.rotate_copies),
                );
            }
            ArrayDefinition::CurveAlong(a) => {
                data.insert(
                    "path_entity_id".to_string(),
                    json!(a.path_entity_id),
                );
                data.insert(
                    "align_to_tangent".to_string(),
                    json!(a.align_to_tangent),
                );
                data.insert(
                    "offset_to_start".to_string(),
                    json!(a.offset_to_start),
                );
                data.insert("spacing".to_string(), json!(a.spacing));
                data.insert(
                    "template_anchor".to_string(),
                    match a.template_anchor {
                        Some(((x, y), angle)) => json!([[x, y], angle]),
                        None => Value::Null,
                    },
                );
            }
        }
        Value::Object(data)
    }

    pub fn restore(&mut self, state: &Value) {
        self.common_mut().restore(state);
        match self {
            ArrayDefinition::Circular(a) => {
                a.total_angle_deg = state
                    .get("total_angle_deg")
                    .and_then(Value::as_f64)
                    .unwrap_or(a.total_angle_deg);
                a.common.rotate_copies = state
                    .get("rotate_copies")
                    .and_then(Value::as_bool)
                    .unwrap_or(a.common.rotate_copies);
            }
            ArrayDefinition::CurveAlong(a) => {
                a.path_entity_id = state
                    .get("path_entity_id")
                    .and_then(read_i64)
                    .unwrap_or(a.path_entity_id);
                a.align_to_tangent = state
                    .get("align_to_tangent")
                    .and_then(Value::as_bool)
                    .unwrap_or(a.align_to_tangent);
                a.offset_to_start = state
                    .get("offset_to_start")
                    .and_then(Value::as_f64)
                    .unwrap_or(a.offset_to_start);
                a.spacing = state
                    .get("spacing")
                    .and_then(Value::as_f64)
                    .unwrap_or(a.spacing);
                a.template_anchor = read_anchor(state.get("template_anchor"));
            }
        }
    }
}

fn read_pairs(value: Option<&Value>) -> Vec<(i64, Vec<u64>)> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let pair = item.as_array()?;
                    Some((
                        read_i64(pair.first()?)?,
                        pair.get(1)?
                            .as_array()?
                            .iter()
                            .filter_map(read_u64)
                            .collect(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_anchor(value: Option<&Value>) -> Option<((f64, f64), f64)> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    let pair = value.as_array()?;
    let point = pair.first()?.as_array()?;
    Some((
        (point.first()?.as_f64()?, point.get(1)?.as_f64()?),
        pair.get(1)?.as_f64()?,
    ))
}

fn read_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str()?.parse::<i64>().ok())
}

fn read_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse::<u64>().ok())
}

/// How one array instance is derived from the template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementKind {
    Rotation,
    Translation,
    CurveAligned,
}

/// One rigid placement of an array instance.
#[derive(Clone, Copy, Debug)]
pub struct InstancePlacement {
    pub kind: PlacementKind,
    pub angle: f64,
    pub center: (f64, f64),
    pub delta: (f64, f64),
    pub target_center: (f64, f64),
}

impl PlacementTransform for InstancePlacement {
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        InstancePlacement::transform_point(self, x, y)
    }

    fn transform_offset(&self, dx: f64, dy: f64) -> (f64, f64) {
        InstancePlacement::transform_offset(self, dx, dy)
    }
}

impl InstancePlacement {
    pub fn translation(delta: (f64, f64)) -> Self {
        InstancePlacement {
            kind: PlacementKind::Translation,
            angle: 0.0,
            center: (0.0, 0.0),
            delta,
            target_center: (0.0, 0.0),
        }
    }

    pub fn rotation(angle: f64, center: (f64, f64)) -> Self {
        InstancePlacement {
            kind: PlacementKind::Rotation,
            angle,
            center,
            delta: (0.0, 0.0),
            target_center: (0.0, 0.0),
        }
    }

    pub fn curve_aligned(
        angle: f64,
        center: (f64, f64),
        target_center: (f64, f64),
    ) -> Self {
        InstancePlacement {
            kind: PlacementKind::CurveAligned,
            angle,
            center,
            delta: (0.0, 0.0),
            target_center,
        }
    }

    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        match self.kind {
            PlacementKind::Rotation | PlacementKind::CurveAligned => {
                let ca = self.angle.cos();
                let sa = self.angle.sin();
                let dx = x - self.center.0;
                let dy = y - self.center.1;
                let origin = if self.kind == PlacementKind::Rotation {
                    self.center
                } else {
                    self.target_center
                };
                (origin.0 + ca * dx - sa * dy, origin.1 + sa * dx + ca * dy)
            }
            PlacementKind::Translation => (x + self.delta.0, y + self.delta.1),
        }
    }

    pub fn transform_offset(&self, dx: f64, dy: f64) -> (f64, f64) {
        match self.kind {
            PlacementKind::Rotation | PlacementKind::CurveAligned => {
                let ca = self.angle.cos();
                let sa = self.angle.sin();
                (ca * dx - sa * dy, sa * dx + ca * dy)
            }
            PlacementKind::Translation => (dx, dy),
        }
    }
}

/// Circular-array placement math.
#[derive(Clone, Debug)]
pub struct CircularArrayStrategy {
    pub count: u32,
    pub rotate_copies: bool,
    pub total_angle_deg: f64,
    pub center: (f64, f64),
    pub radius: f64,
}

impl CircularArrayStrategy {
    pub fn template_placement(
        &self,
        template_center: (f64, f64),
    ) -> InstancePlacement {
        let (cx, cy) = self.center;
        let vx = template_center.0 - cx;
        let vy = template_center.1 - cy;
        let d = vx.hypot(vy);
        if d < 1e-9 || self.radius <= 0.0 {
            return InstancePlacement::translation((0.0, 0.0));
        }
        let k = self.radius / d;
        InstancePlacement::translation((
            cx + vx * k - template_center.0,
            cy + vy * k - template_center.1,
        ))
    }

    pub fn member_placements(
        &self,
        template_center: (f64, f64),
    ) -> Vec<InstancePlacement> {
        let count = self.count.max(1);
        let total = self.total_angle_deg.to_radians();
        let (cx, cy) = self.center;
        let mut placements = Vec::new();
        for j in 1..count {
            let theta = total * j as f64 / count as f64;
            if self.rotate_copies {
                placements.push(InstancePlacement::rotation(theta, (cx, cy)));
            } else {
                let vx = template_center.0 - cx;
                let vy = template_center.1 - cy;
                let nx = cx + theta.cos() * vx - theta.sin() * vy;
                let ny = cy + theta.sin() * vx + theta.cos() * vy;
                placements.push(InstancePlacement::translation((
                    nx - template_center.0,
                    ny - template_center.1,
                )));
            }
        }
        placements
    }

    pub fn capture_master_frame(
        &self,
        registry: &EntityRegistry,
        guide_id: u64,
    ) -> Option<((f64, f64), f64)> {
        let circle = registry
            .get_entity(guide_id)?
            .as_any()
            .downcast_ref::<Circle>()?;
        let center = registry.point_position(circle.center)?;
        let radius_point = registry.point_position(circle.radius_point)?;
        Some((
            center,
            (radius_point.0 - center.0).hypot(radius_point.1 - center.1),
        ))
    }

    pub fn apply_frame(
        &self,
        registry: &mut EntityRegistry,
        array: &ArrayDefinition,
        old_frame: ((f64, f64), f64),
    ) {
        let ((ocx, ocy), _old_radius) = old_frame;
        let (ncx, ncy) = self.center;
        let (dx, dy) = (ncx - ocx, ncy - ocy);
        if dx.abs() > 1e-12 || dy.abs() > 1e-12 {
            EntityGroup::new(array.living_entity_ids(registry))
                .with_helpers(registry)
                .translate(registry, dx, dy);
        }
        let members = array.living_members(registry);
        for (_, eids) in members {
            let group = EntityGroup::new(eids);
            if group.points(registry).is_empty() {
                continue;
            }
            let center = group.center(registry);
            group.with_helpers(registry).radial_project(
                registry,
                (ncx, ncy),
                self.radius,
                Some(center),
            );
        }
        let circle_ids = registry
            .get_entity(array.common().guide_circle_id)
            .and_then(|entity| {
                entity
                    .as_any()
                    .downcast_ref::<Circle>()
                    .map(|c| (c.center, c.radius_point))
            });
        if let Some((center_id, radius_id)) = circle_ids {
            registry.set_point_position(center_id, ncx, ncy);
            registry.set_point_position(radius_id, ncx + self.radius, ncy);
        }
    }
}

/// Curve-along placement math.
#[derive(Clone, Debug)]
pub struct CurveAlongArrayStrategy {
    pub count: u32,
    pub rotate_copies: bool,
    pub path_entity_id: i64,
    pub align_to_tangent: bool,
    pub offset_to_start: f64,
    pub spacing: f64,
}

impl CurveAlongArrayStrategy {
    pub fn samples(&self, registry: &EntityRegistry) -> Vec<((f64, f64), f64)> {
        if self.path_entity_id < 0 {
            return Vec::new();
        }
        let count = self.resolve_count(registry);
        if count <= 1 {
            return Vec::new();
        }
        let samples = sample_path(
            registry,
            self.path_entity_id as u64,
            count,
            self.offset_to_start,
        );
        if samples.len() < 2 {
            return Vec::new();
        }
        samples
    }

    fn slot_angle(&self, tangent_angle: f64) -> f64 {
        if self.align_to_tangent {
            tangent_angle
        } else {
            0.0
        }
    }

    pub fn template_placement(
        &self,
        template_center: (f64, f64),
        registry: &EntityRegistry,
    ) -> InstancePlacement {
        let samples = self.samples(registry);
        let Some(&(point, tangent_angle)) = samples.first() else {
            return InstancePlacement::translation((0.0, 0.0));
        };
        InstancePlacement::curve_aligned(
            self.slot_angle(tangent_angle),
            template_center,
            point,
        )
    }

    pub fn member_placements(
        &self,
        registry: &EntityRegistry,
    ) -> Vec<InstancePlacement> {
        let samples = self.samples(registry);
        let Some(&(origin, origin_angle)) = samples.first() else {
            return Vec::new();
        };
        let mut placements = Vec::new();
        for &(point, tangent_angle) in samples.iter().skip(1) {
            placements.push(InstancePlacement::curve_aligned(
                self.slot_angle(tangent_angle) - self.slot_angle(origin_angle),
                origin,
                point,
            ));
        }
        placements
    }

    pub fn reanchor_template(
        &self,
        registry: &mut EntityRegistry,
        array: &mut ArrayDefinition,
        template_eids: &[u64],
        standalone_pids: &[u64],
    ) {
        let group = EntityGroup::new(template_eids.to_vec());
        if group.points(registry).is_empty() {
            return;
        }
        let ArrayDefinition::CurveAlong(array) = array else {
            return;
        };
        let Some((_, anchor_angle)) = array.template_anchor else {
            return;
        };
        let current_center = group.center(registry);
        let placement = self.template_placement(current_center, registry);
        if placement.kind != PlacementKind::CurveAligned {
            return;
        }
        let target = placement.target_center;
        let delta = placement.angle - anchor_angle;
        if (target.0 - current_center.0).abs() < 1e-9
            && (target.1 - current_center.1).abs() < 1e-9
            && delta.abs() < 1e-12
        {
            return;
        }
        let motion =
            InstancePlacement::curve_aligned(delta, current_center, target);
        group
            .with_helpers(registry)
            .apply_placement(registry, &motion);
        for pid in standalone_pids {
            if let Some((x, y)) = registry.point_position(*pid) {
                let (nx, ny) = motion.transform_point(x, y);
                registry.set_point_position(*pid, nx, ny);
            }
        }
        array.template_anchor = Some(((target.0, target.1), placement.angle));
    }

    pub fn resolve_count(&self, registry: &EntityRegistry) -> u32 {
        if self.spacing > 1e-9 && self.path_entity_id >= 0 {
            let total = path_length(registry, self.path_entity_id as u64);
            let offset = self.offset_to_start.min(total);
            let usable = (total - offset).max(0.0);
            if usable <= 0.0 {
                return 1;
            }
            return (usable / self.spacing).floor().max(0.0) as u32 + 1;
        }
        self.count.max(1)
    }
}

/// Samples a guide path at equal arc-length intervals.
pub fn sample_path(
    registry: &EntityRegistry,
    path_entity_id: u64,
    count: u32,
    offset_to_start: f64,
) -> Vec<((f64, f64), f64)> {
    let Some(entity) = registry.get_entity(path_entity_id) else {
        return Vec::new();
    };
    let polyline = polyline_for(entity, registry);
    if polyline.len() < 2 {
        return Vec::new();
    }
    let cum = cumulative_lengths(&polyline);
    let total = *cum.last().unwrap_or(&0.0);
    if total <= 1e-9 {
        return Vec::new();
    }
    let offset = offset_to_start.clamp(0.0, total * (1.0 - 1e-6));
    let usable = total - offset;
    if count <= 1 || usable <= 0.0 {
        return vec![(polyline[0], 0.0)];
    }
    let mut samples = Vec::with_capacity(count as usize);
    for j in 0..count {
        let s = offset + usable * j as f64 / (count - 1) as f64;
        let (point, tangent) = point_at_arclength(&polyline, &cum, s);
        samples.push((point, tangent.1.atan2(tangent.0)));
    }
    samples
}

/// Total arc length of a guide path.
pub fn path_length(registry: &EntityRegistry, path_entity_id: u64) -> f64 {
    let Some(entity) = registry.get_entity(path_entity_id) else {
        return 0.0;
    };
    let polyline = polyline_for(entity, registry);
    if polyline.len() < 2 {
        return 0.0;
    }
    *cumulative_lengths(&polyline).last().unwrap_or(&0.0)
}

/// Sampling tolerance for array guide paths, matching the
/// discretization used by contour assembly and geometry emission.
const GUIDE_TOLERANCE: f64 = 0.02;

fn polyline_for(
    entity: &dyn Entity,
    registry: &EntityRegistry,
) -> Vec<(f64, f64)> {
    let any = entity.as_any();
    // Guide paths support the curve kinds the reference supports.
    // Sampling goes through the canonical entity discretization so a
    // guide polyline matches the entity's contour/boolean geometry.
    if any.downcast_ref::<Line>().is_some()
        || any.downcast_ref::<Arc>().is_some()
        || any.downcast_ref::<Bezier>().is_some()
    {
        return entity.to_polyline(registry, GUIDE_TOLERANCE);
    }
    Vec::new()
}

pub(crate) fn cumulative_lengths(polyline: &[(f64, f64)]) -> Vec<f64> {
    let mut cum = Vec::with_capacity(polyline.len());
    cum.push(0.0);
    for pair in polyline.windows(2) {
        let dx = pair[1].0 - pair[0].0;
        let dy = pair[1].1 - pair[0].1;
        cum.push(cum[cum.len() - 1] + dx.hypot(dy));
    }
    cum
}

pub(crate) fn point_at_arclength(
    polyline: &[(f64, f64)],
    cum: &[f64],
    s: f64,
) -> ((f64, f64), (f64, f64)) {
    let tangent = |a: (f64, f64), b: (f64, f64)| {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let n = dx.hypot(dy);
        if n == 0.0 {
            (1.0, 0.0)
        } else {
            (dx / n, dy / n)
        }
    };
    if s <= 0.0 {
        return (polyline[0], tangent(polyline[0], polyline[1]));
    }
    if s >= *cum.last().unwrap_or(&0.0) {
        let a = polyline[polyline.len() - 2];
        let b = polyline[polyline.len() - 1];
        return (b, tangent(a, b));
    }
    let mut lo = 0;
    let mut hi = cum.len() - 1;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if cum[mid] < s {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let seg = lo.max(1);
    let a = polyline[seg - 1];
    let b = polyline[seg];
    let seg_len = cum[seg] - cum[seg - 1];
    let t = if seg_len > 0.0 {
        (s - cum[seg - 1]) / seg_len
    } else {
        0.0
    };
    (
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t),
        tangent(a, b),
    )
}

/// Shared behavior of every sketch array strategy.
pub trait ArrayStrategy: Send + Sync {
    fn count(&self) -> u32;
    fn set_count(&mut self, value: u32);
    fn rotate_copies(&self) -> bool;
    fn set_rotate_copies(&mut self, value: bool);
    fn needs_center_point(&self) -> bool {
        false
    }
    fn uses_template_anchor(&self) -> bool {
        false
    }
    fn existing_master_id(&self) -> Option<i64> {
        None
    }
    fn template_placement(
        &self,
        template_center: (f64, f64),
        registry: Option<&EntityRegistry>,
    ) -> InstancePlacement;
    fn member_placements(
        &self,
        template_center: (f64, f64),
        registry: Option<&EntityRegistry>,
    ) -> Vec<InstancePlacement>;
    fn capture_master_frame(
        &self,
        _registry: &EntityRegistry,
        _array: &ArrayDefinition,
    ) -> Option<((f64, f64), f64)> {
        None
    }
    fn apply_frame(
        &self,
        _registry: &mut EntityRegistry,
        _array: &ArrayDefinition,
        _old_frame: ((f64, f64), f64),
    ) {
    }
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl ArrayStrategy for CircularArrayStrategy {
    fn count(&self) -> u32 {
        self.count
    }

    fn set_count(&mut self, value: u32) {
        self.count = value;
    }

    fn rotate_copies(&self) -> bool {
        self.rotate_copies
    }

    fn set_rotate_copies(&mut self, value: bool) {
        self.rotate_copies = value;
    }

    fn needs_center_point(&self) -> bool {
        true
    }

    fn template_placement(
        &self,
        template_center: (f64, f64),
        _registry: Option<&EntityRegistry>,
    ) -> InstancePlacement {
        CircularArrayStrategy::template_placement(self, template_center)
    }

    fn member_placements(
        &self,
        template_center: (f64, f64),
        _registry: Option<&EntityRegistry>,
    ) -> Vec<InstancePlacement> {
        CircularArrayStrategy::member_placements(self, template_center)
    }

    fn capture_master_frame(
        &self,
        registry: &EntityRegistry,
        array: &ArrayDefinition,
    ) -> Option<((f64, f64), f64)> {
        CircularArrayStrategy::capture_master_frame(
            self,
            registry,
            array.common().guide_circle_id,
        )
    }

    fn apply_frame(
        &self,
        registry: &mut EntityRegistry,
        array: &ArrayDefinition,
        old_frame: ((f64, f64), f64),
    ) {
        CircularArrayStrategy::apply_frame(self, registry, array, old_frame)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ArrayStrategy for CurveAlongArrayStrategy {
    fn count(&self) -> u32 {
        self.count
    }

    fn set_count(&mut self, value: u32) {
        self.count = value;
    }

    fn rotate_copies(&self) -> bool {
        self.rotate_copies
    }

    fn set_rotate_copies(&mut self, value: bool) {
        self.rotate_copies = value;
    }

    fn uses_template_anchor(&self) -> bool {
        true
    }

    fn existing_master_id(&self) -> Option<i64> {
        if self.path_entity_id >= 0 {
            Some(self.path_entity_id)
        } else {
            None
        }
    }

    fn template_placement(
        &self,
        template_center: (f64, f64),
        registry: Option<&EntityRegistry>,
    ) -> InstancePlacement {
        let Some(registry) = registry else {
            return InstancePlacement::translation((0.0, 0.0));
        };
        CurveAlongArrayStrategy::template_placement(
            self,
            template_center,
            registry,
        )
    }

    fn member_placements(
        &self,
        _template_center: (f64, f64),
        registry: Option<&EntityRegistry>,
    ) -> Vec<InstancePlacement> {
        let Some(registry) = registry else {
            return Vec::new();
        };
        CurveAlongArrayStrategy::member_placements(self, registry)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
