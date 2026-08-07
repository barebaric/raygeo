//! Scene compiler: convert Ops into GPU-ready 3D vertex data.
//!
//! Produces flat `Vec<f32>` buffers (powered vertices, power values,
//! laser indices, travel vertices, zero-power vertices, scanline
//! overlay data) plus per-command offset arrays in a single Rust
//! traversal, eliminating per-command PyO3 round-trips.
//!
//! For rotary layers, vertex Y-coordinates are stored in degrees
//! during the walk and wrapped onto a cylinder surface during
//! finalization via [`transform_to_cylinder`].

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::geo::algo::cylindrical::{
    transform_to_cylinder, OVERLAY_RADIAL_OFFSET,
};
use crate::geo::shape::arc::linearize_arc;
use crate::geo::shape::bezier::linearize_bezier_segment;
use crate::geo::types::Point3D;
use crate::image::scan::{
    extract_overlay_segments, extract_zero_power_segments,
};
use crate::ops::axis::Axis;
use crate::ops::container::Ops;
use crate::ops::convert::{EncodeCtx, EncodeOutput, Encoder};
use crate::ops::enums::CommandType;
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};

const Z_OFFSET_NON_POWERED: f32 = 0.01;
const ARC_RESOLUTION: f64 = 0.1;
const BEZIER_TOLERANCE: f64 = 0.1;

// ------------------------------------------------------------------
// Public structs
// ------------------------------------------------------------------

/// Per-layer rendering configuration.
#[derive(Clone, Debug, Default)]
pub struct LayerConfig {
    pub rotary_enabled: bool,
    pub rotary_diameter: f64,
    pub axis_position: f64,
    pub reverse: bool,
}

/// Specification for the 3D scene encoder.
///
/// Carries the world-to-visual transform and per-layer rendering
/// configuration. Implements [`Encoder`] so it can be driven through
/// the same trait as other converters.
#[derive(Clone, Debug)]
pub struct SceneSpec {
    pub world_to_visual: [[f32; 4]; 4],
    pub layer_configs: HashMap<String, LayerConfig>,
}

/// Metadata for one layer, used by the caller for texture generation.
#[derive(Clone, Debug)]
pub struct LayerInfo {
    pub cmd_start: usize,
    pub cmd_end: usize,
    pub is_rotary: bool,
    pub diameter: f64,
    pub has_scanlines: bool,
    pub scanline_laser: String,
    pub activation_cmd_idx: usize,
    pub axis_position: f64,
    pub reverse: bool,
}

/// Vertex + overlay data for one rendering group (flat or rotary).
#[derive(Clone, Debug, Default)]
pub struct VertexGroupData {
    pub is_rotary: bool,
    pub powered_verts: Vec<f32>,
    pub powered_attrib: Vec<f32>,
    pub travel_verts: Vec<f32>,
    pub zero_power_verts: Vec<f32>,
    pub powered_cmd_offsets: Vec<i32>,
    pub travel_cmd_offsets: Vec<i32>,
    pub overlay_positions: Vec<f32>,
    pub overlay_attrib: Vec<f32>,
    pub overlay_cmd_offsets: Vec<i32>,
}

/// Complete output of [`Ops::compile_scene_3d`].
#[derive(Clone, Debug)]
pub struct CompiledSceneData {
    pub groups: Vec<VertexGroupData>,
    pub laser_uid_order: Vec<String>,
    pub layer_infos: Vec<LayerInfo>,
}

// ------------------------------------------------------------------
// Private helpers: rotary coordinate conversion
// ------------------------------------------------------------------

pub(crate) fn find_degrees_from_extra_pub(
    extra: Option<&[(Axis, f64)]>,
) -> Option<f64> {
    find_degrees_from_extra(extra)
}

fn find_degrees_from_extra(extra: Option<&[(Axis, f64)]>) -> Option<f64> {
    let axes = extra?;
    for &target in &[Axis::A, Axis::B, Axis::C, Axis::U, Axis::Y] {
        for &(axis, val) in axes {
            if axis == target {
                return Some(val);
            }
        }
    }
    None
}

