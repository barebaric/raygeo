use super::container::Ops;
use super::types::{MarkerCmd, MoveCmd, OpCategory};

impl Ops {
    #[allow(clippy::type_complexity)]
    pub fn translate_layers(
        &mut self,
        default_offset: (f64, f64, f64),
        layer_offsets: Option<&[(String, (f64, f64, f64))]>,
    ) {
        let mut active_offset = default_offset;
        let mut in_named_layer = false;

        for node in &mut self.commands {
            match &mut node.category {
                OpCategory::Marker(MarkerCmd::LayerStart(uid)) => {
                    if let Some(offsets) = layer_offsets {
                        let mut found = false;
                        for (key, offset) in offsets {
                            if key.as_str() == uid.as_ref() {
                                active_offset = *offset;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            active_offset = default_offset;
                        }
                    } else {
                        active_offset = default_offset;
                    }
                    in_named_layer = true;
                }
                OpCategory::Marker(MarkerCmd::LayerEnd(_)) => {
                    active_offset = default_offset;
                    in_named_layer = false;
                }
                OpCategory::Moving { end: e, cmd } => {
                    if !in_named_layer {
                        active_offset = default_offset;
                    }
                    let (ox, oy, oz) = active_offset;
                    if ox != 0.0 || oy != 0.0 || oz != 0.0 {
                        *e = (e.0 - ox, e.1 - oy, e.2 - oz);
                        // Also update control points. Arc centers are relative, so they don't change.
                        match cmd {
                            MoveCmd::BezierTo { c1, c2 } => {
                                *c1 = (c1.0 - ox, c1.1 - oy, c1.2 - oz);
                                *c2 = (c2.0 - ox, c2.1 - oy, c2.2 - oz);
                            }
                            MoveCmd::QuadraticBezierTo { control } => {
                                *control = (
                                    control.0 - ox,
                                    control.1 - oy,
                                    control.2 - oz,
                                );
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    if !in_named_layer {
                        active_offset = default_offset;
                    }
                }
            }
        }

        self.invalidate_time_cache();
    }
}
