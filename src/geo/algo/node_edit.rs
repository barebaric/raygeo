//! Node editing: moving, inserting and deleting the nodes of a path,
//! toggling segments between line and curve, and moving Bezier handles.
//!
//! A node is the end point of a command. In a closed contour the start
//! node (the Move) and the end node of the last command are one node, so
//! editing either edits both. Arcs that touch an edited node are turned
//! into Beziers first. Every operation returns a new geometry and the
//! index of the edited command in it, because indices can shift.

use crate::geo::algo::fitting::convert_arc_to_beziers_from_array;
use crate::geo::geometry::Geometry;
use crate::geo::shape::bezier::split_bezier;
use crate::geo::types::{Command, Point, Point3D};

const CLOSE_TOLERANCE: f64 = 1e-9;

/// The command index range of the contour holding `index`:
/// (index of its Move, index of its last command).
fn contour_range(data: &[Command], index: usize) -> (usize, usize) {
    let mut start = index;
    while start > 0 && !matches!(data[start], Command::Move { .. }) {
        start -= 1;
    }
    let mut end = index.max(start);
    while end + 1 < data.len() && !matches!(data[end + 1], Command::Move { .. })
    {
        end += 1;
    }
    (start, end)
}

fn is_closed(data: &[Command], start: usize, end: usize) -> bool {
    end > start
        && matches!(data[start], Command::Move { .. })
        && data[start].end_point().distance(data[end].end_point())
            <= CLOSE_TOLERANCE
}

fn start_of(data: &[Command], index: usize) -> Point3D {
    if index == 0 {
        Point3D::ZERO
    } else {
        data[index - 1].end_point()
    }
}

fn finish(data: Vec<Command>) -> Geometry {
    let mut geo = Geometry::new();
    geo.last_move_to = data
        .iter()
        .rev()
        .find(|c| matches!(c, Command::Move { .. }))
        .map(|c| c.end_point())
        .unwrap_or(Point3D::ZERO);
    geo.uniform_scalable =
        !data.iter().any(|c| matches!(c, Command::Arc { .. }));
    geo.data = data;
    geo
}

/// Replaces the arcs at `indices` by Beziers. Returns the new commands
/// and a function mapping an old command index to its new index (the
/// last command of a converted arc).
fn convert_arcs(
    data: &[Command],
    indices: &[usize],
) -> (Vec<Command>, Vec<(usize, usize)>) {
    let mut out = Vec::with_capacity(data.len());
    let mut growth = Vec::new();
    for (i, cmd) in data.iter().enumerate() {
        if let (
            true,
            Command::Arc {
                end,
                center_offset,
                normal,
            },
        ) = (indices.contains(&i), cmd)
        {
            let beziers = convert_arc_to_beziers_from_array(
                start_of(data, i),
                *end,
                Point::new(center_offset.x, center_offset.y),
                normal.z < 0.0,
            );
            growth.push((i, beziers.len().saturating_sub(1)));
            out.extend(beziers);
        } else {
            out.push(cmd.clone());
        }
    }
    (out, growth)
}

fn remap(index: usize, growth: &[(usize, usize)]) -> usize {
    index
        + growth
            .iter()
            .filter(|(i, _)| *i <= index)
            .map(|(_, g)| g)
            .sum::<usize>()
}

/// The commands that share the node at `index`: the command ending
/// there and the one starting there, for both ends of a closed contour.
fn node_commands(data: &[Command], index: usize) -> Vec<usize> {
    let (start, end) = contour_range(data, index);
    let mut indices = vec![index];
    if index < end {
        indices.push(index + 1);
    }
    if is_closed(data, start, end) {
        if index == start {
            indices.push(end);
        } else if index == end {
            indices.push(start);
            if start < end {
                indices.push(start + 1);
            }
        }
    }
    indices
}

fn without_arcs_at(
    data: &[Command],
    index: usize,
    touching: &[usize],
) -> (Vec<Command>, usize) {
    let arcs: Vec<usize> = touching
        .iter()
        .copied()
        .filter(|i| matches!(data[*i], Command::Arc { .. }))
        .collect();
    if arcs.is_empty() {
        return (data.to_vec(), index);
    }
    let (converted, growth) = convert_arcs(data, &arcs);
    (converted, remap(index, &growth))
}

fn shift_end(cmd: &mut Command, to: Point3D) {
    let delta = to - cmd.end_point();
    match cmd {
        Command::Move { end } | Command::Line { end } => *end = to,
        Command::Arc { end, .. } => *end = to,
        Command::Bezier { end, control2, .. } => {
            *end = to;
            *control2 += delta;
        }
    }
}

fn shift_start(cmd: &mut Command, delta: Point3D) {
    if let Command::Bezier { control1, .. } = cmd {
        *control1 += delta;
    }
}

/// Moves the node at `index` to (x, y), dragging its Bezier handles
/// along. Returns the new geometry and the node's index in it.
pub fn move_node(
    geo: &Geometry,
    index: usize,
    x: f64,
    y: f64,
) -> (Geometry, usize) {
    if index >= geo.data.len() {
        return (geo.clone(), index);
    }
    let touching = node_commands(&geo.data, index);
    let (mut data, index) = without_arcs_at(&geo.data, index, &touching);
    let (start, end) = contour_range(&data, index);
    let closed = is_closed(&data, start, end);
    let old = data[index].end_point();
    let to = Point3D::new(x, y, old.z);
    let delta = to - old;
    shift_end(&mut data[index], to);
    if index < end {
        shift_start(&mut data[index + 1], delta);
    }
    if closed && index == start {
        shift_end(&mut data[end], to);
    } else if closed && index == end {
        shift_end(&mut data[start], to);
        if start < end {
            shift_start(&mut data[start + 1], delta);
        }
    }
    (finish(data), index)
}