fn degrees_to_mu(degrees: f64, diameter: f64, reverse: bool) -> f64 {
    if diameter <= 0.0 {
        return degrees;
    }
    let sign = if reverse { -1.0 } else { 1.0 };
    degrees * PI * diameter / 360.0 * sign
}

fn mu_to_degrees(mu: f64, diameter: f64, reverse: bool) -> f64 {
    if diameter <= 0.0 {
        return 0.0;
    }
    let circumference = diameter * PI;
    let deg = (mu / circumference) * 360.0;
    if reverse {
        -deg
    } else {
        deg
    }
}

fn visual_end_point(node: &OpNode) -> Point3D {
    let end = node.end_point();
    match find_degrees_from_extra(node.extra_axes()) {
        Some(degrees) => Point3D::new(end.x, degrees, end.z),
        None => end,
    }
}

fn reconstruct_mu_pos(node: &OpNode, diameter: f64, reverse: bool) -> Point3D {
    let end = node.end_point();
    match find_degrees_from_extra(node.extra_axes()) {
        Some(degrees) => {
            let mu = degrees_to_mu(degrees, diameter, reverse);
            Point3D::new(end.x, mu, end.z)
        }
        None => end,
    }
}

fn mu_to_visual(p: Point3D, diameter: f64, reverse: bool) -> Point3D {
    Point3D::new(p.x, mu_to_degrees(p.y, diameter, reverse), p.z)
}

// ------------------------------------------------------------------
// Private helpers: vertex / color packing
// ------------------------------------------------------------------

fn push_vert(vec: &mut Vec<f32>, p: Point3D) {
    vec.push(p.x as f32);
    vec.push(p.y as f32);
    vec.push(p.z as f32);
}

fn push_segment(vec: &mut Vec<f32>, start: Point3D, end: Point3D) {
    push_vert(vec, start);
    push_vert(vec, end);
}

fn apply_transform(verts: &mut [f32], t: &[[f32; 4]; 4]) {
    let n = verts.len() / 3;
    for i in 0..n {
        let x = verts[i * 3];
        let y = verts[i * 3 + 1];
        let z = verts[i * 3 + 2];
        verts[i * 3] = t[0][0] * x + t[0][1] * y + t[0][2] * z + t[0][3];
        verts[i * 3 + 1] = t[1][0] * x + t[1][1] * y + t[1][2] * z + t[1][3];
        verts[i * 3 + 2] = t[2][0] * x + t[2][1] * y + t[2][2] * z + t[2][3];
    }
}

fn add_z_offset(verts: &mut [f32]) {
    let n = verts.len() / 3;
    for i in 0..n {
        verts[i * 3 + 2] += Z_OFFSET_NON_POWERED;
    }
}

// ------------------------------------------------------------------
// Rotary segment tracking
// ------------------------------------------------------------------

#[allow(dead_code)]
struct RotarySegStart {
    pv_start: usize,
    tv_start: usize,
    zpv_start: usize,
    ov_start: usize,
    cmd_start: usize,
}

struct RotarySeg {
    pv_start: usize,
    pv_end: usize,
    tv_start: usize,
    tv_end: usize,
    zpv_start: usize,
    zpv_end: usize,
    ov_start: usize,
    ov_end: usize,
    diameter: f64,
}

// ------------------------------------------------------------------
// Accumulator
// ------------------------------------------------------------------

struct Accumulator {
    is_rotary: bool,
    pv: Vec<f32>,
    pva: Vec<f32>,
    tv: Vec<f32>,
    zpv: Vec<f32>,
    ov_pos: Vec<f32>,
    ov_attrib: Vec<f32>,
    pv_cum: usize,
    tv_cum: usize,
    ov_cum: usize,
    pv_off: Vec<i32>,
    tv_off: Vec<i32>,
    ov_off: Vec<i32>,
    rotary_segments: Vec<RotarySeg>,
    current_rotary_start: Option<RotarySegStart>,
    diameter: f64,
    reverse: bool,
    axis_position: f64,
}

