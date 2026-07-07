//! Profiling trace record format and recorder adapter.
//!
//! Defines the per-step record serialised as MessagePack via rmp-serde.
//! The generic [`crate::trace::Tracer`] writes these records to the
//! self-contained trace file.
//!
//! [`TraceRecorder`] wraps an optional [`Tracer`] and exposes one-line
//! methods for each record type.  Runtime gating (via
//! `ProfileOuterOptions::trace_path` / `ProfileInnerOptions::trace_path`)
//! means call sites in the orchestrator are unconditional.

use serde::Serialize;
use std::path::PathBuf;

use crate::ops::container::Ops;
use crate::trace::Tracer;
use crate::types::{Point3D, Polygon};

#[derive(Serialize)]
struct GeometryRecord {
    pub kind: &'static str,
    pub tool_radius: f64,
    pub boundary: Vec<(f64, f64)>,
    pub islands: Vec<Vec<(f64, f64)>>,
    pub offset_polys: Vec<Vec<(f64, f64)>>,
    pub walk_order: Vec<u32>,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct TraceRecordHeader {
    pub kind: &'static str,
    pub status: u8,
    pub step_idx: u32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub heading: f64,
    pub prev_x: f64,
    pub prev_y: f64,
    pub prev_z: f64,
    pub ops_len: u32,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct ProfilePayload {
    pub iters: u32,
    pub smoothed_heading: f64,
    pub predicted_angle: f64,
    pub iteration_angle: f64,
    pub eng_angle: f64,
    pub eng_area: f64,
    pub eng_chord: f64,
    pub cut_area: f64,
    pub wall_distance: f64,
    pub target_polygon_idx: u32,
    pub polygon_perimeter: f64,
    pub cumulative_distance: f64,
    pub current_feed_rate: i32,
    pub step_length_used: f64,
    pub engagement_reductions: u32,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct TraceRecord {
    #[serde(flatten)]
    pub header: TraceRecordHeader,
    pub payload: ProfilePayload,
}

pub(crate) struct TraceRecorder {
    tracer: Option<Tracer>,
    step_idx: u32,
}

impl TraceRecorder {
    pub fn new(
        trace_path: Option<&PathBuf>,
        tool_radius: f64,
        boundary: &Polygon,
        islands: &[Polygon],
        offset_polys: &[Polygon],
        walk_order: &[u32],
    ) -> Self {
        let tracer = match trace_path {
            Some(path) => match Tracer::open(path) {
                Ok(mut t) => {
                    t.write(&GeometryRecord {
                        kind: "geometry",
                        tool_radius,
                        boundary: boundary.iter().map(|p| (p.x, p.y)).collect(),
                        islands: islands
                            .iter()
                            .map(|poly| {
                                poly.iter().map(|p| (p.x, p.y)).collect()
                            })
                            .collect(),
                        offset_polys: offset_polys
                            .iter()
                            .map(|poly| {
                                poly.iter().map(|p| (p.x, p.y)).collect()
                            })
                            .collect(),
                        walk_order: walk_order.to_vec(),
                    });
                    Some(t)
                }
                Err(e) => {
                    eprintln!("trace: failed to open {:?}: {}", path, e);
                    None
                }
            },
            None => None,
        };
        Self {
            tracer,
            step_idx: 1,
        }
    }

    fn build_record(
        &self,
        kind: &'static str,
        step_idx: u32,
        pos: Point3D,
        heading: f64,
        prev: Point3D,
        ops_len: u32,
    ) -> TraceRecord {
        TraceRecord {
            header: TraceRecordHeader {
                kind,
                status: 0,
                step_idx,
                pos_x: pos.x,
                pos_y: pos.y,
                pos_z: pos.z,
                heading,
                prev_x: prev.x,
                prev_y: prev.y,
                prev_z: prev.z,
                ops_len,
            },
            payload: ProfilePayload {
                iters: 0,
                smoothed_heading: heading,
                predicted_angle: 0.0,
                iteration_angle: 0.0,
                eng_angle: 0.0,
                eng_area: 0.0,
                eng_chord: 0.0,
                cut_area: 0.0,
                wall_distance: 0.0,
                target_polygon_idx: 0,
                polygon_perimeter: 0.0,
                cumulative_distance: 0.0,
                current_feed_rate: 0,
                step_length_used: 0.0,
                engagement_reductions: 0,
            },
        }
    }

    pub fn record_init(&mut self, pos: Point3D, heading: f64, ops_len: u32) {
        let rec = self.build_record("init", 0, pos, heading, pos, ops_len);
        if let Some(ref mut tr) = self.tracer {
            tr.write(&rec);
        }
    }

    pub fn record_polygon_start(
        &mut self,
        pos: Point3D,
        heading: f64,
        ops_len: u32,
        target_polygon_idx: u32,
        polygon_perimeter: f64,
    ) {
        let mut rec = self.build_record(
            "polygon_start",
            self.step_idx,
            pos,
            heading,
            pos,
            ops_len,
        );
        rec.payload.target_polygon_idx = target_polygon_idx;
        rec.payload.polygon_perimeter = polygon_perimeter;
        if let Some(ref mut tr) = self.tracer {
            tr.write(&rec);
        }
        self.step_idx += 1;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_cut(
        &mut self,
        pos: Point3D,
        heading: f64,
        prev: Point3D,
        ops_len: u32,
        target_polygon_idx: u32,
        polygon_perimeter: f64,
        cumulative_distance: f64,
        current_feed_rate: i32,
        step_length_used: f64,
        engagement_reductions: u32,
        eng_angle: f64,
        eng_area: f64,
        eng_chord: f64,
        wall_distance: f64,
    ) {
        let mut rec = self.build_record(
            "cut",
            self.step_idx,
            pos,
            heading,
            prev,
            ops_len,
        );
        rec.payload.target_polygon_idx = target_polygon_idx;
        rec.payload.polygon_perimeter = polygon_perimeter;
        rec.payload.cumulative_distance = cumulative_distance;
        rec.payload.current_feed_rate = current_feed_rate;
        rec.payload.step_length_used = step_length_used;
        rec.payload.engagement_reductions = engagement_reductions;
        rec.payload.eng_angle = eng_angle;
        rec.payload.eng_area = eng_area;
        rec.payload.eng_chord = eng_chord;
        rec.payload.cut_area = eng_area;
        rec.payload.wall_distance = wall_distance;
        if let Some(ref mut tr) = self.tracer {
            tr.write(&rec);
        }
        self.step_idx += 1;
    }

    pub fn record_feed_change(
        &mut self,
        pos: Point3D,
        heading: f64,
        ops_len: u32,
        _old_feed_rate: i32,
        new_feed_rate: i32,
    ) {
        let mut rec = self.build_record(
            "feed_change",
            self.step_idx,
            pos,
            heading,
            pos,
            ops_len,
        );
        rec.payload.current_feed_rate = new_feed_rate;
        if let Some(ref mut tr) = self.tracer {
            tr.write(&rec);
        }
        self.step_idx += 1;
    }

    pub fn record_polygon_end(
        &mut self,
        pos: Point3D,
        heading: f64,
        ops_len: u32,
        target_polygon_idx: u32,
    ) {
        let mut rec = self.build_record(
            "polygon_end",
            self.step_idx,
            pos,
            heading,
            pos,
            ops_len,
        );
        rec.payload.target_polygon_idx = target_polygon_idx;
        if let Some(ref mut tr) = self.tracer {
            tr.write(&rec);
        }
        self.step_idx += 1;
    }

    pub fn record_exit(&mut self, pos: Point3D, heading: f64, ops_len: u32) {
        let rec = self.build_record(
            "exit",
            self.step_idx,
            pos,
            heading,
            pos,
            ops_len,
        );
        if let Some(ref mut tr) = self.tracer {
            tr.write(&rec);
        }
    }

    pub fn finish(self, _ops: &Ops) {
        if let Some(mut t) = self.tracer {
            let _ = t.finish();
        }
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.tracer.is_some()
    }
}
