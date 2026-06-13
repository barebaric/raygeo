//! Tab operations: create holding tabs on toolpaths.
//!
//! When laser-cutting, holding tabs keep the cut piece in place. This
//! module supports two modes:
//!
//! - **Gap mode**: removes a section of the path at each tab location.
//! - **Power mode**: reduces the laser power in the tab region instead
//!   of cutting a gap, so the material stays connected but weaker.
//!
//! The main entry points are [`apply_tab_gaps`] and [`apply_tab_power`].

use super::container::Ops;
use super::enums::{CommandCategory, CommandType, SectionType};
use super::types::{MoveCmd, OpCategory};
use crate::types::Point3D;

/// A clip point: the center and width of a tab on the toolpath.
#[derive(Clone, Debug)]
pub struct ClipPoint {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

/// A distance interval along a subpath that should be gapped or power-modulated.
#[derive(Clone, Debug)]
struct TabRegion {
    start: f64,
    end: f64,
}

/// Key identifying a subpath within a specific section.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SubpathKey {
    section_idx: usize,
    subpath_idx: usize,
}

/// Apply gap-mode tabs to the given ops.
///
/// For each clip point, the closest subpath is found and a gap of the
/// given width is cut at the nearest point. The ops are rewritten with
/// gaps in those regions.
///
/// - `ops`: The input ops (will be replaced in-place).
/// - `clips`: Tab positions and widths.
pub fn apply_tab_gaps(ops: &mut Ops, clips: &[ClipPoint]) {
    if clips.is_empty() || ops.is_empty() {
        return;
    }
    let section_ranges = ops.iter_section_ranges();
    let assignments = assign_clips_to_subpaths(ops, &section_ranges, clips);

    let mut new_ops = Ops::new();
    for (sec_idx, sec_range) in section_ranges.iter().enumerate() {
        let sec_type = sec_range.section_type;
        let marker_indices = &sec_range.marker_indices;
        let content_indices = &sec_range.content_indices;

        for &mi in marker_indices {
            new_ops.commands.push(ops.commands[mi].clone());
        }

        if sec_type != Some(SectionType::VectorOutline) {
            for &ci in content_indices {
                new_ops.commands.push(ops.commands[ci].clone());
            }
            continue;
        }

        let content_ops = ops.sub_ops(content_indices);
        let subpaths = content_ops.subpath_indices();
        for (sp_idx, sp_indices) in subpaths.iter().enumerate() {
            let key = SubpathKey {
                section_idx: sec_idx,
                subpath_idx: sp_idx,
            };
            if let Some(sp_clips) = assignments.get(&key) {
                let sp_ops = content_ops.sub_ops(sp_indices);
                let has_curves = sp_ops.commands.iter().any(|n| {
                    matches!(
                        n.category,
                        OpCategory::Moving {
                            cmd: MoveCmd::BezierTo { .. },
                            ..
                        } | OpCategory::Moving {
                            cmd: MoveCmd::QuadraticBezierTo { .. },
                            ..
                        }
                    )
                });
                if has_curves {
                    let processed = clip_subpath_with_gaps(&sp_ops, sp_clips);
                    new_ops.extend(&processed);
                } else {
                    let mut sp = sp_ops;
                    sp.preload_state();
                    for clip in sp_clips {
                        sp.clip_at(clip.x, clip.y, clip.width);
                    }
                    new_ops.extend(&sp);
                }
            } else {
                for &si in sp_indices {
                    new_ops.commands.push(content_ops.commands[si].clone());
                }
            }
        }
    }

    ops.replace_with(&new_ops);
}

