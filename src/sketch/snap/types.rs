//! Core value types of the sketch snap engine: snap lines, snap
//! points, their kinds and styles, and the drag context the
//! producers filter against.

/// The kind of a snap line or point, mirroring the reference's
/// ordinal constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SnapLineType(pub u8);

impl SnapLineType {
    pub const ENTITY_POINT_VALUE: u8 = 0;
    pub const ON_ENTITY_VALUE: u8 = 1;
    pub const INTERSECTION_VALUE: u8 = 2;
    pub const MIDPOINT_VALUE: u8 = 3;
    pub const EQUIDISTANT_VALUE: u8 = 4;
    pub const TANGENT_VALUE: u8 = 5;
    pub const CENTER_VALUE: u8 = 6;

    pub const ENTITY_POINT: SnapLineType = SnapLineType(0);
    pub const ON_ENTITY: SnapLineType = SnapLineType(1);
    pub const INTERSECTION: SnapLineType = SnapLineType(2);
    pub const MIDPOINT: SnapLineType = SnapLineType(3);
    pub const EQUIDISTANT: SnapLineType = SnapLineType(4);
    pub const TANGENT: SnapLineType = SnapLineType(5);
    pub const CENTER: SnapLineType = SnapLineType(6);

    /// Selection priority: higher wins when several snaps are
    /// within threshold.
    pub fn priority_value(self) -> i32 {
        match self.0 {
            Self::ENTITY_POINT_VALUE => 100,
            Self::MIDPOINT_VALUE => 90,
            Self::ON_ENTITY_VALUE => 80,
            Self::INTERSECTION_VALUE => 70,
            Self::EQUIDISTANT_VALUE => 60,
            Self::TANGENT_VALUE => 40,
            Self::CENTER_VALUE => 30,
            _ => 0,
        }
    }

    pub fn name_value(self) -> &'static str {
        match self.0 {
            Self::ENTITY_POINT_VALUE => "ENTITY_POINT",
            Self::ON_ENTITY_VALUE => "ON_ENTITY",
            Self::INTERSECTION_VALUE => "INTERSECTION",
            Self::MIDPOINT_VALUE => "MIDPOINT",
            Self::EQUIDISTANT_VALUE => "EQUIDISTANT",
            Self::TANGENT_VALUE => "TANGENT",
            Self::CENTER_VALUE => "CENTER",
            _ => "UNKNOWN",
        }
    }
}

/// Display style of a snap line kind.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapLineStyle {
    pub color: [f64; 4],
    pub dash: Option<Vec<f64>>,
    pub line_width: f64,
}

fn style(
    color: [f64; 4],
    dash: Option<Vec<f64>>,
    line_width: f64,
) -> SnapLineStyle {
    SnapLineStyle {
        color,
        dash,
        line_width,
    }
}

/// The canonical style per snap kind; the single source both the
/// Python-facing style table and lookups read.
pub fn style_for(kind: SnapLineType) -> SnapLineStyle {
    match kind.0 {
        SnapLineType::ENTITY_POINT_VALUE => {
            style([0.2, 0.6, 1.0, 0.9], Some(vec![8.0, 4.0]), 2.0)
        }
        SnapLineType::ON_ENTITY_VALUE => {
            style([1.0, 0.4, 0.8, 0.9], Some(vec![8.0, 4.0]), 2.0)
        }
        SnapLineType::INTERSECTION_VALUE => {
            style([1.0, 0.2, 0.8, 0.9], Some(vec![8.0, 4.0]), 2.0)
        }
        SnapLineType::MIDPOINT_VALUE => {
            style([0.2, 0.9, 0.3, 0.9], Some(vec![8.0, 4.0]), 2.0)
        }
        SnapLineType::EQUIDISTANT_VALUE => {
            style([1.0, 0.6, 0.2, 0.9], Some(vec![8.0, 4.0]), 2.0)
        }
        SnapLineType::TANGENT_VALUE => {
            style([0.7, 0.3, 0.9, 0.9], Some(vec![8.0, 4.0]), 2.0)
        }
        SnapLineType::CENTER_VALUE => {
            style([1.0, 0.3, 0.3, 0.9], Some(vec![8.0, 4.0]), 2.0)
        }
        _ => style([0.0, 0.6, 1.0, 0.8], None, 1.0),
    }
}

/// All `(kind, style)` pairs, in reference ordinal order.
pub fn style_table() -> Vec<(SnapLineType, SnapLineStyle)> {
    [
        SnapLineType::ENTITY_POINT,
        SnapLineType::ON_ENTITY,
        SnapLineType::INTERSECTION,
        SnapLineType::MIDPOINT,
        SnapLineType::EQUIDISTANT,
        SnapLineType::TANGENT,
        SnapLineType::CENTER,
    ]
    .into_iter()
    .map(|kind| (kind, style_for(kind)))
    .collect()
}

/// A horizontal or vertical snap guide.
#[derive(Clone, Debug)]
pub struct SnapLine {
    pub is_horizontal: bool,
    pub coordinate: f64,
    pub kind: SnapLineType,
    /// The id of the entity the snap came from, when known.
    pub source: Option<u64>,
}

/// A snap target point.
#[derive(Clone, Debug)]
pub struct SnapPoint {
    pub x: f64,
    pub y: f64,
    pub kind: SnapLineType,
    /// The id of the entity the snap came from, when known.
    pub source: Option<u64>,
    pub spacing: Option<f64>,
    pub is_horizontal: bool,
    pub pattern_coords: Option<Vec<f64>>,
    pub axis_coord: Option<f64>,
}

/// What the host is currently dragging; the producers filter their
/// candidates against it.
#[derive(Clone, Debug, Default)]
pub struct SnapContext {
    pub dragged_point_ids: Vec<u64>,
    pub dragged_entity_ids: Vec<u64>,
    pub preview_entity_ids: Vec<u64>,
    pub suppress_coincident_points: bool,
}

impl SnapContext {
    pub fn is_point_dragged(&self, point_id: u64) -> bool {
        self.dragged_point_ids.contains(&point_id)
    }

    pub fn is_entity_dragged(&self, entity_id: u64) -> bool {
        self.dragged_entity_ids.contains(&entity_id)
    }

    pub fn is_preview_entity(&self, entity_id: u64) -> bool {
        self.preview_entity_ids.contains(&entity_id)
    }

    /// Whether `(x, y)` sits on a currently dragged point; used to
    /// suppress snap candidates that coincide with the drag itself.
    pub fn coincides_with_dragged(
        &self,
        registry: &crate::sketch::registry::EntityRegistry,
        x: f64,
        y: f64,
        epsilon: f64,
    ) -> bool {
        if !self.suppress_coincident_points {
            return false;
        }
        for pid in &self.dragged_point_ids {
            if let Some(point) = registry.get_point(*pid) {
                if (point.x - x).abs() <= epsilon
                    && (point.y - y).abs() <= epsilon
                {
                    return true;
                }
            }
        }
        false
    }
}
