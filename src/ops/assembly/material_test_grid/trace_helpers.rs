use std::collections::BTreeMap;

use crate::ops::assembly::trace_utils as tu;
use crate::trace_types::{Meta, MetaValue, ToolSnapshot};
use crate::types::Point3D;

use super::MaterialTestGridParams;

pub(super) fn tool_snapshot(pos: Point3D, prev: Point3D) -> ToolSnapshot {
    tu::tool_snapshot(pos, 0.0, prev)
}

pub(super) fn build_attrs(params: &MaterialTestGridParams) -> Meta {
    let mut attrs: Meta = BTreeMap::new();
    tu::meta_insert_u32(&mut attrs, "cols", params.cols);
    tu::meta_insert_u32(&mut attrs, "rows", params.rows);
    tu::meta_insert_f64(&mut attrs, "min_speed", params.min_speed);
    tu::meta_insert_f64(&mut attrs, "max_speed", params.max_speed);
    tu::meta_insert_f64(&mut attrs, "min_power", params.min_power);
    tu::meta_insert_f64(&mut attrs, "max_power", params.max_power);
    tu::meta_insert_u32(&mut attrs, "min_passes", params.min_passes);
    tu::meta_insert_u32(&mut attrs, "max_passes", params.max_passes);
    tu::meta_insert_f64(&mut attrs, "min_offset", params.min_offset);
    tu::meta_insert_f64(&mut attrs, "max_offset", params.max_offset);
    tu::meta_insert_f64(&mut attrs, "shape_size", params.shape_size);
    tu::meta_insert_f64(&mut attrs, "spacing", params.spacing);
    tu::meta_insert_f64(
        &mut attrs,
        "line_interval_mm",
        params.line_interval_mm,
    );
    attrs.insert("mode".into(), MetaValue::Str(params.mode.clone()));
    attrs.insert("grid_mode".into(), MetaValue::Str(params.grid_mode.clone()));
    tu::meta_insert_bool(&mut attrs, "include_labels", params.include_labels);
    tu::meta_insert_f64(&mut attrs, "label_power", params.label_power);
    tu::meta_insert_i64(&mut attrs, "label_speed", params.label_speed as i64);
    attrs
}

pub(super) fn init_meta(total_cells: u32) -> Meta {
    let mut m: Meta = BTreeMap::new();
    tu::meta_insert_u32(&mut m, "total_cells", total_cells);
    m
}
