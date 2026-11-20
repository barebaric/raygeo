//! Persistent definitions of sketch arrays.

use serde_json::{json, Value};

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
