use super::container::Ops;
use super::enums::CommandType;
use super::types::{MoveCmd, OpCategory, OpNode};
use crate::constants::{EPSILON_COLLINEAR, EPSILON_GAP_CLOSE};
use crate::geo::algo::clipping::{
    clip_line_segment_with_polygons, clip_line_segment_with_rect,
    subtract_polygons_from_line_segment,
};
use crate::geo::algo::fitting::fit_points_with_primitives;
use crate::geo::algo::interp::{
    compute_segment_delta, compute_t_range, slice_scanline_data,
};
use crate::geo::shape::arc::is_arc_inside_polygons;
use crate::geo::shape::bezier::is_bezier_inside_polygons;
use crate::types::Command;
use crate::types::{Point, Point3D, Polygon, Rect};

/// Add a clipped line segment to `new_ops`, inserting a move-to if the pen position
/// does not match the segment start.
///
/// - `new_ops`: Target ops sequence.
/// - `segment`: Optional `(start, end)` pair; `None` disables the pen.
/// - `pen_pos`: Current pen position (mutated in place).
fn add_clipped_segment(
    new_ops: &mut Ops,
    segment: Option<(Point3D, Point3D)>,
    pen_pos: &mut Option<Point3D>,
) {
    if let Some((p1, p2)) = segment {
        if needs_move_to(*pen_pos, p1) {
            new_ops.move_to(p1.x, p1.y, p1.z, None);
        }
        new_ops.line_to(p2.x, p2.y, p2.z, None);
        *pen_pos = Some(p2);
    } else {
        *pen_pos = None;
    }
}

