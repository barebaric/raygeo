//! Geometry: Core geometric path structure.
//!
//! This module provides the main `Geometry` struct for building and manipulating
//! geometric paths. Paths are constructed using command-based operations like
//! `move_to`, `line_to`, `arc_to`, and `bezier_to`. Commands are appended
//! directly to the data array on construction and can be queried for various
//! properties.

use crate::geo::query::get_positions_at_distances_from_array;
use crate::geo::shape::arc::get_arc_bounds;
use crate::geo::shape::bezier::get_bezier_bounds;
use crate::types::{BezierControls, Command, Point, Point3D, Rect};

/// A geometric path consisting of move, line, arc, and bezier commands.
///
/// Commands are appended directly to the `data` array on construction.
#[derive(Clone, Debug)]
pub struct Geometry {
    /// Command data stored as typed Command enum variants.
    pub(crate) data: Vec<Command>,
    /// The position where the last MOVE command was issued.
    pub last_move_to: Point3D,
    /// Whether the geometry can be uniformly scaled without distortion (false if arcs present).
    pub uniform_scalable: bool,
}

impl PartialEq for Geometry {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
            && self.last_move_to == other.last_move_to
            && self.uniform_scalable == other.uniform_scalable
    }
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
}

impl Geometry {
    /// Creates a new empty Geometry.
    pub fn new() -> Self {
        Geometry {
            data: Vec::new(),
            last_move_to: (0.0, 0.0, 0.0),
            uniform_scalable: true,
        }
    }

    /// Moves the current position to the specified point.
    /// Starts a new subpath; subsequent commands will continue from this point.
    pub fn move_to(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.last_move_to = (x, y, z);
        self.data.push(Command::Move { end: (x, y, z) });
        self
    }

    /// Draws a straight line from the current position to the specified point.
    pub fn line_to(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.data.push(Command::Line { end: (x, y, z) });
        self
    }

    /// Closes the current subpath by drawing a line back to the starting point.
    /// The starting point is the position of the last `move_to` command.
    pub fn close_path(&mut self) -> &mut Self {
        self.line_to(
            self.last_move_to.0,
            self.last_move_to.1,
            self.last_move_to.2,
        );
        self
    }

    /// Draws an arc from the current position to the specified endpoint.
    ///
    /// The arc is defined by:
    /// - `(x, y, z)`: The endpoint coordinates
    /// - `(i, j)`: The offset from the start point to the arc center
    /// - `clockwise`: Whether to draw the arc in clockwise direction
    pub fn arc_to(
        &mut self,
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
    ) -> &mut Self {
        self.uniform_scalable = false;
        self.data.push(Command::Arc {
            end: (x, y, z),
            center_offset: (i, j),
            clockwise,
        });
        self
    }

    /// Draws a cubic Bezier curve from the current position to the endpoint.
    ///
    /// The curve is defined by three control points:
    /// - `c1`: First control point
    /// - `c2`: Second control point
    /// - `p1`: End point (the start point is the current position)
    pub fn bezier_to(&mut self, controls: BezierControls, z: f64) -> &mut Self {
        let (c1, c2, p1) = controls;
        self.data.push(Command::Bezier {
            end: (p1.0, p1.1, z),
            control1: c1,
            control2: c2,
        });
        self
    }

    /// Returns a shared reference to the command data.
    pub fn data(&self) -> &Vec<Command> {
        &self.data
    }

    /// Creates a new geometry from a list of 3D points connected by line segments.
    pub fn from_points(points: &[(f64, f64, f64)], close: bool) -> Self {
        let mut geo = Self::new();
        if points.is_empty() {
            return geo;
        }
        geo.move_to(points[0].0, points[0].1, points[0].2);
        for &(x, y, z) in points.iter().skip(1) {
            geo.line_to(x, y, z);
        }
        if close && points.len() > 1 {
            geo.close_path();
        }
        geo
    }

    /// Returns the total number of commands.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if there are no commands.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clears all data and resets the geometry.
    pub fn clear(&mut self) -> &mut Self {
        self.data.clear();
        self.uniform_scalable = true;
        self
    }