/// Apply power-mode tabs to the given ops.
///
/// Instead of cutting a gap, the laser power is reduced in the tab
/// region to create a weaker connection that holds the piece in place.
///
/// - `ops`: The input ops (will be replaced in-place).
/// - `clips`: Tab positions and widths.
/// - `tab_power`: The power level to use inside tab regions (0.0–1.0).
/// - `original_power`: The normal cutting power to restore after the tab.
pub fn apply_tab_power(
    ops: &mut Ops,
    clips: &[ClipPoint],
    tab_power: f64,
    original_power: f64,
) {
    if clips.is_empty() || ops.is_empty() {
        return;
    }
    let section_ranges = ops.iter_section_ranges();
    let assignments = assign_clips_to_subpaths(ops, &section_ranges, clips);

    let mut new_ops = Ops::new();
    for (sec_idx, sec_range) in section_ranges.iter().enumerate() {
        let sec_type = sec_range.section_type;
        let marker_indices = &sec_range.marker_indices;
        let content_indices = &sec_range.content_indices;

        for &mi in marker_indices {
            new_ops.commands.push(ops.commands[mi].clone());
        }

        if sec_type != Some(SectionType::VectorOutline) {
            for &ci in content_indices {
                new_ops.commands.push(ops.commands[ci].clone());
            }
            continue;
        }

        let content_ops = ops.sub_ops(content_indices);
        let subpaths = content_ops.subpath_indices();
        for (sp_idx, sp_indices) in subpaths.iter().enumerate() {
            let key = SubpathKey {
                section_idx: sec_idx,
                subpath_idx: sp_idx,
            };
            if let Some(sp_clips) = assignments.get(&key) {
                let sp_ops = content_ops.sub_ops(sp_indices);
                let processed = insert_power_commands(
                    &sp_ops,
                    sp_clips,
                    tab_power,
                    original_power,
                );
                new_ops.extend(&processed);
            } else {
                for &si in sp_indices {
                    new_ops.commands.push(content_ops.commands[si].clone());
                }
            }
        }
    }

    ops.replace_with(&new_ops);
}

// ---------------------------------------------------------------------------
// Clip-to-subpath assignment
// ---------------------------------------------------------------------------

fn assign_clips_to_subpaths(
    ops: &Ops,
    section_ranges: &[super::group::OpsSectionRange],
    clips: &[ClipPoint],
) -> std::collections::HashMap<SubpathKey, Vec<ClipPoint>> {
    use std::collections::HashMap;

    let mut all_subpaths: Vec<(SubpathKey, Ops)> = Vec::new();
    for (sec_idx, sec_range) in section_ranges.iter().enumerate() {
        if sec_range.section_type != Some(SectionType::VectorOutline) {
            continue;
        }
        let content_ops = ops.sub_ops(&sec_range.content_indices);
        let sp_ranges = content_ops.subpath_indices();
        for (sp_idx, sp_indices) in sp_ranges.iter().enumerate() {
            let sp_ops = content_ops.sub_ops(sp_indices);
            let key = SubpathKey {
                section_idx: sec_idx,
                subpath_idx: sp_idx,
            };
            all_subpaths.push((key, sp_ops));
        }
    }

    let mut assignments: HashMap<SubpathKey, Vec<ClipPoint>> = HashMap::new();

    for clip in clips {
        let mut best_key: Option<SubpathKey> = None;
        let mut best_dist_sq = f64::INFINITY;

        for (key, sp_ops) in &all_subpaths {
            let mut probe = sp_ops.copy();
            probe.preload_state();
            let geo = probe.to_geometry();
            if let Some((_, _, pt)) =
                crate::geo::query::find_closest_point_on_path_from_array(
                    &geo.data, clip.x, clip.y,
                )
            {
                let dx = clip.x - pt.0;
                let dy = clip.y - pt.1;
                let d_sq = dx * dx + dy * dy;
                if d_sq < best_dist_sq {
                    best_dist_sq = d_sq;
                    best_key = Some(key.clone());
                }
            }
        }

        if let Some(key) = best_key {
            let threshold = (clip.width * 2.0).powi(2);
            if best_dist_sq <= threshold {
                assignments.entry(key).or_default().push(clip.clone());
            }
        }
    }

    assignments
}

// ---------------------------------------------------------------------------
// Gap-mode clipping (bezier-aware)
// ---------------------------------------------------------------------------