impl Ops {
    /// Clip all commands to the interior of an axis-aligned rectangle.
    ///
    /// - `rect`: The clipping rectangle.
    /// - Returns: A new `Ops` containing only the portions of commands inside `rect`.
    pub fn clip_rect(&self, rect: Rect) -> Self {
        let mut new_ops = Ops::new();
        if self.commands.is_empty() {
            return new_ops;
        }

        let mut last_point: Point3D = Point3D::new(0.0, 0.0, 0.0);
        let mut clipped_pen_pos: Option<Point3D> = None;

        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                match cmd {
                    MoveCmd::ScanLine { power_values } => {
                        let clipped =
                            clip_line_segment_with_rect(last_point, *end, rect);
                        let kept: Vec<(Point3D, Point3D)> =
                            clipped.into_iter().collect();
                        clipped_pen_pos = append_clipped_scanline(
                            &mut new_ops,
                            last_point,
                            *end,
                            power_values,
                            &kept,
                            clipped_pen_pos,
                        );
                        last_point = *end;
                    }
                    MoveCmd::MoveTo => {
                        last_point = *end;
                        clipped_pen_pos = None;
                    }
                    _ => {
                        let linearized = crate::ops::linearize::linearize_node(
                            node, last_point,
                        );
                        let mut p_seg_start = last_point;
                        for lnode in &linearized.commands {
                            let p_seg_end = lnode.end_point();
                            let clipped = clip_line_segment_with_rect(
                                p_seg_start,
                                p_seg_end,
                                rect,
                            );
                            add_clipped_segment(
                                &mut new_ops,
                                clipped,
                                &mut clipped_pen_pos,
                            );
                            p_seg_start = p_seg_end;
                        }
                        last_point = *end;
                    }
                }
            } else {
                new_ops.commands.push(node.clone());
            }
        }

        new_ops
    }

    /// Remove (erase) the interior of the given polygons from all commands.
    ///
    /// - `regions`: Polygons whose interiors should be removed.
    /// - Returns: `self` for method chaining.
    pub fn subtract_regions(&mut self, regions: &[Polygon]) -> &mut Self {
        if regions.is_empty() || self.commands.is_empty() {
            return self;
        }

        let mut new_ops = Ops::new();
        let mut last_point: Point3D = Point3D::new(0.0, 0.0, 0.0);
        let mut pen_pos: Option<Point3D> = None;

        let first_move_idx = self
            .commands
            .iter()
            .position(|node| node.is_moving())
            .unwrap_or(self.commands.len());

        for node in &self.commands[..first_move_idx] {
            new_ops.commands.push(node.clone());
        }

        for node in &self.commands[first_move_idx..] {
            if let OpCategory::Moving { end, cmd } = &node.category {
                match cmd {
                    MoveCmd::MoveTo => {
                        last_point = *end;
                        pen_pos = None;
                    }
                    MoveCmd::ScanLine { power_values } => {
                        let kept = subtract_polygons_from_line_segment(
                            last_point, *end, regions,
                        );
                        pen_pos = append_clipped_scanline(
                            &mut new_ops,
                            last_point,
                            *end,
                            power_values,
                            &kept,
                            pen_pos,
                        );
                        last_point = *end;
                    }
                    _ => {
                        let linearized = crate::ops::linearize::linearize_node(
                            node, last_point,
                        );
                        let mut p_seg_start = last_point;
                        for lnode in &linearized.commands {
                            let p_seg_end = lnode.end_point();
                            let kept = subtract_polygons_from_line_segment(
                                p_seg_start,
                                p_seg_end,
                                regions,
                            );
                            for (sub_p1, sub_p2) in kept {
                                if needs_move_to(pen_pos, sub_p1) {
                                    new_ops.move_to(
                                        sub_p1.x, sub_p1.y, sub_p1.z, None,
                                    );
                                }
                                new_ops.line_to(
                                    sub_p2.x, sub_p2.y, sub_p2.z, None,
                                );
                                pen_pos = Some(sub_p2);
                            }
                            p_seg_start = p_seg_end;
                        }
                        last_point = *end;
                    }
                }
            } else {
                new_ops.commands.push(node.clone());
            }
        }

        self.commands = new_ops.commands;
        self.invalidate_time_cache();
        if !self.commands.is_empty() {
            for node in self.commands.iter().rev() {
                if let OpCategory::Moving {
                    end,
                    cmd: MoveCmd::MoveTo,
                } = &node.category
                {
                    self.last_move_to = *end;
                    break;
                }
            }
        }
        self
    }

    /// Clip commands to the interior of the given polygons.
    ///
    /// Commands outside the regions are discarded. Arcs and Beziers are linearized
    /// during clipping.
    ///
    /// - `regions`: Polygons defining the clipping regions.
    /// - `_tolerance`: Not currently used.
    /// - Returns: `self` for method chaining.
    pub fn clip_to_regions(
        &mut self,
        regions: &[Polygon],
        _tolerance: f64,
    ) -> &mut Self {
        let valid_regions: Vec<Polygon> =
            regions.iter().filter(|r| r.len() >= 3).cloned().collect();
        if valid_regions.is_empty() || self.commands.is_empty() {
            return self;
        }

        let mut new_ops = Ops::new();
        let mut last_point: Point3D = Point3D::new(0.0, 0.0, 0.0);
        let mut pen_pos: Option<Point3D> = None;

        let first_move_idx = self
            .commands
            .iter()
            .position(|node| node.is_moving())
            .unwrap_or(self.commands.len());

        for node in &self.commands[..first_move_idx] {
            new_ops.commands.push(node.clone());
        }

        for node in &self.commands[first_move_idx..] {
            if let OpCategory::Moving { end, cmd } = &node.category {
                match cmd {
                    MoveCmd::MoveTo => {
                        last_point = *end;
                        pen_pos = None;
                    }
                    MoveCmd::ScanLine { power_values } => {
                        let kept = clip_line_segment_with_polygons(
                            last_point,
                            *end,
                            &valid_regions,
                        );
                        pen_pos = append_clipped_scanline(
                            &mut new_ops,
                            last_point,
                            *end,
                            power_values,
                            &kept,
                            pen_pos,
                        );
                        last_point = *end;
                    }
                    _ => {
                        let linearized = crate::ops::linearize::linearize_node(
                            node, last_point,
                        );
                        let mut p_seg_start = last_point;
                        for lnode in &linearized.commands {
                            let p_seg_end = lnode.end_point();
                            let kept = clip_line_segment_with_polygons(
                                p_seg_start,
                                p_seg_end,
                                &valid_regions,
                            );
                            for (sub_p1, sub_p2) in kept {
                                if needs_move_to(pen_pos, sub_p1) {
                                    new_ops.move_to(
                                        sub_p1.x, sub_p1.y, sub_p1.z, None,
                                    );
                                }
                                new_ops.line_to(
                                    sub_p2.x, sub_p2.y, sub_p2.z, None,
                                );
                                pen_pos = Some(sub_p2);
                            }
                            p_seg_start = p_seg_end;
                        }
                        last_point = *end;
                    }
                }
            } else {
                new_ops.commands.push(node.clone());
            }
        }

        self.commands = new_ops.commands;
        self.invalidate_time_cache();
        if !self.commands.is_empty() {
            for node in self.commands.iter().rev() {
                if let OpCategory::Moving {
                    end,
                    cmd: MoveCmd::MoveTo,
                } = &node.category
                {
                    self.last_move_to = *end;
                    break;
                }
            }
        }
        self
    }

    /// Cut a gap of `width` at the closest point on the path to `(x, y)`.
    ///
    /// - `x`: X coordinate of the hit location.
    /// - `y`: Y coordinate of the hit location.
    /// - `width`: Width of the gap to remove.
    /// - Returns: `true` if a gap was successfully cut.
    pub fn clip_at(&mut self, x: f64, y: f64, width: f64) -> bool {
        if width <= EPSILON_GAP_CLOSE {
            return false;
        }

        let (command_index, _) = match find_hit_command(self, x, y, width) {
            Some(v) => v,
            None => return false,
        };

        let (start_idx, end_idx) = find_subpath_bounds(self, command_index);

        if start_idx >= self.len() {
            return false;
        }
        if !self.commands[start_idx].is_moving() {
            return false;
        }

        let subpath_indices: Vec<usize> = (start_idx..end_idx).collect();
        let mut temp_ops = self.sub_ops(&subpath_indices);
        temp_ops.preload_state();
        temp_ops.linearize_all();

        if temp_ops.len() < 2 {
            return false;
        }

        let linear_geo_cmds: Vec<usize> = (0..temp_ops.len())
            .filter(|&j| {
                matches!(
                    temp_ops.commands[j].category,
                    OpCategory::Moving {
                        cmd: MoveCmd::MoveTo,
                        ..
                    } | OpCategory::Moving {
                        cmd: MoveCmd::LineTo,
                        ..
                    }
                )
            })
            .collect();

        if linear_geo_cmds.len() < 2 {
            return false;
        }

        let linear_temp_geo = temp_ops.to_geometry();
        let linear_closest =
            crate::geo::query::find_closest_point_on_path_from_array(
                &linear_temp_geo.data,
                x,
                y,
            );
        let (linear_segment_idx, linear_t2, _) = match linear_closest {
            Some(v) => v,
            None => return false,
        };

        let hit_dist = accumulate_distance_to_hit(
            &temp_ops,
            &linear_geo_cmds,
            linear_segment_idx,
            linear_t2,
        );

        let half_width = width / 2.0;
        let gap_start_dist = hit_dist - half_width;
        let gap_end_dist = hit_dist + half_width;

        let gaps = &[(gap_start_dist, gap_end_dist)];
        let mut new_subpath = build_clipped_subpath(&temp_ops, gaps);

        // For a closed subpath, if the gap extends past the total path
        // length, wrap the excess around to the beginning of the path.
        let total_length = accumulate_distance_to_hit(
            &temp_ops,
            &linear_geo_cmds,
            linear_geo_cmds.len() - 1,
            1.0,
        );
        if gap_start_dist < 0.0 || gap_end_dist > total_length {
            let first_pt = temp_ops.commands[0].end_point();
            let last_pt = temp_ops
                .commands
                .iter()
                .rev()
                .find(|n| n.is_moving())
                .map(|n| n.end_point())
                .unwrap_or(first_pt);
            let is_closed = (first_pt.x - last_pt.x).powi(2)
                + (first_pt.y - last_pt.y).powi(2)
                < EPSILON_GAP_CLOSE * EPSILON_GAP_CLOSE;
            if is_closed {
                let mut wrap_gaps: Vec<(f64, f64)> = Vec::new();
                if gap_start_dist < 0.0 {
                    wrap_gaps
                        .push((total_length + gap_start_dist, total_length));
                }
                if gap_end_dist > total_length {
                    wrap_gaps.push((0.0, gap_end_dist - total_length));
                }
                wrap_gaps.push((
                    gap_start_dist.max(0.0),
                    gap_end_dist.min(total_length),
                ));
                new_subpath = build_clipped_subpath(&temp_ops, &wrap_gaps);
            }
        }

        let original_endpoint = self.commands
            [if end_idx > 0 { end_idx - 1 } else { 0 }]
        .end_point();
        let mut new_endpoint: Option<Point3D> = None;
        if !new_subpath.is_empty() {
            for node in new_subpath.commands.iter().rev() {
                if node.is_moving() {
                    new_endpoint = Some(node.end_point());
                    break;
                }
            }
        }

        let endpoint_match = match new_endpoint {
            Some(ne) => {
                let d =
                    (original_endpoint.x - ne.x, original_endpoint.y - ne.y);
                (d.0 * d.0 + d.1 * d.1).sqrt() <= EPSILON_GAP_CLOSE
            }
            None => false,
        };

        if !endpoint_match {
            // For a closed subpath where the gap wraps around the seam,
            // the new subpath ends at the correct position (before the
            // seam). Adding a travel move back to the seam would undo
            // the wrapping. Check whether the seam falls inside the gap.
            let first_pt = temp_ops.commands[0].end_point();
            let last_pt = temp_ops
                .commands
                .iter()
                .rev()
                .find(|n| n.is_moving())
                .map(|n| n.end_point())
                .unwrap_or(first_pt);
            let subpath_is_closed = (first_pt.x - last_pt.x).powi(2)
                + (first_pt.y - last_pt.y).powi(2)
                < EPSILON_GAP_CLOSE * EPSILON_GAP_CLOSE;
            let seam_gapped = subpath_is_closed
                && (gap_start_dist < 0.0 || gap_end_dist > total_length);
            if !seam_gapped {
                new_subpath.move_to(
                    original_endpoint.x,
                    original_endpoint.y,
                    original_endpoint.z,
                    None,
                );
            }
        }

        let mut new_cmds = Vec::new();

        // First pass: preserve state/marker commands before the first moving command
        for j in 0..start_idx {
            new_cmds.push(self.commands[j].clone());
        }

        // Second pass: add the replacement subpath from the clipper output
        for j in 0..new_subpath.commands.len() {
            new_cmds.push(new_subpath.commands[j].clone());
        }

        // Third pass: preserve commands after the replaced segment
        for j in end_idx..self.commands.len() {
            new_cmds.push(self.commands[j].clone());
        }

        self.commands = new_cmds;
        self.invalidate_time_cache();
        true
    }

    /// Clip commands to polygon regions, preserving arcs and Beziers via refitting.
    ///
    /// Unlike `clip_to_regions`, this method attempts to fit arcs and Beziers
    /// back from the clipped linear segments.
    ///
    /// - `regions`: Polygons defining the clipping regions.
    /// - `tolerance`: Fit tolerance for primitive refitting.
    /// - Returns: `self` for method chaining.
    pub fn clip_ops_to_regions(
        &mut self,
        regions: &[Polygon],
        tolerance: f64,
    ) -> &mut Self {
        let valid_regions: Vec<Polygon> =
            regions.iter().filter(|r| r.len() >= 3).cloned().collect();
        if valid_regions.is_empty() {
            return self;
        }

        let mut new_ops = Ops::new();
        let mut last_point: Point3D = Point3D::new(0.0, 0.0, 0.0);
        let mut pen_pos: Option<Point3D> = None;

        let first_move_idx = self
            .commands
            .iter()
            .position(|node| node.is_moving())
            .unwrap_or(self.commands.len());

        for node in &self.commands[..first_move_idx] {
            new_ops.commands.push(node.clone());
        }

        for node in &self.commands[first_move_idx..] {
            if let OpCategory::Moving { end, cmd } = &node.category {
                match cmd {
                    MoveCmd::MoveTo => {
                        last_point = *end;
                        pen_pos = None;
                    }
                    MoveCmd::ScanLine { power_values } => {
                        let kept = clip_line_segment_with_polygons(
                            last_point,
                            *end,
                            &valid_regions,
                        );
                        pen_pos = append_clipped_scanline(
                            &mut new_ops,
                            last_point,
                            *end,
                            power_values,
                            &kept,
                            pen_pos,
                        );
                        last_point = *end;
                    }
                    MoveCmd::ArcTo { center, cw } => {
                        if is_arc_inside_polygons(
                            Point::new(last_point.x, last_point.y),
                            Point::new(end.x, end.y),
                            *center,
                            *cw,
                            &valid_regions,
                        ) {
                            if needs_move_to(pen_pos, last_point) {
                                new_ops.move_to(
                                    last_point.x,
                                    last_point.y,
                                    last_point.z,
                                    None,
                                );
                            }
                            new_ops.commands.push(node.clone());
                            pen_pos = Some(*end);
                            last_point = *end;
                        } else {
                            pen_pos = clip_and_refit_arc(
                                &mut new_ops,
                                node,
                                last_point,
                                pen_pos,
                                &valid_regions,
                                tolerance,
                            );
                            last_point = *end;
                        }
                    }
                    MoveCmd::BezierTo { control1, control2 } => {
                        if is_bezier_inside_polygons(
                            Point::new(last_point.x, last_point.y),
                            Point::new(control1.x, control1.y),
                            Point::new(control2.x, control2.y),
                            Point::new(end.x, end.y),
                            &valid_regions,
                        ) {
                            if needs_move_to(pen_pos, last_point) {
                                new_ops.move_to(
                                    last_point.x,
                                    last_point.y,
                                    last_point.z,
                                    None,
                                );
                            }
                            new_ops.commands.push(node.clone());
                            pen_pos = Some(*end);
                            last_point = *end;
                        } else {
                            pen_pos = clip_and_refit_bezier(
                                &mut new_ops,
                                node,
                                last_point,
                                pen_pos,
                                &valid_regions,
                                tolerance,
                            );
                            last_point = *end;
                        }
                    }
                    _ => {
                        let linearized = crate::ops::linearize::linearize_node(
                            node, last_point,
                        );
                        let mut p_seg_start = last_point;
                        for lnode in &linearized.commands {
                            let p_seg_end = lnode.end_point();
                            let kept_segments = clip_line_segment_with_polygons(
                                p_seg_start,
                                p_seg_end,
                                &valid_regions,
                            );
                            for (sub_p1, sub_p2) in kept_segments {
                                if needs_move_to(pen_pos, sub_p1) {
                                    new_ops.move_to(
                                        sub_p1.x, sub_p1.y, sub_p1.z, None,
                                    );
                                }
                                new_ops.line_to(
                                    sub_p2.x, sub_p2.y, sub_p2.z, None,
                                );
                                pen_pos = Some(sub_p2);
                            }
                            p_seg_start = p_seg_end;
                        }
                        last_point = *end;
                    }
                }
            } else {
                new_ops.commands.push(node.clone());
            }
        }

        self.commands = new_ops.commands;
        self.invalidate_time_cache();
        if !self.commands.is_empty() {
            for node in self.commands.iter().rev() {
                if let OpCategory::Moving {
                    end,
                    cmd: MoveCmd::MoveTo,
                } = &node.category
                {
                    self.last_move_to = *end;
                    break;
                }
            }
        }
        self
    }
}