impl Accumulator {
    fn new(total_cmds: usize, is_rotary: bool) -> Self {
        Accumulator {
            is_rotary,
            pv: Vec::new(),
            pva: Vec::new(),
            tv: Vec::new(),
            zpv: Vec::new(),
            ov_pos: Vec::new(),
            ov_attrib: Vec::new(),
            pv_cum: 0,
            tv_cum: 0,
            ov_cum: 0,
            pv_off: vec![0; total_cmds + 1],
            tv_off: vec![0; total_cmds + 1],
            ov_off: vec![0; total_cmds + 1],
            rotary_segments: Vec::new(),
            current_rotary_start: None,
            diameter: 0.0,
            reverse: false,
            axis_position: 0.0,
        }
    }

    fn record_offset(&mut self, cmd_idx: usize) {
        self.pv_off[cmd_idx + 1] = self.pv_cum as i32;
        self.tv_off[cmd_idx + 1] = self.tv_cum as i32;
        self.ov_off[cmd_idx + 1] = self.ov_cum as i32;
    }

    fn begin_rotary_segment(&mut self, cmd_idx: usize) {
        self.current_rotary_start = Some(RotarySegStart {
            pv_start: self.pv.len() / 3,
            tv_start: self.tv.len() / 3,
            zpv_start: self.zpv.len() / 3,
            ov_start: self.ov_pos.len() / 3,
            cmd_start: cmd_idx + 1,
        });
    }

    fn end_rotary_segment(&mut self, diameter: f64, _cmd_idx: usize) {
        if let Some(s) = self.current_rotary_start.take() {
            self.rotary_segments.push(RotarySeg {
                pv_start: s.pv_start,
                pv_end: self.pv.len() / 3,
                tv_start: s.tv_start,
                tv_end: self.tv.len() / 3,
                zpv_start: s.zpv_start,
                zpv_end: self.zpv.len() / 3,
                ov_start: s.ov_start,
                ov_end: self.ov_pos.len() / 3,
                diameter,
            });
        }
    }

    fn has_content(&self) -> bool {
        !self.pv.is_empty()
            || !self.tv.is_empty()
            || !self.zpv.is_empty()
            || !self.ov_pos.is_empty()
    }
}

// ------------------------------------------------------------------
// Offset remapping after cylinder subdivision
// ------------------------------------------------------------------

fn remap_offsets(
    offsets: &[i32],
    expansions: &[(usize, Vec<i32>)],
) -> Vec<i32> {
    if expansions.is_empty() {
        return offsets.to_vec();
    }
    offsets
        .iter()
        .map(|&pre_count| {
            let mut mapped = pre_count;
            for &(seg_start, ref cum_subs) in expansions {
                let num_input_pairs = cum_subs.len() - 1;
                let num_input_verts = num_input_pairs * 2;
                let pc = pre_count as usize;
                if pc <= seg_start {
                    continue;
                }
                if pc >= seg_start + num_input_verts {
                    let total_extra = cum_subs[cum_subs.len() - 1] * 2
                        - num_input_verts as i32;
                    mapped += total_extra;
                } else {
                    let vert_offset = pc - seg_start;
                    let pair_idx = (vert_offset / 2).min(num_input_pairs);
                    let extra_verts = vert_offset % 2;
                    mapped = (seg_start
                        + cum_subs[pair_idx] as usize * 2
                        + extra_verts) as i32;
                    break;
                }
            }
            mapped
        })
        .collect()
}

// ------------------------------------------------------------------
// Cylinder wrapping for rotary accumulators
// ------------------------------------------------------------------

/// Result of wrapping rotary vertex data onto a cylinder.
struct CylinderWrapped {
    pv: Vec<f32>,
    pva: Vec<f32>,
    tv: Vec<f32>,
    zpv: Vec<f32>,
    ov_pos: Vec<f32>,
    ov_attrib: Vec<f32>,
    pv_expansion: Vec<(usize, Vec<i32>)>,
    ov_expansion: Vec<(usize, Vec<i32>)>,
}

