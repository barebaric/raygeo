//! Sketch document data that is shared with the host.

use serde_json::{json, Value};

use crate::sketch::object_list::{shared_list, SharedList};

/// Current sketch file format version.
pub const FORMAT_VERSION: u32 = 1;

/// Default fill color used by the sketcher.
pub const DEFAULT_FILL_COLOR: [f64; 4] = [0.85, 0.85, 0.85, 0.7];

/// How a sketch fill is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillStyle {
    Solid,
    LinearGradient,
    RadialGradient,
}

impl FillStyle {
    /// The reference serialization value.
    pub fn as_str(self) -> &'static str {
        match self {
            FillStyle::Solid => "solid",
            FillStyle::LinearGradient => "linear_gradient",
            FillStyle::RadialGradient => "radial_gradient",
        }
    }

    /// Parses the reference serialization value; defaults to solid.
    pub fn parse(value: &str) -> Self {
        match value {
            "linear_gradient" => FillStyle::LinearGradient,
            "radial_gradient" => FillStyle::RadialGradient,
            _ => FillStyle::Solid,
        }
    }
}

/// A filled area bounded by sketch entities.
#[derive(Clone, Debug)]
pub struct Fill {
    pub uid: String,
    pub boundary: Vec<(u64, bool)>,
    pub style: FillStyle,
    pub color: [f64; 4],
    pub gradient_stops: Vec<(f64, [f64; 4])>,
    pub gradient_angle: f64,
}

impl Fill {
    /// Creates a fill with the reference defaults.
    pub fn new(
        uid: String,
        boundary: Vec<(u64, bool)>,
        style: FillStyle,
        color: [f64; 4],
        gradient_stops: Vec<(f64, [f64; 4])>,
        gradient_angle: f64,
    ) -> Self {
        Fill {
            uid,
            boundary,
            style,
            color,
            gradient_stops,
            gradient_angle,
        }
    }

    /// Serializes the fill to the reference dictionary shape.
    pub fn to_json(&self) -> Value {
        let mut data = json!({
            "uid": self.uid,
            "boundary": self
                .boundary
                .iter()
                .map(|(id, forward)| json!([id, forward]))
                .collect::<Vec<_>>(),
            "style": self.style.as_str(),
            "color": self.color,
        });
        if !self.gradient_stops.is_empty() {
            data["gradient_stops"] = json!(self
                .gradient_stops
                .iter()
                .map(|(pos, color)| json!([pos, color]))
                .collect::<Vec<_>>());
        }
        if self.gradient_angle != 0.0 {
            data["gradient_angle"] = json!(self.gradient_angle);
        }
        data
    }

    /// Deserializes a fill from the reference dictionary shape.
    pub fn from_json(data: &Value) -> Option<Fill> {
        let boundary = data
            .get("boundary")?
            .as_array()?
            .iter()
            .map(|item| {
                let pair = item.as_array()?;
                Some((pair.first()?.as_u64()?, pair.get(1)?.as_bool()?))
            })
            .collect::<Option<Vec<_>>>()?;
        let color = read_color(data.get("color"))?;
        let style = FillStyle::parse(
            data.get("style").and_then(Value::as_str).unwrap_or("solid"),
        );
        let gradient_stops = data
            .get("gradient_stops")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let pair = item.as_array()?;
                        Some((
                            pair.first()?.as_f64()?,
                            read_color(pair.get(1))?,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(Fill {
            uid: data
                .get("uid")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            boundary,
            style,
            color,
            gradient_stops,
            gradient_angle: data
                .get("gradient_angle")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        })
    }
}

fn read_color(value: Option<&Value>) -> Option<[f64; 4]> {
    let array = value?.as_array()?;
    Some([
        array.first()?.as_f64()?,
        array.get(1)?.as_f64()?,
        array.get(2)?.as_f64()?,
        array.get(3)?.as_f64()?,
    ])
}

/// Locating metadata of a sketch document.
#[derive(Clone, Debug)]
pub struct DocumentMeta {
    pub format_version: u32,
    pub uid: String,
    pub name: String,
    pub hidden: bool,
    pub origin_id: u64,
}

impl DocumentMeta {
    /// Creates metadata for a new document.
    pub fn new(uid: String, name: String) -> Self {
        DocumentMeta {
            format_version: FORMAT_VERSION,
            uid,
            name,
            hidden: false,
            origin_id: 0,
        }
    }

    /// Serializes the metadata to the reference dictionary shape.
    pub fn to_json(&self) -> Value {
        json!({
            "version": self.format_version,
            "uid": self.uid,
            "name": self.name,
            "hidden": self.hidden,
            "origin_id": self.origin_id,
        })
    }

    /// Deserializes metadata from a reference sketch dictionary.
    pub fn from_json(data: &Value) -> DocumentMeta {
        DocumentMeta {
            format_version: data
                .get("version")
                .and_then(Value::as_u64)
                .unwrap_or(1) as u32,
            uid: data
                .get("uid")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            name: data
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            hidden: data
                .get("hidden")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            origin_id: data
                .get("origin_id")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        }
    }
}

/// The host-agnostic sketch document core.
///
/// The document owns its constraint and array collections. The type
/// parameters are host handles; the Python binding instantiates them
/// with its constraint and array wrapper classes. A pure Rust host can
/// use the same document with core constraint and array values.
#[derive(Clone)]
pub struct Sketch<TConstraint, TArray> {
    /// Constraints in document order.
    pub constraints: SharedList<TConstraint>,
    /// Array definitions in document order.
    pub arrays: SharedList<TArray>,
}

impl<TConstraint, TArray> Sketch<TConstraint, TArray> {
    /// Creates an empty document.
    pub fn new() -> Self {
        Sketch {
            constraints: shared_list(),
            arrays: shared_list(),
        }
    }
}

impl<TConstraint, TArray> Default for Sketch<TConstraint, TArray> {
    fn default() -> Self {
        Sketch::new()
    }
}
