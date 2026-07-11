//! Debug dumping for Ops.

use std::fmt::Write;

use crate::ops::container::Ops;
use crate::ops::types::{MoveCmd, OpCategory};

impl Ops {
    pub fn dump(&self) {
        print!("{}", self.format_dump());
    }

    pub fn format_dump(&self) -> String {
        let mut out = format!("Ops {{ len: {} }}\n", self.commands.len());
        for (i, node) in self.commands.iter().enumerate() {
            let ct = node.command_type();
            write!(out, "  [{}] {}", i, ct).unwrap();
            if let OpCategory::Moving { end, cmd } = &node.category {
                write!(out, " end=({:.3},{:.3},{:.3})", end.x, end.y, end.z)
                    .unwrap();
                match cmd {
                    MoveCmd::ArcTo { center, cw } => {
                        write!(
                            out,
                            " arc=(i={:.3},j={:.3},cw={})",
                            center.x, center.y, cw
                        )
                        .unwrap();
                    }
                    MoveCmd::BezierTo { control1, control2 } => {
                        write!(
                            out,
                            " bezier=(control1=({:.3},{:.3}),control2=({:.3},{:.3}))",
                            control1.x, control1.y, control2.x, control2.y
                        )
                        .unwrap();
                    }
                    _ => {}
                }
            }
            writeln!(out).unwrap();
        }
        out
    }
}