fn finalize_rotary_cylinder(acc: &Accumulator) -> Option<CylinderWrapped> {
    let mut exp_pv: Vec<Vec<f32>> = Vec::new();
    let mut exp_pva: Vec<Vec<f32>> = Vec::new();
    let mut exp_tv: Vec<Vec<f32>> = Vec::new();
    let mut exp_zpv: Vec<Vec<f32>> = Vec::new();
    let mut exp_ov_pos: Vec<Vec<f32>> = Vec::new();
    let mut exp_ov_attrib: Vec<Vec<f32>> = Vec::new();
    let mut pv_expansion: Vec<(usize, Vec<i32>)> = Vec::new();
    let mut ov_expansion: Vec<(usize, Vec<i32>)> = Vec::new();

    for seg in &acc.rotary_segments {
        let d = seg.diameter;
        if d <= 0.0 {
            continue;
        }

        if seg.pv_end > seg.pv_start {
            let slice = &acc.pv[seg.pv_start * 3..seg.pv_end * 3];
            let colors = &acc.pva[seg.pv_start * 4..seg.pv_end * 4];
            let (wv, wc, cum) =
                transform_to_cylinder(slice, d, Some(colors), true, 0.0);
            exp_pv.push(wv);
            exp_pva.push(wc.unwrap_or_default());
            pv_expansion.push((seg.pv_start, cum));
        }

        if seg.tv_end > seg.tv_start {
            let slice = &acc.tv[seg.tv_start * 3..seg.tv_end * 3];
            let (wv, _, _) = transform_to_cylinder(slice, d, None, true, 0.0);
            exp_tv.push(wv);
        }

        if seg.zpv_end > seg.zpv_start {
            let slice = &acc.zpv[seg.zpv_start * 3..seg.zpv_end * 3];
            let (wv, _, _) = transform_to_cylinder(slice, d, None, true, 0.0);
            exp_zpv.push(wv);
        }

        if seg.ov_end > seg.ov_start {
            let slice = &acc.ov_pos[seg.ov_start * 3..seg.ov_end * 3];
            let colors = &acc.ov_attrib[seg.ov_start * 4..seg.ov_end * 4];
            let (wv, wc, cum) = transform_to_cylinder(
                slice,
                d,
                Some(colors),
                true,
                OVERLAY_RADIAL_OFFSET,
            );
            exp_ov_pos.push(wv);
            exp_ov_attrib.push(wc.unwrap_or_default());
            ov_expansion.push((seg.ov_start, cum));
        }
    }

    if exp_pv.is_empty()
        && exp_tv.is_empty()
        && exp_zpv.is_empty()
        && exp_ov_pos.is_empty()
    {
        return None;
    }

    let pv: Vec<f32> = exp_pv.into_iter().flatten().collect();
    let pva: Vec<f32> = exp_pva.into_iter().flatten().collect();
    let tv: Vec<f32> = exp_tv.into_iter().flatten().collect();
    let zpv: Vec<f32> = exp_zpv.into_iter().flatten().collect();
    let ov_pos: Vec<f32> = exp_ov_pos.into_iter().flatten().collect();
    let ov_attrib: Vec<f32> = exp_ov_attrib.into_iter().flatten().collect();

    Some(CylinderWrapped {
        pv,
        pva,
        tv,
        zpv,
        ov_pos,
        ov_attrib,
        pv_expansion,
        ov_expansion,
    })
}

// ------------------------------------------------------------------
// Core compilation
// ------------------------------------------------------------------

