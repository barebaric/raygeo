use crate::ops::transform::apply::{Phase, Transformer};
use crate::ops::Ops;

/// Parameters for the [`apply_multipass`] transformer.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiPassSpec {
    /// Total number of passes (must be >= 1).
    pub passes: u32,
    /// Z distance to move down after each pass.
    pub z_step_down: f64,
}

impl Transformer for MultiPassSpec {
    fn phase(&self) -> Phase {
        Phase::PostProcessing
    }

    fn apply(&self, ops: &mut Ops) {
        apply_multipass(ops, self.passes, self.z_step_down);
    }

    fn name(&self) -> &'static str {
        "multipass"
    }
}

/// Repeats the ops sequence multiple times, optionally translating each
/// subsequent pass down the Z axis.
///
/// If `passes <= 1` and `z_step_down == 0.0`, this is a no-op.
/// If the ops are empty, this is a no-op.
pub fn apply_multipass(ops: &mut Ops, passes: u32, z_step_down: f64) {
    if passes <= 1 && z_step_down == 0.0 {
        return;
    }
    if ops.is_empty() {
        return;
    }

    let original = ops.copy();

    for i in 1..passes {
        let mut pass_ops = original.copy();
        if z_step_down != 0.0 {
            let z_offset = (i as f64) * z_step_down;
            pass_ops.translate(0.0, 0.0, -z_offset.abs());
        }
        ops.extend(&pass_ops);
    }
}