    /// Returns the axis-aligned bounding rectangle of the geometry.
    /// Returns (0, 0, 0, 0) if the geometry is empty.
    pub fn rect(&self) -> Rect {
        if self.data.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }
        crate::geo::query::get_bounding_rect_from_array(&self.data)
    }

    /// Returns the bounding box of a single segment at the given index.
    /// Returns None for Move commands or if the index is out of bounds.
    pub fn segment_bounds(&self, index: usize) -> Option<Rect> {
        let cmd = self.data.get(index)?;
        let end = cmd.end_point();

        let (sx, sy) = if index > 0 {
            let prev = self.data[index - 1].end_point();
            (prev.0, prev.1)
        } else {
            (0.0, 0.0)
        };

        match cmd {
            Command::Move { .. } => None,
            Command::Line { .. } => Some((
                sx.min(end.0),
                sy.min(end.1),
                sx.max(end.0),
                sy.max(end.1),
            )),
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => Some(get_arc_bounds(
                (sx, sy),
                (end.0, end.1),
                *center_offset,
                *clockwise,
            )),
            Command::Bezier {
                control1, control2, ..
            } => Some(get_bezier_bounds(
                (sx, sy),
                *control1,
                *control2,
                (end.0, end.1),
            )),
        }
    }

    /// Given a list of distances along the path, returns the corresponding
    /// (segment_index, t, point_2d) for each distance.
    ///
    /// Distances are clamped to [0, total_length].
    pub fn get_positions_at_distances(
        &self,
        distances: &[f64],
    ) -> Vec<(usize, f64, Point)> {
        get_positions_at_distances_from_array(&self.data, distances)
    }

    /// Returns indices of all segments whose bounding box intersects the
    /// given rectangle. Excludes Move commands.
    pub fn segments_in_frame(
        &self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> Vec<usize> {
        let fmin_x = x1.min(x2);
        let fmin_y = y1.min(y2);
        let fmax_x = x1.max(x2);
        let fmax_y = y1.max(y2);

        let mut result = Vec::new();
        for idx in 0..self.data.len() {
            if let Some(bbox) = self.segment_bounds(idx) {
                if bbox.2 >= fmin_x
                    && bbox.0 <= fmax_x
                    && bbox.3 >= fmin_y
                    && bbox.1 <= fmax_y
                {
                    result.push(idx);
                }
            }
        }
        result
    }

    /// Returns the total path length (sum of all segment lengths).
    /// Returns 0.0 if the geometry is empty.
    pub fn distance(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        crate::geo::query::get_total_distance_from_array(&self.data)
    }

    /// Returns the total area enclosed by the geometry (absolute value).
    /// Returns 0.0 if the geometry is empty.
    pub fn area(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        crate::geo::algo::analysis::get_area_from_array(&self.data)
    }

    /// Returns true if the geometry forms a closed path within the given tolerance.
    /// A closed path starts and ends at the same point (within tolerance).
    pub fn is_closed(&self, tolerance: f64) -> bool {
        if self.data.len() < 2 {
            return false;
        }
        if !matches!(&self.data[0], Command::Move { .. }) {
            return false;
        }
        let start = self.data[0].end_point();
        let end = self.data[self.data.len() - 1].end_point();
        let dist_sq = (start.0 - end.0).powi(2)
            + (start.1 - end.1).powi(2)
            + (start.2 - end.2).powi(2);
        dist_sq < tolerance * tolerance
    }

    /// Creates a deep copy of the geometry.
    pub fn copy(&self) -> Geometry {
        let mut new_geo = Geometry::new();
        new_geo.last_move_to = self.last_move_to;
        new_geo.uniform_scalable = self.uniform_scalable;
        new_geo.data = self.data.clone();
        new_geo
    }

    /// Applies an affine transformation matrix to the geometry in place.
    /// The matrix is a 4x4 transformation matrix.
    pub fn transform(&mut self, matrix: &[[f64; 4]; 4]) -> &mut Self {
        if self.data.is_empty() {
            return self;
        }
        self.data = crate::geo::math::apply_affine_transform_to_array(
            &self.data, matrix,
        );
        let last_move_vec = [
            self.last_move_to.0,
            self.last_move_to.1,
            self.last_move_to.2,
            1.0,
        ];
        self.last_move_to = (
            matrix[0][0] * last_move_vec[0]
                + matrix[0][1] * last_move_vec[1]
                + matrix[0][2] * last_move_vec[2]
                + matrix[0][3] * last_move_vec[3],
            matrix[1][0] * last_move_vec[0]
                + matrix[1][1] * last_move_vec[1]
                + matrix[1][2] * last_move_vec[2]
                + matrix[1][3] * last_move_vec[3],
            matrix[2][0] * last_move_vec[0]
                + matrix[2][1] * last_move_vec[1]
                + matrix[2][2] * last_move_vec[2]
                + matrix[2][3] * last_move_vec[3],
        );
        self
    }

    /// Extends this geometry by appending all commands from another geometry.
    pub fn extend(&mut self, other: &Geometry) -> &mut Self {
        if !other.data.is_empty() {
            self.data.extend(other.data.clone());
        }
        self.uniform_scalable = self.uniform_scalable && other.uniform_scalable;
        self
    }

    /// Returns the geometry decomposed into continuous segments.
    /// Each segment is a vector of points representing a continuous path
    /// between MOVE commands.
    pub fn segments(&self) -> Vec<Vec<Point3D>> {
        if self.data.is_empty() {
            return Vec::new();
        }

        let mut all_segments: Vec<Vec<Point3D>> = Vec::new();
        let mut current_segment: Vec<Point3D> = Vec::new();
        let mut last_point: Point3D = (0.0, 0.0, 0.0);

        for cmd in &self.data {
            let end_point = cmd.end_point();

            match cmd {
                Command::Move { .. } => {
                    if !current_segment.is_empty() {
                        all_segments.push(current_segment);
                        current_segment = Vec::new();
                    }
                    current_segment.push(end_point);
                }
                _ => {
                    if current_segment.is_empty() {
                        current_segment.push(last_point);
                    }
                    current_segment.push(end_point);
                }
            }
            last_point = end_point;
        }

        if !current_segment.is_empty() {
            all_segments.push(current_segment);
        }

        all_segments
    }

    /// Returns a reference to the command at the given index, if it exists.
    pub fn get_command_at(&self, index: usize) -> Option<&Command> {
        self.data.get(index)
    }

    /// Returns an iterator over all commands.
    pub fn iter_commands(&self) -> impl Iterator<Item = &Command> + '_ {
        self.data.iter()
    }
}