/// Check whether the pen needs a move-to before drawing to `target`.
///
/// - `pen_pos`: Current pen position (`None` means pen is off).
/// - `target`: Desired target point.
/// - Returns: `true` if a move-to command is needed.
fn needs_move_to(pen_pos: Option<Point3D>, target: Point3D) -> bool {
    match pen_pos {
        Some(prev) => {
            let dx = target.x - prev.x;
            let dy = target.y - prev.y;
            (dx * dx + dy * dy).sqrt() > EPSILON_GAP_CLOSE
        }
        None => true,
    }
}

/// Append clipped scanline segments to `new_ops`, slicing power data to match.
///
/// - `new_ops`: Target ops sequence.
/// - `last_point`: Start of the original scanline.
/// - `end`: End of the original scanline.
/// - `scanline_data`: Original power data for the scanline.
/// - `kept_segments`: Clipped sub-segments to keep.
/// - `pen_pos`: Current pen position.
/// - Returns: Updated pen position.
fn append_clipped_scanline(
    new_ops: &mut Ops,
    last_point: Point3D,
    end: Point3D,
    scanline_data: &[u8],
    kept_segments: &[(Point3D, Point3D)],
    pen_pos: Option<Point3D>,
) -> Option<Point3D> {
    let delta = compute_segment_delta(last_point, end);
    let mut pen_pos = pen_pos;

    for (new_start, new_end) in kept_segments {
        let (t_start, t_end) =
            compute_t_range(last_point, *new_start, *new_end, &delta);
        let new_pv = slice_scanline_data(scanline_data, t_start, t_end);

        if !new_pv.is_empty() {
            if needs_move_to(pen_pos, *new_start) {
                new_ops.move_to(new_start.x, new_start.y, new_start.z, None);
            }
            new_ops.scan_to(new_end.x, new_end.y, new_end.z, new_pv, None);
            pen_pos = Some(*new_end);
        }
    }

    pen_pos
}

