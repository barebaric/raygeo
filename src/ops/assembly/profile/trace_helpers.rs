use std::collections::BTreeMap;

use crate::trace_types::{Meta, MetaValue, ToolSnapshot};
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

pub(super) fn tool_snapshot(
    pos: Point3D,
    heading: f64,
    prev: Point3D,
) -> ToolSnapshot {
    ToolSnapshot {
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
        heading,
        prev_x: prev.x,
        prev_y: prev.y,
        prev_z: prev.z,
    }
}

pub(super) fn build_attrs(
    offset_polys: &[Polygon],
    walk_order: &[u32],
) -> Meta {
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
    attrs
}

pub(super) fn init_meta(polygon_idx: u32) -> Meta {
    let mut m: Meta = BTreeMap::new();
    meta_insert_u32(&mut m, "polygon_idx", polygon_idx);
    m
}

#[allow(clippy::too_many_arguments)]
pub(super) fn cut_meta(
    target_polygon_idx: u32,
    cumulative_distance: f64,
    polygon_perimeter: f64,
    wall_distance: f64,
    current_feed_rate: i32,
    step_length_used: f64,
    engagement_reductions: u32,
    is_polygon_start: bool,
) -> Meta {
    let mut m: Meta = BTreeMap::new();
    meta_insert_u32(&mut m, "target_polygon_idx", target_polygon_idx);
    meta_insert_f64(&mut m, "cumulative_distance", cumulative_distance);
    meta_insert_f64(&mut m, "polygon_perimeter", polygon_perimeter);
    meta_insert_f64(&mut m, "wall_distance", wall_distance);
    meta_insert_i64(&mut m, "current_feed_rate", current_feed_rate as i64);
    meta_insert_f64(&mut m, "step_length", step_length_used);
    meta_insert_u32(&mut m, "engagement_reductions", engagement_reductions);
    if is_polygon_start {
        meta_insert_bool(&mut m, "polygon_start", true);
    }
    m
}

pub(super) fn feed_change_meta(old_feed_rate: i32, new_feed_rate: i32) -> Meta {
    let mut m: Meta = BTreeMap::new();
    meta_insert_bool(&mut m, "feed_change", true);
    meta_insert_i64(&mut m, "current_feed_rate", new_feed_rate as i64);
    meta_insert_i64(&mut m, "old_feed_rate", old_feed_rate as i64);
    meta_insert_i64(&mut m, "new_feed_rate", new_feed_rate as i64);
    m
}
