use std::collections::BTreeMap;

use crate::geo::algo::medial_axis::MedialAxis;
use crate::ops::assembly::result::{AssemblyTrace, TraceEventData};
use crate::ops::cut::ClearedArea;
use crate::trace_types::{
    EventKind, Meta, MetaValue, MoveKind, ProgressSnapshot, ToolSnapshot,
};
use crate::types::{Point3D, Polygon};

use super::tool::Tool;
use super::AdaptiveClearingOptions;

fn polygon_to_meta(poly: &Polygon) -> MetaValue {
    MetaValue::List(
        poly.iter()
            .map(|p| {
                MetaValue::List(vec![MetaValue::F64(p.x), MetaValue::F64(p.y)])
            })
            .collect(),
    )
}

fn xy_points_to_meta(points: &[(f64, f64)]) -> MetaValue {
    MetaValue::List(
        points
            .iter()
            .map(|(x, y)| {
                MetaValue::List(vec![MetaValue::F64(*x), MetaValue::F64(*y)])
            })
            .collect(),
    )
}

fn point3d_to_list(p: Point3D) -> MetaValue {
    MetaValue::List(vec![
        MetaValue::F64(p.x),
        MetaValue::F64(p.y),
        MetaValue::F64(p.z),
    ])
}

fn meta_insert_f64(meta: &mut Meta, key: &str, value: f64) {
    meta.insert(key.into(), MetaValue::F64(value));
}

fn meta_insert_u32(meta: &mut Meta, key: &str, value: u32) {
    meta.insert(key.into(), MetaValue::U32(value));
}

fn make_tool_snapshot(tool: &Tool, prev_pos: Point3D) -> ToolSnapshot {
    ToolSnapshot {
        pos_x: tool.pos.x,
        pos_y: tool.pos.y,
        pos_z: tool.pos.z,
        heading: tool.heading,
        prev_x: prev_pos.x,
        prev_y: prev_pos.y,
        prev_z: prev_pos.z,
    }
}

fn make_progress(step_idx: u32, ops_len: u32) -> ProgressSnapshot {
    ProgressSnapshot {
        step_idx,
        ops_len,
        status: 0,
    }
}

pub(super) struct AdaptiveTracelet {
    attrs: Option<Meta>,
    events: Vec<TraceEventData>,
    step_idx: u32,
    active: bool,
}

impl AdaptiveTracelet {
    pub(super) fn new(opts: &AdaptiveClearingOptions) -> Self {
        let mut attrs: Meta = BTreeMap::new();
        meta_insert_f64(&mut attrs, "tool_radius", opts.tool_radius);
        attrs.insert("boundary".into(), polygon_to_meta(&opts.pocket_boundary));
        attrs.insert(
            "islands".into(),
            MetaValue::List(opts.islands.iter().map(polygon_to_meta).collect()),
        );
        Self {
            attrs: Some(attrs),
            events: Vec::new(),
            step_idx: 0,
            active: true,
        }
    }

    pub(super) fn set_seeds(&mut self, seeds: &[Polygon]) {
        if let Some(ref mut attrs) = self.attrs {
            attrs.insert(
                "seeds".into(),
                MetaValue::List(seeds.iter().map(polygon_to_meta).collect()),
            );
        }
    }

    pub(super) fn set_mat(&mut self, mat: &MedialAxis) {
        if let Some(ref mut attrs) = self.attrs {
            attrs.insert(
                "mat_nodes".into(),
                MetaValue::List(
                    mat.nodes
                        .iter()
                        .map(|n| {
                            MetaValue::List(vec![
                                MetaValue::F64(n.point.x),
                                MetaValue::F64(n.point.y),
                            ])
                        })
                        .collect(),
                ),
            );
            attrs.insert(
                "mat_clearances".into(),
                MetaValue::List(
                    mat.nodes
                        .iter()
                        .map(|n| MetaValue::F64(n.clearance))
                        .collect(),
                ),
            );
            attrs.insert(
                "mat_edges".into(),
                MetaValue::List(
                    mat.edges
                        .iter()
                        .map(|&(i, j)| {
                            MetaValue::List(vec![
                                MetaValue::U32(i as u32),
                                MetaValue::U32(j as u32),
                            ])
                        })
                        .collect(),
                ),
            );
            meta_insert_u32(attrs, "mat_root", mat.root as u32);
        }
    }

