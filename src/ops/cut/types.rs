use crate::types::Point3D;

/// Position and heading of the cutting tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolPose {
    pub pos: Point3D,
    pub heading: f64,
}
