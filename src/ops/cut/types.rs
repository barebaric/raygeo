use crate::types::Point;

/// Position and heading of the cutting tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolPose {
    pub pos: Point,
    pub heading: f64,
}
