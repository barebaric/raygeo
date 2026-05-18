use super::container::Ops;
use super::enums::{CommandCategory, CommandType};

impl Ops {
    pub fn translate_layers(
        &mut self,
        default_offset: (f64, f64, f64),
        layer_offsets: Option<&[(String, (f64, f64, f64))]>,
    ) {
        let mut active_offset = default_offset;
        let mut in_named_layer = false;

        for i in 0..self.soa.len() {
            let ct = self.soa.command_type(i);
            if ct == CommandType::LayerStart {
                if let Some(offsets) = layer_offsets {
                    let uid = self.soa.layer_uid(i);
                    let mut found = false;
                    for (key, offset) in offsets {
                        if key.as_str() == uid {
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
                continue;
            }

            if ct == CommandType::LayerEnd {
                active_offset = default_offset;
                in_named_layer = false;
                continue;
            }

            if !in_named_layer {
                active_offset = default_offset;
            }

            if self.soa.category(i) == CommandCategory::Moving {
                let end = self.soa.endpoint(i);
                let (ox, oy, oz) = active_offset;
                if ox != 0.0 || oy != 0.0 || oz != 0.0 {
                    self.soa
                        .set_endpoint(i, (end.0 - ox, end.1 - oy, end.2 - oz));
                }
            }
        }

        self.invalidate_time_cache();
    }
}