/// Find the command index and closest point for a hit test at `(x, y)`.
///
/// - `ops`: The ops sequence.
/// - `x`: X coordinate of the hit.
/// - `y`: Y coordinate of the hit.
/// - `width`: Hit radius threshold.
/// - Returns: `(command_index, point)` if a command is within the hit radius.
fn find_hit_command(
    ops: &Ops,
    x: f64,
    y: f64,
    width: f64,
) -> Option<(usize, Point)> {
    let geo = ops.to_geometry();
    let closest = crate::geo::query::find_closest_point_on_path_from_array(
        &geo.data, x, y,
    );
    let (segment_index, _linear_t, point_on_path) = closest?;

    let dist_sq = (x - point_on_path.x) * (x - point_on_path.x)
        + (y - point_on_path.y) * (y - point_on_path.y);
    if dist_sq > (width * 2.0) * (width * 2.0) {
        return None;
    }

    let mut geo_idx = 0;
    for (cmd_idx, node) in ops.commands.iter().enumerate() {
        if node.is_moving() {
            if geo_idx == segment_index {
                return Some((cmd_idx, point_on_path));
            }
            geo_idx += 1;
        }
    }
    None
}

/// Find the start and end indices of the subpath containing `command_index`.
///
/// The start is the preceding `MoveTo` command; the end is the next `MoveTo`
/// (or the end of ops).
///
/// - `ops`: The ops sequence.
/// - `command_index`: Index of a command within the subpath.
/// - Returns: `(start_idx, end_idx)`.
fn find_subpath_bounds(ops: &Ops, command_index: usize) -> (usize, usize) {
    let mut start_idx = 0;
    for i in (0..=command_index).rev() {
        if let OpCategory::Moving {
            cmd: MoveCmd::MoveTo,
            ..
        } = &ops.commands[i].category
        {
            start_idx = i;
            break;
        }
    }

    let mut end_idx = ops.len();
    for i in (start_idx + 1)..ops.len() {
        if let OpCategory::Moving {
            cmd: MoveCmd::MoveTo,
            ..
        } = &ops.commands[i].category
        {
            end_idx = i;
            break;
        }
    }

    (start_idx, end_idx)
}

