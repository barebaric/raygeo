//! Convert between Ops and Geometry.

use crate::geo::geometry::Geometry;
use crate::ops::container::Ops;
use crate::ops::types::{MoveCmd, OpCategory};

impl Ops {
    pub fn from_geometry(
        geometry: &Geometry,
    ) -> Result<Self, crate::RaygeoError> {
        let mut ops = Ops::new();
        if geometry.data.is_empty() {
            ops.last_move_to = geometry.last_move_to;
            return Ok(ops);
        }

        for cmd in &geometry.data {
            match cmd {
                crate::Command::Move { end } => {
                    ops.move_to(end.x, end.y, end.z, None);
                }
                crate::Command::Line { end } => {
                    ops.line_to(end.x, end.y, end.z, None);
                }
                crate::Command::Arc {
                    end,
                    center_offset,
                    normal,
                } => {
                    let clockwise = normal.z < 0.0;
                    ops.arc_to(
                        end.x,
                        end.y,
                        center_offset.x,
                        center_offset.y,
                        clockwise,
                        end.z,
                        None,
                    );
                }
                crate::Command::Bezier {
                    end,
                    control1,
                    control2,
                } => {
                    ops.bezier_to(*control1, *control2, *end, None);
                }
            }
        }
        ops.last_move_to = geometry.last_move_to;
        Ok(ops)
    }

    pub fn to_geometry(&self) -> Geometry {
        let mut geo = Geometry::new();
        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                match cmd {
                    MoveCmd::MoveTo => {
                        geo.move_to(end.x, end.y, end.z);
                    }
                    MoveCmd::LineTo => {
                        geo.line_to(end.x, end.y, end.z);
                    }
                    MoveCmd::ArcTo { center, cw } => {
                        geo.arc_to(
                            end.x, end.y, center.x, center.y, *cw, end.z,
                        );
                    }
                    MoveCmd::BezierTo { control1, control2 } => {
                        geo.bezier_to(*control1, *control2, *end);
                    }
                    _ => {}
                }
            }
        }
        geo
    }
}
