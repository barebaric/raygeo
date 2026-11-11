//! Points of the sketch model.

use serde_json::{json, Value};

/// Chirality/handle role of a point inside connected geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaypointType {
    Sharp,
    Smooth,
    Symmetric,
}

impl WaypointType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WaypointType::Sharp => "sharp",
            WaypointType::Smooth => "smooth",
            WaypointType::Symmetric => "symmetric",
        }
    }

    pub fn parse(value: &str) -> Option<WaypointType> {
        match value {
            "sharp" => Some(WaypointType::Sharp),
            "smooth" => Some(WaypointType::Smooth),
            "symmetric" => Some(WaypointType::Symmetric),
            _ => None,
        }
    }
}

/// A sketch point with model coordinates and solver flags.
#[derive(Clone, Debug)]
pub struct SketchPoint {
    /// Unique point id, stable across solves.
    pub id: u64,
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Whether the point is pinned and never moves.
    pub fixed: bool,
    /// Handle role for connected geometry.
    pub waypoint_type: WaypointType,
    /// Fully-constrained state, computed by the solver (not
    /// serialized).
    pub constrained: bool,
}

impl SketchPoint {
    /// Serializes to the reference dictionary shape (`.ryp`
    /// compatible).  The waypoint type is only emitted when it is
    /// not the default `sharp`.
    pub fn to_json(&self) -> Value {
        let mut data = json!({
            "id": self.id,
            "x": self.x,
            "y": self.y,
            "fixed": self.fixed,
        });
        if self.waypoint_type != WaypointType::Sharp {
            data["waypoint_type"] = json!(self.waypoint_type.as_str());
        }
        data
    }

    /// Deserializes from the reference dictionary shape.
    pub fn from_json(data: &Value) -> Option<SketchPoint> {
        Some(SketchPoint {
            id: data.get("id")?.as_u64()?,
            x: data.get("x")?.as_f64()?,
            y: data.get("y")?.as_f64()?,
            fixed: data.get("fixed").and_then(Value::as_bool).unwrap_or(false),
            waypoint_type: data
                .get("waypoint_type")
                .and_then(Value::as_str)
                .and_then(WaypointType::parse)
                .unwrap_or(WaypointType::Sharp),
            constrained: false,
        })
    }
}
