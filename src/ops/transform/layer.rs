use crate::geo::types::Point3D;
use crate::ops::container::Ops;
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory};

impl Ops {
    #[allow(clippy::type_complexity)]
    pub fn translate_layers(
        &mut self,
        default_offset: (f64, f64, f64),
        layer_offsets: Option<&[(String, (f64, f64, f64))]>,
    ) {
        let mut active_offset = default_offset;
        let mut in_named_layer = false;

        for node in self.cmds_mut().iter_mut() {
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
                        *e = Point3D::new(e.x - ox, e.y - oy, e.z - oz);
                        // Also update control points. Arc centers are relative, so they don't change.
                        match cmd {
                            MoveCmd::BezierTo(data) => {
                                data.control1 = Point3D::new(
                                    data.control1.x - ox,
                                    data.control1.y - oy,
                                    data.control1.z - oz,
                                );
                                data.control2 = Point3D::new(
                                    data.control2.x - ox,
                                    data.control2.y - oy,
                                    data.control2.z - oz,
                                );
                            }
                            MoveCmd::QuadraticBezierTo { control } => {
                                *control = Point3D::new(
                                    control.x - ox,
                                    control.y - oy,
                                    control.z - oz,
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
