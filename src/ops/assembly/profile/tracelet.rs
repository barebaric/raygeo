use std::collections::BTreeMap;

use crate::ops::assembly::result::{AssemblyTrace, TraceEventData};
use crate::trace_types::{
    EventKind, Meta, MetaValue, MoveKind, ProgressSnapshot, ToolSnapshot,
};
use crate::types::{Point3D, Polygon};

fn polygon_to_meta(poly: &Polygon) -> MetaValue {
    MetaValue::List(
        poly.iter()
            .map(|p| {
                MetaValue::List(vec![MetaValue::F64(p.x), MetaValue::F64(p.y)])
            })
            .collect(),
    )
}

fn meta_insert_f64(meta: &mut Meta, key: &str, value: f64) {
    meta.insert(key.into(), MetaValue::F64(value));
}

fn meta_insert_u32(meta: &mut Meta, key: &str, value: u32) {
    meta.insert(key.into(), MetaValue::U32(value));
}

fn meta_insert_i64(meta: &mut Meta, key: &str, value: i64) {
    meta.insert(key.into(), MetaValue::I64(value));
}

fn meta_insert_bool(meta: &mut Meta, key: &str, value: bool) {
    meta.insert(key.into(), MetaValue::Bool(value));
}

pub(super) struct ProfileTracelet {
    attrs: Option<Meta>,
    events: Vec<TraceEventData>,
    step_idx: u32,
}

impl ProfileTracelet {
    pub(super) fn new() -> Self {
        Self {
            attrs: None,
            events: Vec::new(),
            step_idx: 0,
        }
    }

    pub(super) fn set_attrs(
        &mut self,
        offset_polys: &[Polygon],
        walk_order: &[u32],
    ) {
        let mut attrs: Meta = BTreeMap::new();
        attrs.insert(
            "offset_polys".into(),
            MetaValue::List(offset_polys.iter().map(polygon_to_meta).collect()),
        );
        attrs.insert(
            "walk_order".into(),
            MetaValue::List(
                walk_order.iter().map(|&i| MetaValue::U32(i)).collect(),
            ),
        );
        self.attrs = Some(attrs);
    }

    pub(super) fn record_init(
        &mut self,
        pos: Point3D,
        heading: f64,
        polygon_idx: u32,
    ) {
        let mut meta: Meta = BTreeMap::new();
        meta_insert_u32(&mut meta, "polygon_idx", polygon_idx);
        self.events.push(TraceEventData {
            kind: EventKind::Init,
            move_kind: None,
            tool: Some(ToolSnapshot {
                pos_x: pos.x,
                pos_y: pos.y,
                pos_z: pos.z,
                heading,
                prev_x: pos.x,
                prev_y: pos.y,
                prev_z: pos.z,
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
        pos: Point3D,
        prev: Point3D,
        heading: f64,
        target_polygon_idx: u32,
        cumulative_distance: f64,
        polygon_perimeter: f64,
        wall_distance: f64,
        current_feed_rate: i32,
        step_length_used: f64,
        engagement_reductions: u32,
        is_polygon_start: bool,
    ) {
        let mut meta: Meta = BTreeMap::new();
        meta_insert_u32(&mut meta, "target_polygon_idx", target_polygon_idx);
        meta_insert_f64(&mut meta, "cumulative_distance", cumulative_distance);
        meta_insert_f64(&mut meta, "polygon_perimeter", polygon_perimeter);
        meta_insert_f64(&mut meta, "wall_distance", wall_distance);
        meta_insert_i64(
            &mut meta,
            "current_feed_rate",
            current_feed_rate as i64,
        );
        meta_insert_f64(&mut meta, "step_length", step_length_used);
        meta_insert_u32(
            &mut meta,
            "engagement_reductions",
            engagement_reductions,
        );
        if is_polygon_start {
            meta_insert_bool(&mut meta, "polygon_start", true);
        }
        self.events.push(TraceEventData {
            kind: EventKind::Move,
            move_kind: Some(MoveKind::Cut),
            tool: Some(ToolSnapshot {
                pos_x: pos.x,
                pos_y: pos.y,
                pos_z: pos.z,
                heading,
                prev_x: prev.x,
                prev_y: prev.y,
                prev_z: prev.z,
            }),
            progress: Some(ProgressSnapshot {
                step_idx: self.step_idx,
                ops_len: 0,
                status: 0,
            }),
            meta: Some(meta),
        });
        self.step_idx += 1;
    }

    pub(super) fn record_feed_change(
        &mut self,
        pos: Point3D,
        heading: f64,
        ops_len: u32,
        old_feed_rate: i32,
        new_feed_rate: i32,
    ) {
        let mut meta: Meta = BTreeMap::new();
        meta_insert_bool(&mut meta, "feed_change", true);
        meta_insert_i64(&mut meta, "current_feed_rate", new_feed_rate as i64);
        meta_insert_i64(&mut meta, "old_feed_rate", old_feed_rate as i64);
        meta_insert_i64(&mut meta, "new_feed_rate", new_feed_rate as i64);
        self.events.push(TraceEventData {
            kind: EventKind::Move,
            move_kind: Some(MoveKind::Travel),
            tool: Some(ToolSnapshot {
                pos_x: pos.x,
                pos_y: pos.y,
                pos_z: pos.z,
                heading,
                prev_x: pos.x,
                prev_y: pos.y,
                prev_z: pos.z,
            }),
            progress: Some(ProgressSnapshot {
                step_idx: self.step_idx,
                ops_len,
                status: 0,
            }),
            meta: Some(meta),
        });
        self.step_idx += 1;
    }

    pub(super) fn record_exit(&mut self, pos: Point3D, heading: f64) {
        self.events.push(TraceEventData {
            kind: EventKind::Exit,
            move_kind: None,
            tool: Some(ToolSnapshot {
                pos_x: pos.x,
                pos_y: pos.y,
                pos_z: pos.z,
                heading,
                prev_x: pos.x,
                prev_y: pos.y,
                prev_z: pos.z,
            }),
            progress: None,
            meta: None,
        });
    }

    pub(super) fn finish(self) -> AssemblyTrace {
        AssemblyTrace {
            attrs: self.attrs,
            events: self.events,
        }
    }
}