/// Accumulate the path distance from the start of a subpath to the hit point.
///
/// - `temp_ops`: Linearized subpath ops.
/// - `linear_geo_cmds`: Indices of geometry commands (MoveTo/LineTo).
/// - `linear_segment_idx`: Index of the segment containing the hit.
/// - `linear_t`: Parameter along the hit segment [0, 1].
/// - Returns: The cumulative distance to the hit point.
fn accumulate_distance_to_hit(
    temp_ops: &Ops,
    linear_geo_cmds: &[usize],
    linear_segment_idx: usize,
    linear_t: f64,
) -> f64 {
    let mut hit_dist = 0.0;
    let mut last_pos = temp_ops.commands[linear_geo_cmds[0]].end_point();

    for &cmd_idx in linear_geo_cmds.iter().take(linear_segment_idx).skip(1) {
        let end_pt = temp_ops.commands[cmd_idx].end_point();
        let dp = (end_pt.x - last_pos.x, end_pt.y - last_pos.y);
        hit_dist += (dp.0 * dp.0 + dp.1 * dp.1).sqrt();
        last_pos = end_pt;
    }

    let hit_segment_j = linear_geo_cmds[linear_segment_idx];
    let hit_end = temp_ops.commands[hit_segment_j].end_point();
    let dp = (hit_end.x - last_pos.x, hit_end.y - last_pos.y);
    let dist = (dp.0 * dp.0 + dp.1 * dp.1).sqrt();
    hit_dist += linear_t * dist;

    hit_dist
}