/// The command that runs from the start of `a` to the end of `b`,
/// keeping the outer handles of both.
fn merged(data: &[Command], a: usize, b: usize) -> Command {
    let start = start_of(data, a);
    let end = data[b].end_point();
    let control1 = match &data[a] {
        Command::Bezier { control1, .. } => Some(*control1),
        _ => None,
    };
    let control2 = match &data[b] {
        Command::Bezier { control2, .. } => Some(*control2),
        _ => None,
    };
    if control1.is_none() && control2.is_none() {
        return Command::Line { end };
    }
    Command::Bezier {
        end,
        control1: control1.unwrap_or(start),
        control2: control2.unwrap_or(end),
    }
}

/// Deletes the node at `index`, joining the segments on both sides into
/// one. The open end of a contour is simply dropped. A contour left with
/// no segment is removed. Returns the new geometry and the index of the
/// joined command, or of the command now holding the removed position.
pub fn delete_node(geo: &Geometry, index: usize) -> (Geometry, usize) {
    if index >= geo.data.len() {
        return (geo.clone(), index);
    }
    let touching = node_commands(&geo.data, index);
    let (data, index) = without_arcs_at(&geo.data, index, &touching);
    let (start, end) = contour_range(&data, index);
    let closed = is_closed(&data, start, end);
    let segments = end - start;
    let mut out: Vec<Command> = data[..start].to_vec();
    let at_seam = index == start || index == end;
    let mut result_index = start;
    if segments <= 1 || (closed && segments <= 2) {
        // Nothing meaningful is left of this contour.
    } else if closed && at_seam {
        let joined = merged(&data, end, start + 1);
        let new_start = data[start + 1].end_point();
        out.push(Command::Move { end: new_start });
        out.extend(data[start + 2..end].iter().cloned());
        let mut last = joined;
        if let Command::Bezier { end: e, .. } = &mut last {
            *e = new_start;
        } else {
            last = Command::Line { end: new_start };
        }
        result_index = out.len();
        out.push(last);
    } else if index == start {
        out.push(Command::Move {
            end: data[start + 1].end_point(),
        });
        out.extend(data[start + 2..=end].iter().cloned());
    } else if index == end {
        out.extend(data[start..end].iter().cloned());
        result_index = end - 1;
    } else {
        out.extend(data[start..index].iter().cloned());
        result_index = out.len();
        out.push(merged(&data, index, index + 1));
        out.extend(data[index + 2..=end].iter().cloned());
    }
    out.extend(data[end + 1..].iter().cloned());
    let last = out.len().saturating_sub(1);
    (finish(out), result_index.min(last))
}

/// Splits the segment at `index` at parameter `t` (0..1), adding a node.
/// Returns the new geometry and the index of the new node.
pub fn insert_node(geo: &Geometry, index: usize, t: f64) -> (Geometry, usize) {
    if index >= geo.data.len()
        || matches!(geo.data[index], Command::Move { .. })
    {
        return (geo.clone(), index);
    }
    let (mut data, index) = without_arcs_at(&geo.data, index, &[index]);
    let t = t.clamp(0.0, 1.0);
    let start = start_of(&data, index);
    let replacement = match &data[index] {
        Command::Line { end } => {
            let mid = start.lerp(*end, t);
            vec![Command::Line { end: mid }, Command::Line { end: *end }]
        }
        Command::Bezier {
            end,
            control1,
            control2,
        } => {
            let (left, right) = split_bezier(
                start.truncate(),
                control1.truncate(),
                control2.truncate(),
                end.truncate(),
                t,
            );
            let z = start.z + (end.z - start.z) * t;
            vec![
                Command::Bezier {
                    end: left.3.extend(z),
                    control1: left.1.extend(z),
                    control2: left.2.extend(z),
                },
                Command::Bezier {
                    end: *end,
                    control1: right.1.extend(end.z),
                    control2: right.2.extend(end.z),
                },
            ]
        }
        _ => return (finish(data), index),
    };
    data.splice(index..=index, replacement);
    (finish(data), index)
}

/// Turns the segment at `index` into a cubic Bezier (handles at a third
/// and two thirds of the chord) when `curve` is true, or into a straight
/// line when it is false. Returns the new geometry and the segment's
/// index.
pub fn set_segment_curve(
    geo: &Geometry,
    index: usize,
    curve: bool,
) -> (Geometry, usize) {
    if index >= geo.data.len()
        || matches!(geo.data[index], Command::Move { .. })
    {
        return (geo.clone(), index);
    }
    if !curve {
        let mut data = geo.data.clone();
        let end = data[index].end_point();
        data[index] = Command::Line { end };
        return (finish(data), index);
    }
    let (mut data, index) = without_arcs_at(&geo.data, index, &[index]);
    if let Command::Line { end } = data[index] {
        let start = start_of(&data, index);
        data[index] = Command::Bezier {
            end,
            control1: start.lerp(end, 1.0 / 3.0),
            control2: start.lerp(end, 2.0 / 3.0),
        };
    }
    (finish(data), index)
}

/// Moves handle 1 or 2 of the Bezier at `index` to (x, y). Other
/// commands are left alone.
pub fn move_handle(
    geo: &Geometry,
    index: usize,
    handle: u8,
    x: f64,
    y: f64,
) -> Geometry {
    let mut data = geo.data.clone();
    if let Some(Command::Bezier {
        control1, control2, ..
    }) = data.get_mut(index)
    {
        let target = if handle == 1 { control1 } else { control2 };
        *target = Point3D::new(x, y, target.z);
    }
    finish(data)
}