impl Ops {
    /// Compile all commands into GPU-ready 3D scene data.
    ///
    /// Walks the command list once, accumulating vertex data for
    /// flat and rotary rendering groups. For rotary layers, vertices
    /// are stored in degree coordinates during the walk and wrapped
    /// onto a cylinder surface during finalization.
    pub fn compile_scene_3d(&self, spec: &SceneSpec) -> CompiledSceneData {
        let total = self.commands.len();

        let mut accs = [
            Accumulator::new(total, false),
            Accumulator::new(total, true),
        ];

        let mut current_power: f64 = 0.0;
        let mut current_pos = Point3D::new(0.0, 0.0, 0.0);
        let mut current_pos_vis = Point3D::new(0.0, 0.0, 0.0);
        let mut is_initial = true;
        let mut current_laser_uid: String = String::new();
        let mut current_laser_index: i32 = 0;
        let mut laser_uid_order: Vec<String> = Vec::new();
        let mut is_rotary = false;
        let mut rotary_diameter: f64 = 0.0;
        let mut has_mapped_data = false;
        let mut layer_infos: Vec<LayerInfo> = Vec::new();
        let mut current_layer_start: Option<usize> = None;
        let mut current_layer_has_scanlines = false;
        let mut current_layer_scanline_laser: String = String::new();

        let mut arc_buf: Vec<(Point3D, Point3D)> = Vec::new();

        for (i, node) in self.commands.iter().enumerate() {
            let ct = node.command_type();

            match ct {
                CommandType::LayerStart => {
                    let layer_uid: String = if let OpCategory::Marker(
                        MarkerCmd::LayerStart(ref uid),
                    ) = node.category
                    {
                        uid.to_string()
                    } else {
                        String::new()
                    };

                    let cfg = spec.layer_configs.get(&layer_uid);
                    is_rotary = cfg.map(|c| c.rotary_enabled).unwrap_or(false);
                    rotary_diameter =
                        cfg.map(|c| c.rotary_diameter).unwrap_or(0.0);
                    has_mapped_data = is_rotary;

                    let acc = &mut accs[is_rotary as usize];
                    acc.axis_position =
                        cfg.map(|c| c.axis_position).unwrap_or(0.0);
                    acc.reverse = cfg.map(|c| c.reverse).unwrap_or(false);
                    acc.diameter = rotary_diameter;

                    if is_rotary && rotary_diameter > 0.0 {
                        acc.begin_rotary_segment(i);
                    }

                    current_layer_start = Some(i + 1);
                    current_layer_has_scanlines = false;
                    current_layer_scanline_laser.clear();
                }

                CommandType::LayerEnd => {
                    if let Some(cls) = current_layer_start {
                        let acc = &accs[is_rotary as usize];
                        layer_infos.push(LayerInfo {
                            cmd_start: cls,
                            cmd_end: i,
                            is_rotary,
                            diameter: rotary_diameter,
                            has_scanlines: current_layer_has_scanlines,
                            scanline_laser: current_layer_scanline_laser
                                .clone(),
                            activation_cmd_idx: i,
                            axis_position: acc.axis_position,
                            reverse: acc.reverse,
                        });
                    }
                    {
                        let acc = &mut accs[is_rotary as usize];
                        if acc.current_rotary_start.is_some() {
                            acc.end_rotary_segment(rotary_diameter, i);
                        }
                    }
                    current_layer_start = None;
                }

                CommandType::SetHead => {
                    if let OpCategory::State(StateCmd::SetHead(ref uid)) =
                        node.category
                    {
                        current_laser_uid = uid.to_string();
                        let pos = laser_uid_order
                            .iter()
                            .position(|s| s == &current_laser_uid);
                        current_laser_index = match pos {
                            Some(idx) => idx as i32,
                            None => {
                                laser_uid_order.push(current_laser_uid.clone());
                                (laser_uid_order.len() - 1) as i32
                            }
                        };
                    }
                }

                CommandType::SetPower => {
                    if let OpCategory::State(StateCmd::SetPower(p)) =
                        node.category
                    {
                        current_power = p;
                    }
                }

                CommandType::MoveTo => {
                    let vis_end = visual_end_point(node);
                    let acc = &mut accs[is_rotary as usize];
                    if !is_initial {
                        push_segment(&mut acc.tv, current_pos_vis, vis_end);
                        acc.tv_cum += 2;
                    }
                    current_pos_vis = vis_end;
                    if has_mapped_data && is_rotary {
                        current_pos =
                            reconstruct_mu_pos(node, acc.diameter, acc.reverse);
                    } else {
                        current_pos = node.end_point();
                    }
                    is_initial = false;
                }

                CommandType::LineTo => {
                    let vis_end = visual_end_point(node);
                    let acc = &mut accs[is_rotary as usize];
                    if current_power > 0.0 {
                        push_segment(&mut acc.pv, current_pos_vis, vis_end);
                        let p = current_power as f32;
                        let li = current_laser_index as f32;
                        acc.pva.extend([p, li, 0.0, 1.0, p, li, 0.0, 1.0]);
                        acc.pv_cum += 2;
                    } else {
                        push_segment(&mut acc.zpv, current_pos_vis, vis_end);
                    }
                    current_pos_vis = vis_end;
                    if has_mapped_data && is_rotary {
                        current_pos =
                            reconstruct_mu_pos(node, acc.diameter, acc.reverse);
                    } else {
                        current_pos = node.end_point();
                    }
                    is_initial = false;
                }

                CommandType::ArcTo => {
                    let (end, center, cw) = if let OpCategory::Moving {
                        end,
                        cmd: MoveCmd::ArcTo { center, cw },
                    } = &node.category
                    {
                        (*end, *center, *cw)
                    } else {
                        continue;
                    };

                    let acc = &mut accs[is_rotary as usize];
                    let normal = if cw {
                        Point3D::new(0.0, 0.0, -1.0)
                    } else {
                        Point3D::new(0.0, 0.0, 1.0)
                    };

                    if has_mapped_data && is_rotary {
                        let scale =
                            degrees_to_mu(1.0, acc.diameter, acc.reverse);
                        let degrees =
                            find_degrees_from_extra(node.extra_axes());
                        let (mu_end, mu_i, mu_j) = match degrees {
                            Some(deg) => (
                                Point3D::new(end.x, deg * scale, end.z),
                                center.x,
                                center.y * scale,
                            ),
                            None => (end, center.x, center.y),
                        };
                        linearize_arc(
                            mu_end,
                            Point3D::new(mu_i, mu_j, 0.0),
                            normal,
                            current_pos,
                            ARC_RESOLUTION,
                            &mut arc_buf,
                        );
                        let vis_segs: Vec<(Point3D, Point3D)> = arc_buf
                            .iter()
                            .map(|&(s, e)| {
                                (
                                    mu_to_visual(s, acc.diameter, acc.reverse),
                                    mu_to_visual(e, acc.diameter, acc.reverse),
                                )
                            })
                            .collect();
                        if current_power > 0.0 {
                            let p = current_power as f32;
                            let li = current_laser_index as f32;
                            for (ss, se) in &vis_segs {
                                push_segment(&mut acc.pv, *ss, *se);
                                acc.pva
                                    .extend([p, li, 0.0, 1.0, p, li, 0.0, 1.0]);
                            }
                            acc.pv_cum += vis_segs.len() * 2;
                        } else {
                            for (ss, se) in &vis_segs {
                                push_segment(&mut acc.zpv, *ss, *se);
                            }
                        }
                    } else {
                        linearize_arc(
                            end,
                            Point3D::new(center.x, center.y, 0.0),
                            normal,
                            current_pos,
                            ARC_RESOLUTION,
                            &mut arc_buf,
                        );
                        if current_power > 0.0 {
                            let p = current_power as f32;
                            let li = current_laser_index as f32;
                            for &(ss, se) in &arc_buf {
                                push_segment(&mut acc.pv, ss, se);
                                acc.pva
                                    .extend([p, li, 0.0, 1.0, p, li, 0.0, 1.0]);
                            }
                            acc.pv_cum += arc_buf.len() * 2;
                        } else {
                            for &(ss, se) in &arc_buf {
                                push_segment(&mut acc.zpv, ss, se);
                            }
                        }
                    }

                    current_pos_vis = visual_end_point(node);
                    if has_mapped_data && is_rotary {
                        current_pos =
                            reconstruct_mu_pos(node, acc.diameter, acc.reverse);
                    } else {
                        current_pos = end;
                    }
                    is_initial = false;
                }

                CommandType::BezierTo => {
                    let (end, c1, c2) = if let OpCategory::Moving {
                        end,
                        cmd: MoveCmd::BezierTo { control1, control2 },
                    } = &node.category
                    {
                        (*end, *control1, *control2)
                    } else {
                        continue;
                    };

                    let acc = &mut accs[is_rotary as usize];

                    if has_mapped_data && is_rotary {
                        let scale =
                            degrees_to_mu(1.0, acc.diameter, acc.reverse);
                        let degrees =
                            find_degrees_from_extra(node.extra_axes());
                        let (mu_end, mu_c1, mu_c2) = match degrees {
                            Some(deg) => (
                                Point3D::new(end.x, deg * scale, end.z),
                                Point3D::new(c1.x, c1.y * scale, c1.z),
                                Point3D::new(c2.x, c2.y * scale, c2.z),
                            ),
                            None => (end, c1, c2),
                        };
                        let poly = linearize_bezier_segment(
                            current_pos,
                            mu_c1,
                            mu_c2,
                            mu_end,
                            Some(BEZIER_TOLERANCE),
                        );
                        let vis_poly: Vec<Point3D> = poly
                            .iter()
                            .map(|&p| {
                                mu_to_visual(p, acc.diameter, acc.reverse)
                            })
                            .collect();
                        if current_power > 0.0 {
                            let p = current_power as f32;
                            let li = current_laser_index as f32;
                            for j in 0..vis_poly.len().saturating_sub(1) {
                                push_segment(
                                    &mut acc.pv,
                                    vis_poly[j],
                                    vis_poly[j + 1],
                                );
                                acc.pva
                                    .extend([p, li, 0.0, 1.0, p, li, 0.0, 1.0]);
                            }
                            acc.pv_cum +=
                                (vis_poly.len().saturating_sub(1)) * 2;
                        } else {
                            for j in 0..vis_poly.len().saturating_sub(1) {
                                push_segment(
                                    &mut acc.zpv,
                                    vis_poly[j],
                                    vis_poly[j + 1],
                                );
                            }
                        }
                    } else {
                        let poly = linearize_bezier_segment(
                            current_pos,
                            c1,
                            c2,
                            end,
                            Some(BEZIER_TOLERANCE),
                        );
                        if current_power > 0.0 {
                            let p = current_power as f32;
                            let li = current_laser_index as f32;
                            for j in 0..poly.len().saturating_sub(1) {
                                push_segment(&mut acc.pv, poly[j], poly[j + 1]);
                                acc.pva
                                    .extend([p, li, 0.0, 1.0, p, li, 0.0, 1.0]);
                            }
                            acc.pv_cum += (poly.len().saturating_sub(1)) * 2;
                        } else {
                            for j in 0..poly.len().saturating_sub(1) {
                                push_segment(
                                    &mut acc.zpv,
                                    poly[j],
                                    poly[j + 1],
                                );
                            }
                        }
                    }

                    current_pos_vis = visual_end_point(node);
                    if has_mapped_data && is_rotary {
                        current_pos =
                            reconstruct_mu_pos(node, acc.diameter, acc.reverse);
                    } else {
                        current_pos = end;
                    }
                    is_initial = false;
                }

                CommandType::ScanLine => {
                    let (end, power_values) = if let OpCategory::Moving {
                        end,
                        cmd: MoveCmd::ScanLine { power_values },
                    } = &node.category
                    {
                        (*end, power_values.as_ref())
                    } else {
                        continue;
                    };

                    let vis_end = visual_end_point(node);
                    let acc = &mut accs[is_rotary as usize];

                    let zpv = extract_zero_power_segments(
                        (
                            current_pos_vis.x,
                            current_pos_vis.y,
                            current_pos_vis.z,
                        ),
                        (vis_end.x, vis_end.y, vis_end.z),
                        power_values,
                    );
                    acc.zpv.extend(zpv);

                    if !is_initial {
                        let n = extract_overlay_segments(
                            (
                                current_pos_vis.x,
                                current_pos_vis.y,
                                current_pos_vis.z,
                            ),
                            (vis_end.x, vis_end.y, vis_end.z),
                            power_values,
                            current_laser_index,
                            &mut acc.ov_pos,
                            &mut acc.ov_attrib,
                        );
                        acc.ov_cum += n;
                    }

                    current_pos_vis = vis_end;
                    if has_mapped_data && is_rotary {
                        current_pos =
                            reconstruct_mu_pos(node, acc.diameter, acc.reverse);
                    } else {
                        current_pos = end;
                    }
                    is_initial = false;
                    current_layer_has_scanlines = true;
                    if current_layer_scanline_laser.is_empty() {
                        current_layer_scanline_laser =
                            current_laser_uid.clone();
                    }
                }

                _ => {}
            }

            accs[0].record_offset(i);
            accs[1].record_offset(i);
        }

        // Finalize accumulators
        let mut groups = Vec::new();
        for acc in &mut accs {
            if !acc.has_content() {
                continue;
            }
            groups.push(finalize_acc(acc, &spec.world_to_visual));
        }

        CompiledSceneData {
            groups,
            laser_uid_order,
            layer_infos,
        }
    }
}

