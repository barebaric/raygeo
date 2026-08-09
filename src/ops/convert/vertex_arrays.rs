//! Convert Ops into flat GPU-friendly vertex arrays.
//!
//! Produces four flat `Vec<f32>` buffers (powered vertices, powered
//! colors, travel vertices, zero-power vertices) in a single Rust
//! traversal, eliminating per-command PyO3 round-trips.

use crate::geo::shape::arc::linearize_arc;
use crate::geo::shape::bezier::linearize_bezier_segment;
use crate::geo::types::Point3D;
use crate::image::scan::extract_zero_power_segments;
use crate::ops::container::Ops;
use crate::ops::convert::{EncodeCtx, EncodeOutput, Encoder};
use crate::ops::enums::CommandType;
use crate::ops::types::{MoveCmd, OpCategory, OpNode, StateCmd};

/// Output buffers for vertex encoding.
#[derive(Debug, Clone)]
pub struct VertexArrays {
    pub powered_vertices: Vec<f32>,
    pub powered_colors: Vec<f32>,
    pub travel_vertices: Vec<f32>,
    pub zero_power_vertices: Vec<f32>,
}

fn push_segment(vertices: &mut Vec<f32>, start: Point3D, end: Point3D) {
    vertices.push(start.x as f32);
    vertices.push(start.y as f32);
    vertices.push(start.z as f32);
    vertices.push(end.x as f32);
    vertices.push(end.y as f32);
    vertices.push(end.z as f32);
}

fn push_color(colors: &mut Vec<f32>, r: f32, g: f32, b: f32) {
    colors.push(r);
    colors.push(g);
    colors.push(b);
    colors.push(1.0);
    colors.push(r);
    colors.push(g);
    colors.push(b);
    colors.push(1.0);
}

fn power_to_grayscale(power: f64) -> (f32, f32, f32) {
    let byte = (power * 255.0).min(255.0) as u8;
    let g = byte as f32 / 255.0;
    (g, g, g)
}

fn linearize_bezier_for_vertex(node: &OpNode, start: Point3D) -> Vec<Point3D> {
    if let OpCategory::Moving {
        end,
        cmd: MoveCmd::BezierTo { control1, control2 },
    } = &node.category
    {
        linearize_bezier_segment(start, *control1, *control2, *end, None)
    } else {
        Vec::new()
    }
}

impl Ops {
    /// Encode all commands into GPU-friendly vertex arrays.
    ///
    /// Returns four flat `Vec<f32>` buffers:
    /// `(powered_vertices, powered_colors, travel_vertices,
    /// zero_power_vertices)`.
    ///
    /// Powered vertices and colors are paired (2 vertices + 2 colors
    /// per segment).  Travel and zero-power vertices are also paired
    /// (2 vertices per segment).  All vertex data is 3-component
    /// (x, y, z); colors are 4-component (r, g, b, a).
    pub fn to_vertex_arrays(&self) -> VertexArrays {
        let mut powered_v = Vec::new();
        let mut powered_c = Vec::new();
        let mut travel_v = Vec::new();
        let mut zero_power_v = Vec::new();

        let mut current_power: f64 = 0.0;
        let mut current_pos = Point3D::new(0.0, 0.0, 0.0);
        let mut is_initial_position = true;

        let mut arc_buf: Vec<(Point3D, Point3D)> = Vec::new();

        for node in self.commands.iter() {
            let ct = node.command_type();

            if ct == CommandType::SetPower {
                if let OpCategory::State(StateCmd::SetPower(p)) = &node.category
                {
                    current_power = *p;
                }
                continue;
            }

            if !node.is_moving() {
                continue;
            }

            let end = node.end_point();

            match ct {
                CommandType::MoveTo => {
                    if !is_initial_position {
                        push_segment(&mut travel_v, current_pos, end);
                    }
                    current_pos = end;
                    is_initial_position = false;
                }

                CommandType::LineTo => {
                    if current_power > 0.0 {
                        let (r, g, b) = power_to_grayscale(current_power);
                        push_segment(&mut powered_v, current_pos, end);
                        push_color(&mut powered_c, r, g, b);
                    } else {
                        push_segment(&mut zero_power_v, current_pos, end);
                    }
                    current_pos = end;
                    is_initial_position = false;
                }

                CommandType::ArcTo => {
                    let (center, cw) = if let OpCategory::Moving {
                        cmd: MoveCmd::ArcTo { center, cw },
                        ..
                    } = &node.category
                    {
                        (*center, *cw)
                    } else {
                        continue;
                    };
                    let normal = if cw {
                        Point3D::new(0.0, 0.0, -1.0)
                    } else {
                        Point3D::new(0.0, 0.0, 1.0)
                    };
                    linearize_arc(
                        end,
                        Point3D::new(center.x, center.y, 0.0),
                        normal,
                        current_pos,
                        0.1,
                        &mut arc_buf,
                    );
                    if current_power > 0.0 {
                        let (r, g, b) = power_to_grayscale(current_power);
                        for (seg_start, seg_end) in &arc_buf {
                            push_segment(&mut powered_v, *seg_start, *seg_end);
                            push_color(&mut powered_c, r, g, b);
                        }
                    } else {
                        for (seg_start, seg_end) in &arc_buf {
                            push_segment(
                                &mut zero_power_v,
                                *seg_start,
                                *seg_end,
                            );
                        }
                    }
                    current_pos = end;
                    is_initial_position = false;
                }

                CommandType::BezierTo => {
                    let polyline =
                        linearize_bezier_for_vertex(node, current_pos);
                    if current_power > 0.0 {
                        let (r, g, b) = power_to_grayscale(current_power);
                        for j in 0..polyline.len().saturating_sub(1) {
                            push_segment(
                                &mut powered_v,
                                polyline[j],
                                polyline[j + 1],
                            );
                            push_color(&mut powered_c, r, g, b);
                        }
                    } else {
                        for j in 0..polyline.len().saturating_sub(1) {
                            push_segment(
                                &mut zero_power_v,
                                polyline[j],
                                polyline[j + 1],
                            );
                        }
                    }
                    current_pos = end;
                    is_initial_position = false;
                }

                CommandType::ScanLine => {
                    let power_values: &[u8] = if let OpCategory::Moving {
                        cmd: MoveCmd::ScanLine { power_values },
                        ..
                    } = &node.category
                    {
                        power_values
                    } else {
                        continue;
                    };
                    let segments = extract_zero_power_segments(
                        (current_pos.x, current_pos.y, current_pos.z),
                        (end.x, end.y, end.z),
                        power_values,
                    );
                    zero_power_v.extend(segments);
                    current_pos = end;
                    is_initial_position = false;
                }

                _ => {}
            }
        }

        VertexArrays {
            powered_vertices: powered_v,
            powered_colors: powered_c,
            travel_vertices: travel_v,
            zero_power_vertices: zero_power_v,
        }
    }
}

/// Spec for the vertex-array encoder.
///
/// Calls [`Ops::to_vertex_arrays`] on the upstream ops.
#[derive(Clone, Debug, Default)]
pub struct VertexSpec;

impl Encoder for VertexSpec {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        ctx.callbacks.report_progress(0.0, "vertex: encode");
        let va = ctx.ops.to_vertex_arrays();
        ctx.callbacks.report_progress(1.0, "vertex: done");
        Ok(EncodeOutput::VertexArrays(va))
    }

    fn name(&self) -> &str {
        "vertex"
    }
}
