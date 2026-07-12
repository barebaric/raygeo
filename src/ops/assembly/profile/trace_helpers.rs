use std::collections::BTreeMap;

use crate::ops::assembly::trace_utils as tu;
use crate::trace_types::{Meta, MetaValue, ToolSnapshot};
use crate::types::{Point3D, Polygon};

pub(super) fn tool_snapshot(
    pos: Point3D,
    heading: f64,
    prev: Point3D,
) -> ToolSnapshot {
    tu::tool_snapshot(pos, heading, prev)
}

pub(super) fn build_attrs(
    offset_polys: &[Polygon],
    walk_order: &[u32],
) -> Meta {
    let mut attrs: Meta = BTreeMap::new();
    attrs.insert(
        "offset_polys".into(),
        MetaValue::List(offset_polys.iter().map(tu::polygon_to_meta).collect()),
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
    tu::meta_insert_u32(&mut m, "polygon_idx", polygon_idx);
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
    tu::meta_insert_u32(&mut m, "target_polygon_idx", target_polygon_idx);
    tu::meta_insert_f64(&mut m, "cumulative_distance", cumulative_distance);
    tu::meta_insert_f64(&mut m, "polygon_perimeter", polygon_perimeter);
    tu::meta_insert_f64(&mut m, "wall_distance", wall_distance);
    tu::meta_insert_i64(&mut m, "current_feed_rate", current_feed_rate as i64);
    tu::meta_insert_f64(&mut m, "step_length", step_length_used);
    tu::meta_insert_u32(&mut m, "engagement_reductions", engagement_reductions);
    if is_polygon_start {
        tu::meta_insert_bool(&mut m, "polygon_start", true);
    }
    m
}

pub(super) fn feed_change_meta(old_feed_rate: i32, new_feed_rate: i32) -> Meta {
    let mut m: Meta = BTreeMap::new();
    tu::meta_insert_bool(&mut m, "feed_change", true);
    tu::meta_insert_i64(&mut m, "current_feed_rate", new_feed_rate as i64);
    tu::meta_insert_i64(&mut m, "old_feed_rate", old_feed_rate as i64);
    tu::meta_insert_i64(&mut m, "new_feed_rate", new_feed_rate as i64);
    m
}