fn finalize_acc(
    acc: &mut Accumulator,
    world_to_visual: &[[f32; 4]; 4],
) -> VertexGroupData {
    if acc.is_rotary {
        if let Some(w) = finalize_rotary_cylinder(acc) {
            let mut tv = w.tv;
            let mut zpv = w.zpv;
            add_z_offset(&mut tv);
            add_z_offset(&mut zpv);

            let pv_off = if w.pv_expansion.is_empty() {
                acc.pv_off.clone()
            } else {
                remap_offsets(&acc.pv_off, &w.pv_expansion)
            };

            let ov_off = if w.ov_expansion.is_empty() {
                acc.ov_off.clone()
            } else {
                remap_offsets(&acc.ov_off, &w.ov_expansion)
            };

            return VertexGroupData {
                is_rotary: true,
                powered_verts: w.pv,
                powered_attrib: w.pva,
                travel_verts: tv,
                zero_power_verts: zpv,
                powered_cmd_offsets: pv_off,
                travel_cmd_offsets: acc.tv_off.clone(),
                overlay_positions: w.ov_pos,
                overlay_attrib: w.ov_attrib,
                overlay_cmd_offsets: ov_off,
            };
        }
    } else {
        apply_transform(&mut acc.pv, world_to_visual);
        apply_transform(&mut acc.tv, world_to_visual);
        apply_transform(&mut acc.zpv, world_to_visual);
        apply_transform(&mut acc.ov_pos, world_to_visual);
    }

    add_z_offset(&mut acc.tv);
    add_z_offset(&mut acc.zpv);

    VertexGroupData {
        is_rotary: acc.is_rotary,
        powered_verts: std::mem::take(&mut acc.pv),
        powered_attrib: std::mem::take(&mut acc.pva),
        travel_verts: std::mem::take(&mut acc.tv),
        zero_power_verts: std::mem::take(&mut acc.zpv),
        powered_cmd_offsets: acc.pv_off.clone(),
        travel_cmd_offsets: acc.tv_off.clone(),
        overlay_positions: std::mem::take(&mut acc.ov_pos),
        overlay_attrib: std::mem::take(&mut acc.ov_attrib),
        overlay_cmd_offsets: acc.ov_off.clone(),
    }
}

// ------------------------------------------------------------------
// Encoder implementation
// ------------------------------------------------------------------

impl Encoder for SceneSpec {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        ctx.callbacks.report_progress(0.0, "scene3d: compile");
        let data = ctx.ops.compile_scene_3d(self);
        ctx.callbacks.report_progress(1.0, "scene3d: done");
        Ok(EncodeOutput::Scene(data))
    }

    fn name(&self) -> &str {
        "scene3d"
    }
}