    pub(super) fn record_init(&mut self, tool: &Tool, cleared: &ClearedArea) {
        if !self.active {
            return;
        }
        let mut meta: Meta = BTreeMap::new();
        meta_insert_f64(&mut meta, "total_area", cleared.total_area());
        meta_insert_f64(&mut meta, "remaining_area", cleared.remaining_area());
        self.events.push(TraceEventData {
            kind: EventKind::Init,
            move_kind: None,
            tool: Some(ToolSnapshot {
                pos_x: tool.pos.x,
                pos_y: tool.pos.y,
                pos_z: tool.pos.z,
                heading: tool.heading,
                prev_x: tool.pos.x,
                prev_y: tool.pos.y,
                prev_z: tool.pos.z,
            }),
            progress: Some(ProgressSnapshot {
                step_idx: 0,
                ops_len: 0,
                status: 0,
            }),
            meta: Some(meta),
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_cut(
        &mut self,
        tool: &Tool,
        cleared: &ClearedArea,
        iters: u32,
        eng_angle: f64,
        eng_area: f64,
        eng_chord: f64,
        cut_area: f64,
        iteration_angle: f64,
        prev_pos: Point3D,
        ops_len: u32,
    ) {
        if !self.active {
            return;
        }
        let mut meta: Meta = BTreeMap::new();
        meta_insert_u32(&mut meta, "iters", iters);
        meta_insert_f64(&mut meta, "eng_angle", eng_angle);
        meta_insert_f64(&mut meta, "eng_area", eng_area);
        meta_insert_f64(&mut meta, "eng_chord", eng_chord);
        meta_insert_f64(&mut meta, "iteration_angle", iteration_angle);
        meta_insert_f64(&mut meta, "cut_area", cut_area);
        meta_insert_f64(&mut meta, "total_area", cleared.total_area());
        meta_insert_f64(&mut meta, "remaining_area", cleared.remaining_area());
        meta_insert_f64(&mut meta, "smoothed_heading", tool.smoothed_heading());
        meta_insert_f64(&mut meta, "predicted_angle", tool.raw_predictor());
        self.events.push(TraceEventData {
            kind: EventKind::Move,
            move_kind: Some(MoveKind::Cut),
            tool: Some(make_tool_snapshot(tool, prev_pos)),
            progress: Some(make_progress(self.step_idx, ops_len)),
            meta: Some(meta),
        });
        self.step_idx += 1;
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_resume(
        &mut self,
        kind: EventKind,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point3D,
        ops_len: u32,
        resume_source: u8,
        route_source: u8,
        resume_reasons: &[u8; 6],
        resume_details: &[u8; 6],
        route_details: &[u8; 4],
        resume_point: Point3D,
        candidates: &[Option<Point3D>; 6],
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        if !self.active {
            return;
        }
        let mut meta: Meta = BTreeMap::new();
        meta_insert_f64(&mut meta, "total_area", cleared.total_area());
        meta_insert_f64(&mut meta, "remaining_area", cleared.remaining_area());
        meta_insert_u32(&mut meta, "resume_source", resume_source as u32);
        meta_insert_u32(&mut meta, "route_source", route_source as u32);
        meta.insert(
            "resume_reasons".into(),
            MetaValue::List(
                resume_reasons
                    .iter()
                    .map(|&v| MetaValue::U32(v as u32))
                    .collect(),
            ),
        );
        meta.insert(
            "resume_details".into(),
            MetaValue::List(
                resume_details
                    .iter()
                    .map(|&v| MetaValue::U32(v as u32))
                    .collect(),
            ),
        );
        meta.insert(
            "route_details".into(),
            MetaValue::List(
                route_details
                    .iter()
                    .map(|&v| MetaValue::U32(v as u32))
                    .collect(),
            ),
        );
        meta.insert("resume_point".into(), point3d_to_list(resume_point));
        meta.insert(
            "candidates".into(),
            MetaValue::List(
                candidates
                    .iter()
                    .map(|c| match c {
                        Some(p) => point3d_to_list(*p),
                        None => MetaValue::List(vec![
                            MetaValue::F64(f64::NAN),
                            MetaValue::F64(f64::NAN),
                            MetaValue::F64(f64::NAN),
                        ]),
                    })
                    .collect(),
            ),
        );
        meta.insert(
            "wall_hug_points".into(),
            xy_points_to_meta(wall_hug_points),
        );
        meta.insert(
            "wall_hug_segment_counts".into(),
            MetaValue::List(
                wall_hug_segment_counts
                    .iter()
                    .map(|&v| MetaValue::U32(v))
                    .collect(),
            ),
        );
        self.events.push(TraceEventData {
            kind,
            move_kind: None,
            tool: Some(make_tool_snapshot(tool, prev_pos)),
            progress: Some(make_progress(self.step_idx, ops_len)),
            meta: Some(meta),
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_exit(
        &mut self,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point3D,
        ops_len: u32,
        resume_reasons: &[u8; 6],
        resume_details: &[u8; 6],
        route_details: &[u8; 4],
        resume_point: Point3D,
        candidates: &[Option<Point3D>; 6],
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        if !self.active {
            return;
        }
        let mut meta: Meta = BTreeMap::new();
        meta_insert_f64(&mut meta, "total_area", cleared.total_area());
        meta_insert_f64(&mut meta, "remaining_area", cleared.remaining_area());
        meta.insert(
            "resume_reasons".into(),
            MetaValue::List(
                resume_reasons
                    .iter()
                    .map(|&v| MetaValue::U32(v as u32))
                    .collect(),
            ),
        );
        meta.insert(
            "resume_details".into(),
            MetaValue::List(
                resume_details
                    .iter()
                    .map(|&v| MetaValue::U32(v as u32))
                    .collect(),
            ),
        );
        meta.insert(
            "route_details".into(),
            MetaValue::List(
                route_details
                    .iter()
                    .map(|&v| MetaValue::U32(v as u32))
                    .collect(),
            ),
        );
        meta.insert("resume_point".into(), point3d_to_list(resume_point));
        meta.insert(
            "candidates".into(),
            MetaValue::List(
                candidates
                    .iter()
                    .map(|c| match c {
                        Some(p) => point3d_to_list(*p),
                        None => MetaValue::List(vec![
                            MetaValue::F64(f64::NAN),
                            MetaValue::F64(f64::NAN),
                            MetaValue::F64(f64::NAN),
                        ]),
                    })
                    .collect(),
            ),
        );
        meta.insert(
            "wall_hug_points".into(),
            xy_points_to_meta(wall_hug_points),
        );
        meta.insert(
            "wall_hug_segment_counts".into(),
            MetaValue::List(
                wall_hug_segment_counts
                    .iter()
                    .map(|&v| MetaValue::U32(v))
                    .collect(),
            ),
        );
        self.events.push(TraceEventData {
            kind: EventKind::Exit,
            move_kind: None,
            tool: Some(make_tool_snapshot(tool, prev_pos)),
            progress: Some(make_progress(self.step_idx, ops_len)),
            meta: Some(meta),
        });
    }

    pub(super) fn finish(self) -> AssemblyTrace {
        AssemblyTrace {
            attrs: self.attrs,
            events: self.events,
        }
    }
}