/// Build a new subpath by removing the given gap ranges.
///
/// - `temp_ops`: Linearized subpath ops.
/// - `gaps`: Slice of `(start, end)` distance ranges to remove.
/// - Returns: A new `Ops` with the gaps removed.
fn build_clipped_subpath(temp_ops: &Ops, gaps: &[(f64, f64)]) -> Ops {
    let mut new_subpath = Ops::new();
    new_subpath.commands.push(temp_ops.commands[0].clone());

    let mut accum_dist = 0.0;
    let mut last_pos = temp_ops.commands[0].end_point();

    for node in temp_ops.commands.iter().skip(1) {
        if let OpCategory::Moving {
            end: p2,
            cmd: MoveCmd::LineTo,
        } = &node.category
        {
            let p1 = last_pos;
            let seg_len = {
                let dp = (p2.x - p1.x, p2.y - p1.y);
                (dp.0 * dp.0 + dp.1 * dp.1).sqrt()
            };

            if seg_len < EPSILON_COLLINEAR {
                last_pos = *p2;
                continue;
            }

            let seg_start_dist = accum_dist;
            let seg_end_dist = accum_dist + seg_len;

            // Start with the full segment as kept, then carve out each gap.
            let mut kept: Vec<(f64, f64)> =
                vec![(seg_start_dist, seg_end_dist)];
            for (gap_start, gap_end) in gaps {
                let mut next_kept: Vec<(f64, f64)> = Vec::new();
                for (ks, ke) in &kept {
                    if *ks < *gap_end && *ke > *gap_start {
                        // overlap → split into up to two kept pieces
                        if *ks < *gap_start {
                            next_kept.push((*ks, (*ke).min(*gap_start)));
                        }
                        if *ke > *gap_end {
                            next_kept.push(((*ks).max(*gap_end), *ke));
                        }
                    } else {
                        next_kept.push((*ks, *ke));
                    }
                }
                kept = next_kept;
            }

            if kept.is_empty() {
                last_pos = *p2;
                accum_dist += seg_len;
                continue;
            }

            let vec_dx = p2.x - p1.x;
            let vec_dy = p2.y - p1.y;
            let dz = p2.z - p1.z;

            for (start_d, end_d) in kept {
                let t_start = if seg_len > 0.0 {
                    (start_d - seg_start_dist) / seg_len
                } else {
                    0.0
                };
                let t_end = if seg_len > 0.0 {
                    (end_d - seg_start_dist) / seg_len
                } else {
                    1.0
                };

                let start_pt = Point3D::new(
                    p1.x + t_start * vec_dx,
                    p1.y + t_start * vec_dy,
                    p1.z + t_start * dz,
                );
                let end_pt = Point3D::new(
                    p1.x + t_end * vec_dx,
                    p1.y + t_end * vec_dy,
                    p1.z + t_end * dz,
                );

                let mut last_kept_pos: Option<Point3D> = None;
                for lnode in new_subpath.commands.iter().rev() {
                    if lnode.is_moving() {
                        last_kept_pos = Some(lnode.end_point());
                        break;
                    }
                }

                if let Some(lkp) = last_kept_pos {
                    if needs_move_to(Some(lkp), start_pt) {
                        new_subpath
                            .move_to(start_pt.x, start_pt.y, start_pt.z, None);
                    }
                }

                new_subpath.line_to(end_pt.x, end_pt.y, end_pt.z, None);
            }

            last_pos = *p2;
            accum_dist += seg_len;
        } else {
            let in_any_gap = gaps
                .iter()
                .any(|(gs, ge)| *gs <= accum_dist && accum_dist <= *ge);
            if !in_any_gap {
                new_subpath.commands.push(node.clone());
            }
        }
    }

    new_subpath
}