fn clip_subpath_with_gaps(sub_ops: &Ops, clips: &[ClipPoint]) -> Ops {
    let gap_regions = compute_gap_regions_from_original(sub_ops, clips);
    if gap_regions.is_empty() {
        return sub_ops.copy();
    }

    let mut result = Ops::new();
    let mut accum_dist = 0.0;
    let mut last_pos: Option<Point3D> = None;

    for i in 0..sub_ops.len() {
        let ct = sub_ops.command_type(i);
        let cat = sub_ops.category(i);

        if ct == CommandType::MoveTo {
            result.commands.push(sub_ops.commands[i].clone());
            last_pos = Some(sub_ops.endpoint(i));
            accum_dist = 0.0;
            continue;
        }

        if cat != CommandCategory::Moving {
            if !in_any_gap(accum_dist, &gap_regions) {
                result.commands.push(sub_ops.commands[i].clone());
            }
            continue;
        }

        let end_pt = sub_ops.endpoint(i);
        let start_pt = match last_pos {
            Some(p) => p,
            None => {
                result.commands.push(sub_ops.commands[i].clone());
                last_pos = Some(end_pt);
                continue;
            }
        };

        let seg_len = if ct == CommandType::BezierTo {
            let (control1, control2) = bezier_params(sub_ops, i);
            bezier_arc_length_2d(start_pt, control1, control2, end_pt)
        } else {
            distance_2d(start_pt, end_pt)
        };

        let seg_start = accum_dist;
        let seg_end = accum_dist + seg_len;

        if seg_len < 1e-9 {
            last_pos = Some(end_pt);
            accum_dist += seg_len;
            continue;
        }

        let kept = compute_kept_ranges(seg_start, seg_end, &gap_regions);

        if let Some(kept) = kept {
            for (k_start, k_end) in kept {
                let d_start = k_start - seg_start;
                let d_end = k_end - seg_start;

                if ct == CommandType::BezierTo {
                    let (control1, control2) = bezier_params(sub_ops, i);
                    let t_start = bezier_distance_to_t(
                        start_pt, control1, control2, end_pt, d_start,
                    );
                    let t_end = bezier_distance_to_t(
                        start_pt, control1, control2, end_pt, d_end,
                    );
                    let sub = extract_bezier_subsegment_3d(
                        start_pt, control1, control2, end_pt, t_start, t_end,
                    );

                    let last_end = get_last_moving_end(&result);
                    if let Some(le) = last_end {
                        if distance_2d(le, sub.0) > 1e-6 {
                            result.move_to(sub.0 .0, sub.0 .1, sub.0 .2, None);
                        }
                    }
                    result.bezier_to(sub.1, sub.2, sub.3, None);
                } else {
                    let t_s = d_start / seg_len;
                    let t_e = d_end / seg_len;
                    let start_pt_interp =
                        interpolate_point(start_pt, end_pt, t_s);
                    let end_pt_interp =
                        interpolate_point(start_pt, end_pt, t_e);

                    let last_end = get_last_moving_end(&result);
                    if let Some(le) = last_end {
                        if distance_2d(le, start_pt_interp) > 1e-6 {
                            result.move_to(
                                start_pt_interp.0,
                                start_pt_interp.1,
                                start_pt_interp.2,
                                None,
                            );
                        }
                    }
                    result.line_to(
                        end_pt_interp.0,
                        end_pt_interp.1,
                        end_pt_interp.2,
                        None,
                    );
                }
            }
        } else {
            result.commands.push(sub_ops.commands[i].clone());
        }

        accum_dist += seg_len;
        last_pos = Some(end_pt);
    }

    let orig_endpoint = get_last_moving_end(sub_ops);
    if let Some(orig) = orig_endpoint {
        let last_end = get_last_moving_end(&result);
        if last_end.is_none_or(|end| distance_2d(end, orig) > 1e-6) {
            result.move_to(orig.0, orig.1, orig.2, None);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Power-mode insertion
// ---------------------------------------------------------------------------

fn insert_power_commands(
    sub_ops: &Ops,
    clips: &[ClipPoint],
    tab_power: f64,
    original_power: f64,
) -> Ops {
    let has_curves = sub_ops.commands.iter().any(|n| {
        matches!(
            n.category,
            OpCategory::Moving {
                cmd: MoveCmd::BezierTo { .. },
                ..
            } | OpCategory::Moving {
                cmd: MoveCmd::QuadraticBezierTo { .. },
                ..
            }
        )
    });

    if has_curves {
        return insert_power_commands_curve_aware(
            sub_ops,
            clips,
            tab_power,
            original_power,
        );
    }

    let mut temp_ops = sub_ops.copy();
    temp_ops.preload_state();
    temp_ops.linearize_all();

    if temp_ops.len() < 2 {
        return sub_ops.copy();
    }

    let geo_indices: Vec<usize> = (0..temp_ops.len())
        .filter(|&i| {
            let ct = temp_ops.command_type(i);
            ct == CommandType::MoveTo || ct == CommandType::LineTo
        })
        .collect();

    if geo_indices.len() < 2 {
        return sub_ops.copy();
    }

    let tab_regions =
        compute_tab_regions_from_linearized(&temp_ops, &geo_indices, clips);
    if tab_regions.is_empty() {
        return sub_ops.copy();
    }

    build_commands_with_power(
        &temp_ops,
        &geo_indices,
        &tab_regions,
        tab_power,
        original_power,
    )
}

fn insert_power_commands_curve_aware(
    sub_ops: &Ops,
    clips: &[ClipPoint],
    tab_power: f64,
    original_power: f64,
) -> Ops {
    let tab_regions = compute_gap_regions_from_original(sub_ops, clips);
    if tab_regions.is_empty() {
        return sub_ops.copy();
    }
    let mut tab_regions = tab_regions;
    tab_regions.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut result = Ops::new();
    let mut accum_dist = 0.0;
    let mut last_pos: Option<Point3D> = None;
    let mut current_power = original_power;

    for i in 0..sub_ops.len() {
        let ct = sub_ops.command_type(i);
        let cat = sub_ops.category(i);

        if ct == CommandType::MoveTo {
            result.commands.push(sub_ops.commands[i].clone());
            last_pos = Some(sub_ops.endpoint(i));
            accum_dist = 0.0;
            continue;
        }

        if ct == CommandType::SetPower {
            result.commands.push(sub_ops.commands[i].clone());
            continue;
        }

        if cat != CommandCategory::Moving {
            result.commands.push(sub_ops.commands[i].clone());
            continue;
        }

        let end_pt = sub_ops.endpoint(i);
        let start_pt = match last_pos {
            Some(p) => p,
            None => {
                result.commands.push(sub_ops.commands[i].clone());
                last_pos = Some(end_pt);
                continue;
            }
        };

        let seg_len = if ct == CommandType::BezierTo {
            let (control1, control2) = bezier_params(sub_ops, i);
            bezier_arc_length_2d(start_pt, control1, control2, end_pt)
        } else {
            distance_2d(start_pt, end_pt)
        };

        let seg_start = accum_dist;
        let seg_end = accum_dist + seg_len;

        if seg_len < 1e-9 {
            last_pos = Some(end_pt);
            accum_dist += seg_len;
            continue;
        }

        let events = collect_events(seg_start, seg_end, &tab_regions);

        if events.is_empty() {
            result.commands.push(sub_ops.commands[i].clone());
        } else if ct == CommandType::BezierTo {
            let (control1, control2) = bezier_params(sub_ops, i);
            split_bezier_with_power(
                &mut result,
                start_pt,
                control1,
                control2,
                end_pt,
                seg_start,
                &events,
                tab_power,
                original_power,
                &mut current_power,
            );
        } else {
            for (event_dist, event_type) in &events {
                if *event_dist > seg_start + 1e-9 {
                    let t = (*event_dist - seg_start) / seg_len;
                    let split_pt = interpolate_point(start_pt, end_pt, t);
                    result.line_to(split_pt.0, split_pt.1, split_pt.2, None);
                }
                let target = match event_type {
                    EventType::Enter => tab_power,
                    EventType::Exit => original_power,
                };
                if (current_power - target).abs() > 1e-9 {
                    result.set_power(target);
                    current_power = target;
                }
            }
            result.commands.push(sub_ops.commands[i].clone());
        }

        accum_dist += seg_len;
        last_pos = Some(end_pt);
    }

    result
}

#[allow(clippy::too_many_arguments)]
fn split_bezier_with_power(
    result: &mut Ops,
    p0: Point3D,
    control1: Point3D,
    control2: Point3D,
    p1: Point3D,
    seg_start: f64,
    events: &[(f64, EventType)],
    tab_power: f64,
    original_power: f64,
    current_power: &mut f64,
) {
    let mut sub_segments: Vec<(f64, f64, f64)> = Vec::new();
    let mut last_t = 0.0;
    let mut last_power = *current_power;

    for (event_dist, event_type) in events {
        let d = event_dist - seg_start;
        let t_event = bezier_distance_to_t(p0, control1, control2, p1, d);
        if t_event > last_t + 1e-9 {
            sub_segments.push((last_t, t_event, last_power));
        }
        last_power = match event_type {
            EventType::Enter => tab_power,
            EventType::Exit => original_power,
        };
        last_t = t_event;
    }

    if last_t < 1.0 - 1e-9 {
        sub_segments.push((last_t, 1.0, last_power));
    }

    for (t_start, t_end, power) in &sub_segments {
        if (power - *current_power).abs() > 1e-9 {
            result.set_power(*power);
            *current_power = *power;
        }
        let sub = extract_bezier_subsegment_3d(
            p0, control1, control2, p1, *t_start, *t_end,
        );
        result.bezier_to(sub.1, sub.2, sub.3, None);
    }
}

// ---------------------------------------------------------------------------
// Gap region computation
// ---------------------------------------------------------------------------

fn compute_gap_regions_from_original(
    sub_ops: &Ops,
    clips: &[ClipPoint],
) -> Vec<TabRegion> {
    let mut temp_ops = sub_ops.copy();
    temp_ops.preload_state();
    let geo = temp_ops.to_geometry();

    if geo.data.is_empty() {
        return Vec::new();
    }

    let mut gap_regions: Vec<TabRegion> = Vec::new();
    for clip in clips {
        let closest = crate::geo::query::find_closest_point_on_path_from_array(
            &geo.data, clip.x, clip.y,
        );
        let (seg_idx, t, pt) = match closest {
            Some(v) => v,
            None => continue,
        };

        let dx = clip.x - pt.0;
        let dy = clip.y - pt.1;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq > (clip.width * 2.0).powi(2) {
            continue;
        }

        let hit_dist = compute_hit_distance_original(sub_ops, seg_idx, t);
        let hit_dist = match hit_dist {
            Some(d) => d,
            None => continue,
        };

        gap_regions.push(TabRegion {
            start: 0.0_f64.max(hit_dist - clip.width / 2.0),
            end: hit_dist + clip.width / 2.0,
        });
    }

    gap_regions
}

fn compute_tab_regions_from_linearized(
    temp_ops: &Ops,
    geo_indices: &[usize],
    clips: &[ClipPoint],
) -> Vec<TabRegion> {
    let mut tab_regions: Vec<TabRegion> = Vec::new();

    let geo_data = {
        let mut probe = temp_ops.copy();
        probe.preload_state();
        probe.to_geometry()
    };

    for clip in clips {
        let closest = crate::geo::query::find_closest_point_on_path_from_array(
            &geo_data.data,
            clip.x,
            clip.y,
        );
        let (seg_idx, t, pt) = match closest {
            Some(v) => v,
            None => continue,
        };

        let dx = clip.x - pt.0;
        let dy = clip.y - pt.1;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq > (clip.width * 2.0).powi(2) {
            continue;
        }

        let hit_dist =
            compute_hit_distance_linearized(temp_ops, geo_indices, seg_idx, t);
        let hit_dist = match hit_dist {
            Some(d) => d,
            None => continue,
        };

        tab_regions.push(TabRegion {
            start: 0.0_f64.max(hit_dist - clip.width / 2.0),
            end: hit_dist + clip.width / 2.0,
        });
    }

    tab_regions
}

fn compute_hit_distance_original(
    sub_ops: &Ops,
    target_geo_idx: usize,
    target_t: f64,
) -> Option<f64> {
    let mut accum = 0.0;
    let mut last_pos: Option<Point3D> = None;
    let mut geo_idx = 0;

    for i in 0..sub_ops.len() {
        let cat = sub_ops.category(i);
        if cat != CommandCategory::Moving {
            continue;
        }

        let ct = sub_ops.command_type(i);
        let end_pt = sub_ops.endpoint(i);

        if ct == CommandType::MoveTo {
            last_pos = Some(end_pt);
            if geo_idx == target_geo_idx {
                return Some(accum);
            }
            geo_idx += 1;
            continue;
        }

        let start = match last_pos {
            Some(p) => p,
            None => {
                last_pos = Some(end_pt);
                geo_idx += 1;
                continue;
            }
        };

        let seg_len = if ct == CommandType::BezierTo {
            let (control1, control2) = bezier_params(sub_ops, i);
            bezier_arc_length_2d(start, control1, control2, end_pt)
        } else {
            distance_2d(start, end_pt)
        };

        if geo_idx == target_geo_idx {
            return Some(accum + target_t * seg_len);
        }

        accum += seg_len;
        last_pos = Some(end_pt);
        geo_idx += 1;
    }

    None
}

fn compute_hit_distance_linearized(
    temp_ops: &Ops,
    geo_indices: &[usize],
    segment_idx: usize,
    t: f64,
) -> Option<f64> {
    if segment_idx >= geo_indices.len() {
        return None;
    }

    let mut hit_dist = 0.0;
    let mut last_pos = temp_ops.endpoint(geo_indices[0]);

    for &idx in geo_indices.iter().take(segment_idx).skip(1) {
        let ct = temp_ops.command_type(idx);
        if ct == CommandType::MoveTo {
            last_pos = temp_ops.endpoint(idx);
        } else if ct == CommandType::LineTo {
            let end = temp_ops.endpoint(idx);
            hit_dist += distance_2d(last_pos, end);
            last_pos = end;
        }
    }

    let hit_idx = geo_indices[segment_idx];
    if temp_ops.command_type(hit_idx) == CommandType::LineTo {
        let end = temp_ops.endpoint(hit_idx);
        let dist = distance_2d(last_pos, end);
        hit_dist += t * dist;
        return Some(hit_dist);
    }

    None
}

// ---------------------------------------------------------------------------
// Power-mode helpers for linearized ops
// ---------------------------------------------------------------------------

fn build_commands_with_power(
    temp_ops: &Ops,
    geo_indices: &[usize],
    tab_regions: &[TabRegion],
    tab_power: f64,
    original_power: f64,
) -> Ops {
    let mut result = Ops::new();
    result.commands.push(temp_ops.commands[0].clone());

    let mut accum_dist = 0.0;
    let mut current_power = original_power;
    let mut last_pos = temp_ops.endpoint(geo_indices[0]);

    for i in 1..temp_ops.len() {
        let ct = temp_ops.command_type(i);
        if ct == CommandType::LineTo {
            let p2 = temp_ops.endpoint(i);
            let seg_len = distance_2d(last_pos, p2);

            if seg_len < 1e-9 {
                last_pos = p2;
                continue;
            }

            let seg_start = accum_dist;
            let seg_end = accum_dist + seg_len;

            let events = collect_events(seg_start, seg_end, tab_regions);

            if !events.is_empty() {
                process_segment_events(
                    &mut result,
                    temp_ops,
                    i,
                    last_pos,
                    p2,
                    seg_len,
                    seg_start,
                    &events,
                    tab_power,
                    original_power,
                    &mut current_power,
                );
            } else {
                result.commands.push(temp_ops.commands[i].clone());
            }

            last_pos = p2;
            accum_dist += seg_len;
        } else if ct == CommandType::MoveTo {
            result.commands.push(temp_ops.commands[i].clone());
            last_pos = temp_ops.endpoint(i);
        } else {
            result.commands.push(temp_ops.commands[i].clone());
        }
    }

    result
}

#[derive(Clone, Copy, Debug)]
enum EventType {
    Enter,
    Exit,
}

fn collect_events(
    seg_start: f64,
    seg_end: f64,
    tab_regions: &[TabRegion],
) -> Vec<(f64, EventType)> {
    let mut events: Vec<(f64, EventType)> = Vec::new();
    for region in tab_regions {
        if region.end <= seg_start || region.start >= seg_end {
            continue;
        }
        let enter = region.start.max(seg_start);
        let exit_ = region.end.min(seg_end);
        events.push((enter, EventType::Enter));
        events.push((exit_, EventType::Exit));
    }
    events.sort_by(|a, b| {
        a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    events
}

#[allow(clippy::too_many_arguments)]
fn process_segment_events(
    result: &mut Ops,
    src_ops: &Ops,
    src_idx: usize,
    p1: Point3D,
    p2: Point3D,
    seg_len: f64,
    seg_start: f64,
    events: &[(f64, EventType)],
    tab_power: f64,
    original_power: f64,
    current_power: &mut f64,
) {
    let mut last_dist = seg_start;

    for (event_dist, event_type) in events {
        if *event_dist > last_dist + 1e-9 {
            let t = (*event_dist - seg_start) / seg_len;
            let split_pt = interpolate_point(p1, p2, t);
            result.line_to(split_pt.0, split_pt.1, split_pt.2, None);
        }

        let target = match event_type {
            EventType::Enter => tab_power,
            EventType::Exit => original_power,
        };
        if (*current_power - target).abs() > 1e-9 {
            result.set_power(target);
            *current_power = target;
        }

        last_dist = *event_dist;
    }

    if seg_start + seg_len > last_dist + 1e-9 {
        result.commands.push(src_ops.commands[src_idx].clone());
    }
}

// ---------------------------------------------------------------------------
// Bezier utilities
// ---------------------------------------------------------------------------

/// Approximate the arc length of a cubic Bezier curve.
pub fn bezier_arc_length_2d(
    p0: Point3D,
    control1: Point3D,
    control2: Point3D,
    p1: Point3D,
) -> f64 {
    let num_samples = 200;
    let mut length = 0.0;
    let mut prev = p0;
    for i in 1..=num_samples {
        let t = i as f64 / num_samples as f64;
        let pt = eval_bezier(p0, control1, control2, p1, t);
        let dx = pt.0 - prev.0;
        let dy = pt.1 - prev.1;
        length += (dx * dx + dy * dy).sqrt();
        prev = pt;
    }
    length
}

/// Convert a target distance along the Bezier to parameter t ∈ [0, 1].
pub fn bezier_distance_to_t(
    p0: Point3D,
    control1: Point3D,
    control2: Point3D,
    p1: Point3D,
    target_dist: f64,
) -> f64 {
    let num_samples = 200;
    let mut accum = 0.0;
    let mut prev = p0;
    for i in 1..=num_samples {
        let t = i as f64 / num_samples as f64;
        let pt = eval_bezier(p0, control1, control2, p1, t);
        let dx = pt.0 - prev.0;
        let dy = pt.1 - prev.1;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if accum + seg_len >= target_dist - 1e-9 {
            if seg_len < 1e-9 {
                return (i as f64 / num_samples as f64).min(1.0);
            }
            let local_t = ((target_dist - accum) / seg_len).clamp(0.0, 1.0);
            return ((i - 1) as f64 + local_t) / num_samples as f64;
        }
        accum += seg_len;
        prev = pt;
    }
    1.0
}

/// Extract a sub-segment [t_start, t_end] from a cubic Bezier curve.
pub fn extract_bezier_subsegment_3d(
    p0: Point3D,
    control1: Point3D,
    control2: Point3D,
    p1: Point3D,
    t_start: f64,
    t_end: f64,
) -> (Point3D, Point3D, Point3D, Point3D) {
    if t_start <= 1e-9 && t_end >= 1.0 - 1e-9 {
        return (p0, control1, control2, p1);
    }
    if t_end >= 1.0 - 1e-9 {
        let (_, right) =
            subdivide_bezier_3d(p0, control1, control2, p1, t_start);
        return right;
    }
    if t_start <= 1e-9 {
        let (left, _) = subdivide_bezier_3d(p0, control1, control2, p1, t_end);
        return left;
    }
    let (_, right) = subdivide_bezier_3d(p0, control1, control2, p1, t_start);
    let s = (t_end - t_start) / (1.0 - t_start);
    let (sub_left, _) =
        subdivide_bezier_3d(right.0, right.1, right.2, right.3, s);
    sub_left
}

#[allow(clippy::type_complexity)]
fn subdivide_bezier_3d(
    a: Point3D,
    b: Point3D,
    c: Point3D,
    d: Point3D,
    t: f64,
) -> (
    (Point3D, Point3D, Point3D, Point3D),
    (Point3D, Point3D, Point3D, Point3D),
) {
    let m01 = lerp_3d(a, b, t);
    let m12 = lerp_3d(b, c, t);
    let m23 = lerp_3d(c, d, t);
    let m0112 = lerp_3d(m01, m12, t);
    let m1223 = lerp_3d(m12, m23, t);
    let sp = lerp_3d(m0112, m1223, t);
    ((a, m01, m0112, sp), (sp, m1223, m23, d))
}

fn eval_bezier(
    p0: Point3D,
    control1: Point3D,
    control2: Point3D,
    p1: Point3D,
    t: f64,
) -> Point3D {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    Point3D(
        mt3 * p0.0
            + 3.0 * mt2 * t * control1.0
            + 3.0 * mt * t2 * control2.0
            + t3 * p1.0,
        mt3 * p0.1
            + 3.0 * mt2 * t * control1.1
            + 3.0 * mt * t2 * control2.1
            + t3 * p1.1,
        mt3 * p0.2
            + 3.0 * mt2 * t * control1.2
            + 3.0 * mt * t2 * control2.2
            + t3 * p1.2,
    )
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

fn lerp_3d(a: Point3D, b: Point3D, t: f64) -> Point3D {
    Point3D(
        a.0 + t * (b.0 - a.0),
        a.1 + t * (b.1 - a.1),
        a.2 + t * (b.2 - a.2),
    )
}

fn interpolate_point(p1: Point3D, p2: Point3D, t: f64) -> Point3D {
    Point3D(
        p1.0 + t * (p2.0 - p1.0),
        p1.1 + t * (p2.1 - p1.1),
        p1.2 + t * (p2.2 - p1.2),
    )
}

fn distance_2d(a: Point3D, b: Point3D) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}

fn bezier_params(ops: &Ops, idx: usize) -> (Point3D, Point3D) {
    if let OpCategory::Moving {
        cmd: MoveCmd::BezierTo { control1, control2 },
        ..
    } = &ops.commands[idx].category
    {
        (*control1, *control2)
    } else {
        (Point3D(0.0, 0.0, 0.0), Point3D(0.0, 0.0, 0.0))
    }
}

fn get_last_moving_end(ops: &Ops) -> Option<Point3D> {
    for i in (0..ops.len()).rev() {
        if ops.commands[i].is_moving() {
            return Some(ops.commands[i].end_point());
        }
    }
    None
}

fn compute_kept_ranges(
    seg_start: f64,
    seg_end: f64,
    gap_regions: &[TabRegion],
) -> Option<Vec<(f64, f64)>> {
    let overlapping: Vec<(f64, f64)> = gap_regions
        .iter()
        .filter_map(|r| {
            if r.start < seg_end && r.end > seg_start {
                Some((r.start.max(seg_start), r.end.min(seg_end)))
            } else {
                None
            }
        })
        .collect();

    if overlapping.is_empty() {
        return None;
    }

    let mut kept = vec![(seg_start, seg_end)];
    for (g_start, g_end) in overlapping {
        let mut new_kept = Vec::new();
        for (k_start, k_end) in kept {
            if k_start < g_start {
                new_kept.push((k_start, k_end.min(g_start)));
            }
            if k_end > g_end {
                new_kept.push((k_start.max(g_end), k_end));
            }
        }
        kept = new_kept;
    }
    Some(kept)
}

fn in_any_gap(dist: f64, gaps: &[TabRegion]) -> bool {
    gaps.iter().any(|g| dist >= g.start && dist <= g.end)
}
