use std::collections::BTreeMap;

use crate::geo::algo::medial_axis::MedialAxis;
use crate::ops::assembly::trace_utils as tu;
use crate::ops::part::ClearedArea;
use crate::ops::part::StockRegion;
use crate::trace_types::{Meta, MetaValue, ToolSnapshot};
use crate::types::{Point3D, Polygon};

use super::tool::Tool;
use super::AdaptiveClearingSpec;

pub(super) fn make_tool_snapshot(
    tool: &Tool,
    prev_pos: Point3D,
) -> ToolSnapshot {
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

pub(super) fn build_attrs(
    opts: &AdaptiveClearingSpec,
    boundary: &Polygon,
    islands: &[Polygon],
    seeds: &[Polygon],
    mat: Option<&MedialAxis>,
) -> Meta {
    let mut attrs: Meta = BTreeMap::new();
    tu::meta_insert_f64(&mut attrs, "tool_radius", opts.tool_radius);
    attrs.insert("boundary".into(), tu::polygon_to_meta(boundary));
    attrs.insert(
        "islands".into(),
        MetaValue::List(islands.iter().map(tu::polygon_to_meta).collect()),
    );
    attrs.insert(
        "seeds".into(),
        MetaValue::List(seeds.iter().map(tu::polygon_to_meta).collect()),
    );
    if let Some(mat) = mat {
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
        tu::meta_insert_u32(&mut attrs, "mat_root", mat.root as u32);
    }
    attrs
}

pub(super) fn init_meta(cleared: &ClearedArea, region: &StockRegion) -> Meta {
    let mut m: Meta = BTreeMap::new();
    tu::meta_insert_f64(&mut m, "total_area", cleared.total_area());
    tu::meta_insert_f64(
        &mut m,
        "remaining_area",
        cleared.remaining_area(region),
    );
    m
}

#[allow(clippy::too_many_arguments)]
pub(super) fn cut_meta(
    tool: &Tool,
    cleared: &ClearedArea,
    region: &StockRegion,
    iters: u32,
    eng_angle: f64,
    eng_area: f64,
    eng_chord: f64,
    cut_area: f64,
    iteration_angle: f64,
) -> Meta {
    let mut m: Meta = BTreeMap::new();
    tu::meta_insert_u32(&mut m, "iters", iters);
    tu::meta_insert_f64(&mut m, "eng_angle", eng_angle);
    tu::meta_insert_f64(&mut m, "eng_area", eng_area);
    tu::meta_insert_f64(&mut m, "eng_chord", eng_chord);
    tu::meta_insert_f64(&mut m, "cut_area", cut_area);
    tu::meta_insert_f64(&mut m, "iteration_angle", iteration_angle);
    tu::meta_insert_f64(&mut m, "total_area", cleared.total_area());
    tu::meta_insert_f64(
        &mut m,
        "remaining_area",
        cleared.remaining_area(region),
    );
    tu::meta_insert_f64(&mut m, "smoothed_heading", tool.smoothed_heading());
    tu::meta_insert_f64(&mut m, "predicted_angle", tool.raw_predictor());
    m
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resume_exit_meta(
    cleared: &ClearedArea,
    region: &StockRegion,
    resume_reasons: &[u8; 6],
    resume_details: &[u8; 6],
    route_details: &[u8; 4],
    resume_point: Point3D,
    candidates: &[Option<Point3D>; 6],
    wall_hug_points: &[(f64, f64)],
    wall_hug_segment_counts: &[u32],
    resume_source: Option<u8>,
    route_source: Option<u8>,
) -> Meta {
    let mut m: Meta = BTreeMap::new();
    tu::meta_insert_f64(&mut m, "total_area", cleared.total_area());
    tu::meta_insert_f64(
        &mut m,
        "remaining_area",
        cleared.remaining_area(region),
    );
    if let Some(src) = resume_source {
        tu::meta_insert_u32(&mut m, "resume_source", src as u32);
    }
    if let Some(src) = route_source {
        tu::meta_insert_u32(&mut m, "route_source", src as u32);
    }
    m.insert(
        "resume_reasons".into(),
        MetaValue::List(
            resume_reasons
                .iter()
                .map(|&v| MetaValue::U32(v as u32))
                .collect(),
        ),
    );
    m.insert(
        "resume_details".into(),
        MetaValue::List(
            resume_details
                .iter()
                .map(|&v| MetaValue::U32(v as u32))
                .collect(),
        ),
    );
    m.insert(
        "route_details".into(),
        MetaValue::List(
            route_details
                .iter()
                .map(|&v| MetaValue::U32(v as u32))
                .collect(),
        ),
    );
    m.insert("resume_point".into(), tu::point3d_to_list(resume_point));
    m.insert(
        "candidates".into(),
        MetaValue::List(
            candidates
                .iter()
                .map(|c| match c {
                    Some(p) => tu::point3d_to_list(*p),
                    None => MetaValue::List(vec![
                        MetaValue::F64(f64::NAN),
                        MetaValue::F64(f64::NAN),
                        MetaValue::F64(f64::NAN),
                    ]),
                })
                .collect(),
        ),
    );
    m.insert(
        "wall_hug_points".into(),
        tu::xy_points_to_meta(wall_hug_points),
    );
    m.insert(
        "wall_hug_segment_counts".into(),
        MetaValue::List(
            wall_hug_segment_counts
                .iter()
                .map(|&v| MetaValue::U32(v))
                .collect(),
        ),
    );
    m
}