/// Linearize once and apply all clip gaps in a single pass.
///
/// Unlike calling `clip_at` in a loop (whose successive calls
/// interfere via travel MoveTos), this linearizes the subpath
/// once, computes all gap regions at once, and emits a single
/// gapped path via `build_clipped_subpath`.
pub fn clip_subpath_linear(
    sub_ops: &Ops,
    clips: &[super::assembly::tabs::ClipPoint],
) -> Ops {
    let mut temp = sub_ops.copy();
    temp.linearize_all();

    let geo = temp.to_geometry();
    // Compute total length by summing the same way build_clipped_subpath
    // will traverse — avoids floating-point drift between geo.distance()
    // and the iterated segment lengths in build_clipped_subpath.
    let total_len: f64 = (1..temp.len())
        .filter(|&j| temp.command_type(j) == CommandType::LineTo)
        .scan(temp.commands[0].end_point(), |last, j| {
            let ep = temp.endpoint(j);
            let d = ((ep.x - last.x).powi(2) + (ep.y - last.y).powi(2)).sqrt();
            *last = ep;
            Some(d)
        })
        .sum();
    let is_closed = geo.is_closed(1e-6);

    let mut gaps: Vec<(f64, f64)> = Vec::new();
    for clip in clips {
        let closest = crate::geo::query::find_closest_point_on_path_from_array(
            &geo.data, clip.x, clip.y,
        );
        let (seg_idx, t, pt) = match closest {
            Some(v) => v,
            None => continue,
        };
        let dx = clip.x - pt.x;
        let dy = clip.y - pt.y;
        if dx * dx + dy * dy > (clip.width * 2.0).powi(2) {
            continue;
        }

        // distance along the linearised path to the hit point.
        // seg_idx from find_closest_point_on_path_from_array is the
        // index in the geometry data (0 = Move, 1 = first LineTo, …).
        // We skip the Move (index 0) and count LineTo commands.
        let mut hd = 0.0;
        let mut last = temp.commands[0].end_point();
        let mut line_idx = 0usize;
        for j in 1..temp.len() {
            if temp.command_type(j) != CommandType::LineTo {
                continue;
            }
            let ep = temp.endpoint(j);
            let seg_len =
                ((ep.x - last.x).powi(2) + (ep.y - last.y).powi(2)).sqrt();
            if line_idx + 1 == seg_idx {
                // this is the segment that was hit
                hd += t * seg_len;
                last = ep;
                break;
            }
            hd += seg_len;
            last = ep;
            line_idx += 1;
        }

        let half = clip.width / 2.0;
        let gs = hd - half;
        let ge = hd + half;

        if is_closed {
            if gs < 0.0 {
                gaps.push((total_len + gs, total_len));
            }
            if ge > total_len {
                gaps.push((0.0, ge - total_len));
            }
        }
        gaps.push((gs.max(0.0).min(total_len), ge.max(0.0).min(total_len)));
    }

    if gaps.is_empty() {
        return sub_ops.copy();
    }
    let mut result = build_clipped_subpath(&temp, &gaps);

    // When the gap wraps around the seam of a closed path the
    // endpoint falls inside a gap — adding a travel MoveTo back
    // to it would undo the wrapping. Check whether the subpath's
    // original endpoint lies within any gap.
    let orig_end = sub_ops.endpoint(sub_ops.len() - 1);
    let last_cut = result
        .commands
        .iter()
        .rev()
        .find(|n| n.is_moving())
        .map(|n| n.end_point());
    let needs_travel = match last_cut {
        Some(ep) => {
            let dx = orig_end.x - ep.x;
            let dy = orig_end.y - ep.y;
            (dx * dx + dy * dy).sqrt() > EPSILON_GAP_CLOSE
        }
        None => false,
    };
    if needs_travel {
        let seam_gapped = gaps
            .iter()
            .any(|(gs, ge)| *gs <= total_len && *ge >= total_len);
        if !seam_gapped {
            result.move_to(orig_end.x, orig_end.y, orig_end.z, None);
        }
    }

    result
}

