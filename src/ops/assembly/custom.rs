//! Custom-command assembler: emit user-provided machine-code lines
//! as [`Ops::custom`] commands, one per line.
//!
//! The lines are raw text and reach the encoder verbatim aside from
//! path-variable expansion (G-code encoder). No coordinate
//! interpretation is ever applied; the caller authors the lines in
//! machine space. See `CommandType::Custom` for the command side.

use crate::geo::types::Point3D;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{AssembleCtx, Assembler};
use crate::ops::container::Ops;
use crate::ops::types::ToolPose;

/// Spec for the custom-command assembler.
///
/// Carries the raw (unexpanded) text lines. One [`Ops::custom`]
/// command is emitted per line, in order.
#[derive(Clone, Debug)]
pub struct CustomSpec {
    pub lines: Vec<String>,
}

impl Assembler for CustomSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "custom: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }

        let mut ops = Ops::new();
        for line in &self.lines {
            ops.custom(line);
        }
        ctx.trace.append_ops(&ops);

        ctx.callbacks.report_progress(1.0, "custom: done");
        // Custom commands never move the tool, so the pose is
        // unchanged: zero poses like other geometry-less results.
        Ok(AssemblyMeta {
            start: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
            end: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
        })
    }

    fn is_scalable(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "custom"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