/// Clip an arc command against polygon regions and refit primitives to the kept chains.
///
/// - `new_ops`: Target ops sequence.
/// - `node`: The node containing the arc command.
/// - `last_point`: Start point of the arc.
/// - `pen_pos`: Current pen position.
/// - `valid_regions`: Polygons to clip against.
/// - `tolerance`: Fit tolerance for primitive refitting.
/// - Returns: Updated pen position.
fn clip_and_refit_arc(
    new_ops: &mut Ops,
    node: &OpNode,
    last_point: Point3D,
    pen_pos: Option<Point3D>,
    valid_regions: &[Polygon],
    tolerance: f64,
) -> Option<Point3D> {
    let arc_state = node.state.clone();
    let linearized = crate::ops::linearize::linearize_node(node, last_point);

    let mut kept_pairs: Vec<(Point3D, Point3D)> = Vec::new();
    let mut p_seg_start = last_point;
    for lnode in &linearized.commands {
        let p_seg_end = lnode.end_point();
        let segs = clip_line_segment_with_polygons(
            p_seg_start,
            p_seg_end,
            valid_regions,
        );
        kept_pairs.extend(segs);
        p_seg_start = p_seg_end;
    }

    let chains = build_chains(&kept_pairs);

    let mut pen_pos = pen_pos;

    for chain in &chains {
        let primitives = fit_points_with_primitives(chain, tolerance);
        if primitives.is_empty() {
            continue;
        }
        let needs_move = needs_move_to(pen_pos, chain[0]);
        if needs_move {
            new_ops.move_to(chain[0].x, chain[0].y, chain[0].z, None);
        }
        for prim_cmd in &primitives {
            let end = prim_cmd.end_point();
            match prim_cmd {
                Command::Line { .. } => {
                    new_ops.line_to(end.x, end.y, end.z, None);
                }
                Command::Arc {
                    center_offset,
                    normal,
                    ..
                } => {
                    let clockwise = normal.z < 0.0;
                    new_ops.arc_to(
                        end.x,
                        end.y,
                        center_offset.x,
                        center_offset.y,
                        clockwise,
                        end.z,
                        None,
                    );
                }
                Command::Bezier {
                    control1, control2, ..
                } => {
                    new_ops.bezier_to(*control1, *control2, end, None);
                }
                Command::Move { .. } => {
                    continue;
                }
            }
            if let Some(ref s) = arc_state {
                let last = new_ops.len() - 1;
                new_ops.commands[last].set_state(s.clone());
            }
        }
        pen_pos = Some(chain[chain.len() - 1]);
    }

    pen_pos
}

/// Clip a Bezier command against polygon regions and refit primitives to the kept chains.
///
/// - `new_ops`: Target ops sequence.
/// - `node`: The node containing the Bezier command.
/// - `last_point`: Start point of the Bezier curve.
/// - `pen_pos`: Current pen position.
/// - `valid_regions`: Polygons to clip against.
/// - `tolerance`: Fit tolerance for primitive refitting.
/// - Returns: Updated pen position.
fn clip_and_refit_bezier(
    new_ops: &mut Ops,
    node: &OpNode,
    last_point: Point3D,
    pen_pos: Option<Point3D>,
    valid_regions: &[Polygon],
    tolerance: f64,
) -> Option<Point3D> {
    let bezier_state = node.state.clone();
    let linearized = crate::ops::linearize::linearize_node(node, last_point);

    let mut kept_pairs: Vec<(Point3D, Point3D)> = Vec::new();
    let mut p_seg_start = last_point;
    for lnode in &linearized.commands {
        let p_seg_end = lnode.end_point();
        let segs = clip_line_segment_with_polygons(
            p_seg_start,
            p_seg_end,
            valid_regions,
        );
        kept_pairs.extend(segs);
        p_seg_start = p_seg_end;
    }

    let chains = build_chains(&kept_pairs);

    let mut pen_pos = pen_pos;

    for chain in &chains {
        let primitives = fit_points_with_primitives(chain, tolerance);
        if primitives.is_empty() {
            continue;
        }
        let needs_move = needs_move_to(pen_pos, chain[0]);
        if needs_move {
            new_ops.move_to(chain[0].x, chain[0].y, chain[0].z, None);
        }
        for prim_cmd in &primitives {
            let end = prim_cmd.end_point();
            match prim_cmd {
                Command::Line { .. } => {
                    new_ops.line_to(end.x, end.y, end.z, None);
                }
                Command::Arc {
                    center_offset,
                    normal,
                    ..
                } => {
                    let clockwise = normal.z < 0.0;
                    new_ops.arc_to(
                        end.x,
                        end.y,
                        center_offset.x,
                        center_offset.y,
                        clockwise,
                        end.z,
                        None,
                    );
                }
                Command::Bezier {
                    control1, control2, ..
                } => {
                    new_ops.bezier_to(*control1, *control2, end, None);
                }
                Command::Move { .. } => {
                    continue;
                }
            }
            if let Some(ref s) = bezier_state {
                let last = new_ops.len() - 1;
                new_ops.commands[last].set_state(s.clone());
            }
        }
        pen_pos = Some(chain[chain.len() - 1]);
    }

    pen_pos
}

/// Connect adjacent `(start, end)` pairs into continuous point chains.
///
/// Pairs whose endpoints are within `EPSILON_GAP_CLOSE` are merged into the same chain.
///
/// - `kept_pairs`: Sequence of clipped segment pairs.
/// - Returns: A vector of continuous point chains.
fn build_chains(kept_pairs: &[(Point3D, Point3D)]) -> Vec<Vec<Point3D>> {
    let mut chains: Vec<Vec<Point3D>> = Vec::new();
    for (p1, p2) in kept_pairs {
        if let Some(last_chain) = chains.last_mut() {
            let last = last_chain[last_chain.len() - 1];
            let dx = p1.x - last.x;
            let dy = p1.y - last.y;
            if (dx * dx + dy * dy).sqrt() <= EPSILON_GAP_CLOSE {
                last_chain.push(*p2);
                continue;
            }
        }
        chains.push(vec![*p1, *p2]);
    }
    chains
}
