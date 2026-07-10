use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyType};
use pyo3_stub_gen::derive::{
    gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods,
};
use pyo3_stub_gen::inventory::submit;
use pyo3_stub_gen::{PyStubType, TypeInfo};

use glam::{DMat4, DVec4};

use crate::python::geo::matrix::Matrix as PyMatrix;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::geo::algo::analysis::{
    get_outward_normal_at_from_array, get_point_at_from_array,
    get_tangent_at_from_array,
};
use crate::geo::algo::cleanup::close_geometry_gaps_from_array;
use crate::geo::algo::fitting::{
    convert_arc_to_beziers_from_array, convert_arcs_to_beziers, fit_curves,
    linearize_data,
};
use crate::geo::algo::intersect::{
    check_intersection_from_array, check_self_intersection_from_array,
};
use crate::geo::algo::offset::grow_geometry;
use crate::geo::algo::simplify::simplify_data;
use crate::geo::algo::topology::{
    close_all_contours, filter_to_external_contours, get_valid_contours_data,
    normalize_winding_orders, remove_inner_edges, reverse_contour,
    split_inner_and_outer_contours, split_into_components, split_into_contours,
};
use crate::geo::geometry::Geometry as CoreGeometry;
use crate::geo::math::map_geometry_to_frame;
use crate::geo::query::find_closest_point_on_path_from_array;
use crate::python::geo::flex_point::{
    point3d_to_tuple, points_to_tuples, polygons_to_tuples,
};
use crate::types::{Command as CoreCommand, Point, Point3D};

/// A rapid-move command with an endpoint but no cutting.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo", name = "Move", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyMove {
    /// Endpoint of the move in 3D space.
    #[pyo3(get)]
    pub end: (f64, f64, f64),
}

/// A straight-line cutting command.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo", name = "Line", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyLine {
    /// Endpoint of the line in 3D space.
    #[pyo3(get)]
    pub end: (f64, f64, f64),
}

/// A circular-arc cutting command.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo", name = "Arc", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyArc {
    /// Endpoint of the arc in 3D space.
    #[pyo3(get)]
    pub end: (f64, f64, f64),
    /// Centre offset from the start point (3D).
    #[pyo3(get)]
    pub center_offset: (f64, f64, f64),
    /// Plane normal of the arc. A positive Z component means CCW in XY.
    #[pyo3(get)]
    pub normal: (f64, f64, f64),
}

#[gen_stub_pymethods]
#[pymethods]
impl PyArc {
    /// Whether the arc is clockwise (computed from the normal).
    #[getter]
    fn clockwise(&self) -> bool {
        self.normal.2 < 0.0
    }
}

/// A cubic-Bezier curve cutting command.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo", name = "Bezier", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyBezier {
    /// Endpoint of the curve in 3D space.
    #[pyo3(get)]
    pub end: (f64, f64, f64),
    /// First control point in 3D space.
    #[pyo3(get)]
    pub control1: (f64, f64, f64),
    /// Second control point in 3D space.
    #[pyo3(get)]
    pub control2: (f64, f64, f64),
}

enum PyTypedCommand {
    Move(PyMove),
    Line(PyLine),
    Arc(PyArc),
    Bezier(PyBezier),
}

impl From<CoreCommand> for PyTypedCommand {
    fn from(cmd: CoreCommand) -> Self {
        match cmd {
            CoreCommand::Move { end } => PyTypedCommand::Move(PyMove {
                end: point3d_to_tuple(end),
            }),
            CoreCommand::Line { end } => PyTypedCommand::Line(PyLine {
                end: point3d_to_tuple(end),
            }),
            CoreCommand::Arc {
                end,
                center_offset,
                normal,
            } => PyTypedCommand::Arc(PyArc {
                end: point3d_to_tuple(end),
                center_offset: point3d_to_tuple(center_offset),
                normal: point3d_to_tuple(normal),
            }),
            CoreCommand::Bezier {
                end,
                control1,
                control2,
            } => PyTypedCommand::Bezier(PyBezier {
                end: point3d_to_tuple(end),
                control1: point3d_to_tuple(control1),
                control2: point3d_to_tuple(control2),
            }),
        }
    }
}

impl PyTypedCommand {
    fn into_py_obj(self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(match self {
            PyTypedCommand::Move(c) => Py::new(py, c)?.into_any(),
            PyTypedCommand::Line(c) => Py::new(py, c)?.into_any(),
            PyTypedCommand::Arc(c) => Py::new(py, c)?.into_any(),
            PyTypedCommand::Bezier(c) => Py::new(py, c)?.into_any(),
        })
    }
}

impl PyStubType for &mut Geometry {
    fn type_input() -> TypeInfo {
        Geometry::type_input()
    }
    fn type_output() -> TypeInfo {
        Geometry::type_output()
    }
}

#[derive(Clone)]
struct FlexPoint {
    x: f64,
    y: f64,
    z: f64,
}

impl<'a, 'py> FromPyObject<'a, 'py> for FlexPoint {
    type Error = PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(p3) = ob.extract::<(f64, f64, f64)>() {
            Ok(FlexPoint {
                x: p3.0,
                y: p3.1,
                z: p3.2,
            })
        } else if let Ok(p2) = ob.extract::<(f64, f64)>() {
            Ok(FlexPoint {
                x: p2.0,
                y: p2.1,
                z: 0.0,
            })
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "expected a 2-tuple or 3-tuple of floats",
            ))
        }
    }
}

/// A sequence of geometric commands (Move, Line, Arc, Bezier).
///
/// The primary building block for vector geometry in raygeo.
/// Geometry objects can be constructed procedurally, parsed from SVG,
/// or obtained by converting an :class:`~raygeo.ops.Ops` sequence.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo", skip_from_py_object)]
#[derive(Clone)]
pub struct Geometry {
    pub(crate) inner: CoreGeometry,
}

submit! {
    gen_methods_from_python! {
        r#"
        from raygeo.geo import types

        class Geometry:
            def __eq__(self, other: object) -> bool:
                """Check equality with another Geometry.

                :complexity: O(n) time, O(1) space
                """
                ...
            def __ne__(self, other: object) -> bool:
                """Check inequality with another Geometry.

                :complexity: O(n) time, O(1) space
                """
                ...
            def transform(self, matrix: Matrix | types.TransformMatrix) -> Geometry:
                """Apply an affine transformation matrix.

                :param matrix: A :class:`~raygeo.geo.Matrix` or a 4x4 matrix
                    as list of lists.
                :returns: A new transformed Geometry.
                :complexity: O(n) time, O(n) space
                """
                ...
            def iter_typed_commands(self) -> list[Move | Line | Arc | Bezier]:
                """Iterate over all commands as typed command objects.

                :complexity: O(n) time, O(n) space
                """
                ...
            def get_typed_command_at(self, index: int) -> Move | Line | Arc | Bezier | None:
                """Get the typed command at the given index.

                :param index: Command index.
                :complexity: O(1) time, O(1) space
                """
                ...
        "#
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Geometry {
    /// Create a new empty Geometry.
    #[new]
    fn new() -> Self {
        Geometry {
            inner: CoreGeometry::new(),
        }
    }

    #[gen_stub(skip)]
    fn __reduce_ex__<'py>(
        slf: &Bound<'py, Self>,
        protocol: i32,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, pyo3::types::PyTuple>> {
        let _ = protocol;
        let borrowed = slf.borrow();
        let data = borrowed.to_dict(py)?;
        let from_dict = slf.get_type().getattr("from_dict")?;
        pyo3::types::PyTuple::new(
            py,
            [
                from_dict.as_any(),
                pyo3::types::PyTuple::new(py, [data.as_any()])?.as_any(),
            ],
        )
    }

    /// Move the pen to the given coordinates.
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :param z: Z coordinate (default 0.0).
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(1) time, O(1) space
    #[pyo3(signature = (x, y, z=0.0))]
    fn move_to(
        slf: Bound<'_, Self>,
        x: f64,
        y: f64,
        z: f64,
    ) -> Bound<'_, Self> {
        slf.borrow_mut().inner.move_to(x, y, z);
        slf
    }

    /// Draw a line to the given coordinates.
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :param z: Z coordinate (default 0.0).
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(1) time, O(1) space
    #[pyo3(signature = (x, y, z=0.0))]
    fn line_to(
        slf: Bound<'_, Self>,
        x: f64,
        y: f64,
        z: f64,
    ) -> Bound<'_, Self> {
        slf.borrow_mut().inner.line_to(x, y, z);
        slf
    }

    /// Close the current sub-path.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(1) time, O(1) space
    fn close_path(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf.borrow_mut().inner.close_path();
        slf
    }

    /// Draw an arc to the given coordinates.
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :param i: I offset from current point to center.
    /// :param j: J offset from current point to center.
    /// :param clockwise: Whether the arc is clockwise.
    /// :param z: Z coordinate (default 0.0).
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(1) time, O(1) space
    #[pyo3(signature = (x, y, i=0.0, j=0.0, clockwise=true, z=0.0))]
    fn arc_to(
        slf: Bound<'_, Self>,
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
    ) -> Bound<'_, Self> {
        slf.borrow_mut().inner.arc_to(x, y, i, j, clockwise, z);
        slf
    }

    /// Draw a cubic bezier curve.
    ///
    /// :param x: End X coordinate.
    /// :param y: End Y coordinate.
    /// :param c1x: First control point X.
    /// :param c1y: First control point Y.
    /// :param c1z: First control point Z (default 0.0).
    /// :param c2x: Second control point X.
    /// :param c2y: Second control point Y.
    /// :param c2z: Second control point Z (default 0.0).
    /// :param z: End Z coordinate (default 0.0).
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(1) time, O(1) space
    #[pyo3(signature = (x, y, c1x, c1y, c2x, c2y, *, c1z=0.0, c2z=0.0, z=0.0))]
    #[allow(clippy::too_many_arguments)]
    fn bezier_to(
        slf: Bound<'_, Self>,
        x: f64,
        y: f64,
        c1x: f64,
        c1y: f64,
        c2x: f64,
        c2y: f64,
        c1z: f64,
        c2z: f64,
        z: f64,
    ) -> Bound<'_, Self> {
        slf.borrow_mut().inner.bezier_to(
            Point3D::new(c1x, c1y, c1z),
            Point3D::new(c2x, c2y, c2z),
            Point3D::new(x, y, z),
        );
        slf
    }

    /// Return the number of commands.
    ///
    /// :complexity: O(1) time, O(1) space
    fn __len__(&mut self) -> usize {
        self.inner.len()
    }

    /// Return a hash of the geometry data.
    ///
    /// :complexity: O(n) time, O(1) space
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for cmd in self.inner.data() {
            std::mem::discriminant(cmd).hash(&mut hasher);
            let (ex, ey, ez) = match cmd {
                CoreCommand::Move { end } | CoreCommand::Line { end } => {
                    (end.x, end.y, end.z)
                }
                CoreCommand::Arc {
                    end,
                    center_offset,
                    normal,
                } => {
                    center_offset.x.to_bits().hash(&mut hasher);
                    center_offset.y.to_bits().hash(&mut hasher);
                    center_offset.z.to_bits().hash(&mut hasher);
                    normal.x.to_bits().hash(&mut hasher);
                    normal.y.to_bits().hash(&mut hasher);
                    normal.z.to_bits().hash(&mut hasher);
                    (end.x, end.y, end.z)
                }
                CoreCommand::Bezier {
                    end,
                    control1,
                    control2,
                } => {
                    control1.x.to_bits().hash(&mut hasher);
                    control1.y.to_bits().hash(&mut hasher);
                    control1.z.to_bits().hash(&mut hasher);
                    control2.x.to_bits().hash(&mut hasher);
                    control2.y.to_bits().hash(&mut hasher);
                    control2.z.to_bits().hash(&mut hasher);
                    (end.x, end.y, end.z)
                }
            };
            ex.to_bits().hash(&mut hasher);
            ey.to_bits().hash(&mut hasher);
            ez.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Check if the geometry has no commands.
    ///
    /// :returns: ``True`` if the geometry has no commands.
    /// :complexity: O(1) time, O(1) space
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Remove all commands from the geometry.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(1) time, O(1) space
    fn clear(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf.borrow_mut().inner.clear();
        slf
    }

    /// The coordinates of the last move-to command.
    ///
    /// :complexity: O(1) time, O(1) space
    #[getter]
    fn last_move_to(&self) -> (f64, f64, f64) {
        point3d_to_tuple(self.inner.last_move_to)
    }

    /// Set the last move-to position.
    ///
    /// :complexity: O(1) time, O(1) space
    #[setter]
    fn set_last_move_to(&mut self, value: (f64, f64, f64)) {
        self.inner.last_move_to = Point3D::new(value.0, value.1, value.2);
    }

    /// Whether the geometry uses uniform scalable arcs.
    ///
    /// :complexity: O(1) time, O(1) space
    #[getter]
    fn uniform_scalable(&self) -> bool {
        self.inner.uniform_scalable
    }

    /// Set whether the geometry uses uniform scalable arcs.
    ///
    /// :complexity: O(1) time, O(1) space
    #[setter(uniform_scalable)]
    fn set_uniform_scalable(&mut self, value: bool) {
        self.inner.uniform_scalable = value;
    }

    /// Return a deep copy of this geometry.
    ///
    /// :returns: A deep copy of the geometry.
    /// :complexity: O(n) time, O(n) space
    fn copy(&self) -> Self {
        Geometry {
            inner: self.inner.copy(),
        }
    }

    /// Get the last point in the geometry.
    ///
    /// :returns: The last point as ``(x, y, z)``, or ``(0, 0, 0)`` if empty.
    /// :complexity: O(1) time, O(1) space
    fn get_last_point(&self) -> (f64, f64, f64) {
        if let Some(last) = self.inner.data().last() {
            return point3d_to_tuple(last.end_point());
        }
        (0.0, 0.0, 0.0)
    }

    /// Apply an affine transformation matrix.
    ///
    /// :param matrix: A :class:`~raygeo.geo.Matrix` or a 4x4 matrix as list
    ///     of lists.
    /// :complexity: O(n) time, O(1) space
    #[gen_stub(skip)]
    fn transform<'py>(
        slf: Bound<'py, Self>,
        matrix: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, Self>> {
        let mat = if let Ok(py_m) = matrix.extract::<PyMatrix>() {
            py_m.inner.to_4x4()
        } else if let Ok(rows) = matrix.extract::<Vec<Vec<f64>>>() {
            if rows.len() != 4 || rows.iter().any(|r| r.len() != 4) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "expected a 4x4 matrix",
                ));
            }
            DMat4::from_cols(
                DVec4::new(rows[0][0], rows[1][0], rows[2][0], rows[3][0]),
                DVec4::new(rows[0][1], rows[1][1], rows[2][1], rows[3][1]),
                DVec4::new(rows[0][2], rows[1][2], rows[2][2], rows[3][2]),
                DVec4::new(rows[0][3], rows[1][3], rows[2][3], rows[3][3]),
            )
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "expected a Matrix or a 4x4 list of lists",
            ));
        };
        slf.borrow_mut().inner.transform(&mat);
        Ok(slf)
    }

    /// Append another geometry's commands to this one.
    ///
    /// :param other: The geometry to append.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    fn extend<'a>(
        slf: Bound<'a, Self>,
        other: &'a Geometry,
    ) -> Bound<'a, Self> {
        slf.borrow_mut().inner.extend(&other.inner);
        slf
    }

    /// Return the bounding rectangle (x_min, y_min, x_max, y_max).
    ///
    /// :returns: (x_min, y_min, x_max, y_max).
    /// :complexity: O(n) time, O(1) space
    fn rect(&mut self) -> (f64, f64, f64, f64) {
        let r = self.inner.rect();
        (r.min.x, r.min.y, r.max.x, r.max.y)
    }

    /// Return the bounding box of a single segment at the given index.
    /// Returns None for Move commands or if the index is out of bounds.
    ///
    /// :param index: Segment index.
    /// :returns: (x_min, y_min, x_max, y_max) or None.
    /// :complexity: O(1) time, O(1) space
    fn segment_bounds(&mut self, index: usize) -> Option<(f64, f64, f64, f64)> {
        self.inner
            .segment_bounds(index)
            .map(|r| (r.min.x, r.min.y, r.max.x, r.max.y))
    }

    /// Given a list of distances along the path, returns the corresponding
    /// (segment_index, t, point) for each distance.
    ///
    /// Distances are clamped to [0, total_length].
    ///
    /// :param distances: List of distances along the path.
    /// :returns: List of (segment_index, t, (x, y)) tuples.
    /// :complexity: O(n + m) time, O(m) space where n is the number of segments and m the number of distances
    fn get_positions_at_distances(
        &mut self,
        distances: Vec<f64>,
    ) -> Vec<(usize, f64, (f64, f64))> {
        self.inner
            .get_positions_at_distances(&distances)
            .into_iter()
            .map(|(a, b, p)| (a, b, (p.x, p.y)))
            .collect()
    }

    /// Return indices of all segments whose bounding box intersects the
    /// given rectangle. Excludes Move commands.
    ///
    /// :param x1: First corner X.
    /// :param y1: First corner Y.
    /// :param x2: Second corner X.
    /// :param y2: Second corner Y.
    /// :returns: List of segment indices.
    /// :complexity: O(n) time, O(n) space
    fn segments_in_frame(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> Vec<usize> {
        self.inner.segments_in_frame(x1, y1, x2, y2)
    }

    /// Return the total path distance.
    ///
    /// :returns: The total path distance in mm.
    /// :complexity: O(n) time, O(1) space
    fn distance(&mut self) -> f64 {
        self.inner.distance()
    }

    /// Return the signed area of the geometry.
    ///
    /// :returns: The signed area in mm².
    /// :complexity: O(n) time, O(1) space
    fn area(&mut self) -> f64 {
        self.inner.area()
    }

    /// Check if the geometry forms a closed path.
    ///
    /// :param tolerance: Max gap between start and end point.
    /// :returns: ``True`` if the geometry forms a closed path.
    /// :complexity: O(n) time, O(1) space
    #[pyo3(signature = (tolerance=1e-6))]
    fn is_closed(&mut self, tolerance: f64) -> bool {
        self.inner.is_closed(tolerance)
    }

    /// Return the geometry split into segments of connected commands.
    ///
    /// :returns: List of segments, each a list of 3D points.
    /// :complexity: O(n) time, O(n) space
    fn segments(&mut self) -> Vec<Vec<(f64, f64, f64)>> {
        self.inner
            .segments()
            .into_iter()
            .map(|seg| seg.into_iter().map(point3d_to_tuple).collect())
            .collect()
    }

    /// The commands as a list of typed command objects.
    ///
    /// :complexity: O(n) time, O(n) space
    #[getter]
    fn data<'py>(&mut self, py: Python<'py>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .data
            .iter()
            .map(|cmd| PyTypedCommand::from(cmd.clone()).into_py_obj(py))
            .collect()
    }

    /// Get the command at the given index as a typed command object.
    ///
    /// :param index: Command index (negative returns None).
    /// :returns: The typed command or ``None``.
    /// :complexity: O(1) time, O(1) space
    fn get_command_at(
        &mut self,
        py: Python<'_>,
        index: isize,
    ) -> PyResult<Option<Py<PyAny>>> {
        if index < 0 {
            return Ok(None);
        }
        let data = self.inner.data();
        match data.get(index as usize) {
            Some(cmd) => {
                Ok(Some(PyTypedCommand::from(cmd.clone()).into_py_obj(py)?))
            }
            None => Ok(None),
        }
    }

    /// Iterate over all commands as typed command objects.
    ///
    /// :returns: List of typed command objects.
    /// :complexity: O(n) time, O(n) space
    fn iter_commands(&mut self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        let data = self.inner.data();
        data.iter()
            .map(|cmd| PyTypedCommand::from(cmd.clone()).into_py_obj(py))
            .collect()
    }

    /// Iterate over all commands as typed PyCommand objects.
    ///
    /// :returns: List of typed command objects.
    #[gen_stub(skip)]
    fn iter_typed_commands(
        &mut self,
        py: Python<'_>,
    ) -> PyResult<Vec<Py<PyAny>>> {
        let data = self.inner.data();
        data.iter()
            .map(|cmd| PyTypedCommand::from(cmd.clone()).into_py_obj(py))
            .collect()
    }

    /// Get the typed command at the given index.
    ///
    /// :param index: Command index.
    /// :returns: The typed command or ``None``.
    #[gen_stub(skip)]
    fn get_typed_command_at(
        &mut self,
        py: Python<'_>,
        index: isize,
    ) -> PyResult<Option<Py<PyAny>>> {
        if index < 0 {
            return Ok(None);
        }
        let data = self.inner.data();
        match data.get(index as usize) {
            Some(cmd) => {
                Ok(Some(PyTypedCommand::from(cmd.clone()).into_py_obj(py)?))
            }
            None => Ok(None),
        }
    }

    /// Serialize the geometry to a dictionary.
    ///
    /// :returns: A dictionary representation of the geometry.
    /// :complexity: O(n) time, O(n) space
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let last_move_to = self.inner.last_move_to;
        let uniform_scalable = self.inner.uniform_scalable;
        let dict = PyDict::new(py);
        dict.set_item(
            "last_move_to",
            vec![last_move_to.x, last_move_to.y, last_move_to.z],
        )?;
        dict.set_item("uniform_scalable", uniform_scalable)?;
        let commands = PyList::empty(py);
        for cmd in &self.inner.data {
            let entry = PyList::empty(py);
            match cmd {
                CoreCommand::Move { end } => {
                    entry.append("M")?;
                    entry.append(end.x)?;
                    entry.append(end.y)?;
                    entry.append(end.z)?;
                }
                CoreCommand::Line { end } => {
                    entry.append("L")?;
                    entry.append(end.x)?;
                    entry.append(end.y)?;
                    entry.append(end.z)?;
                }
                CoreCommand::Arc {
                    end,
                    center_offset,
                    normal,
                } => {
                    entry.append("A")?;
                    entry.append(end.x)?;
                    entry.append(end.y)?;
                    entry.append(end.z)?;
                    entry.append(center_offset.x)?;
                    entry.append(center_offset.y)?;
                    entry.append(center_offset.z)?;
                    entry.append(normal.x)?;
                    entry.append(normal.y)?;
                    entry.append(normal.z)?;
                }
                CoreCommand::Bezier {
                    end,
                    control1,
                    control2,
                } => {
                    entry.append("B")?;
                    entry.append(end.x)?;
                    entry.append(end.y)?;
                    entry.append(end.z)?;
                    entry.append(control1.x)?;
                    entry.append(control1.y)?;
                    entry.append(control1.z)?;
                    entry.append(control2.x)?;
                    entry.append(control2.y)?;
                    entry.append(control2.z)?;
                }
            }
            commands.append(entry)?;
        }
        dict.set_item("commands", commands)?;
        Ok(dict)
    }

    /// Create a Geometry from a dictionary.
    ///
    /// :param data: A dictionary as produced by
    ///     :meth:`to_dict`.
    /// :returns: A new ``Geometry`` instance.
    /// :complexity: O(n) time, O(n) space
    #[classmethod]
    fn from_dict<'py>(
        _cls: &Bound<'py, PyType>,
        data: &Bound<'py, PyDict>,
    ) -> PyResult<Self> {
        let mut geo = Self::new();
        if let Some(lmt) = data.get_item("last_move_to")? {
            if let Ok(lmt_list) = lmt.extract::<Vec<f64>>() {
                if lmt_list.len() >= 3 {
                    geo.inner.last_move_to =
                        Point3D::new(lmt_list[0], lmt_list[1], lmt_list[2]);
                }
            }
        }
        if let Some(us) = data.get_item("uniform_scalable")? {
            if let Ok(val) = us.extract::<bool>() {
                geo.inner.uniform_scalable = val;
            }
        }
        if let Some(cmds) = data.get_item("commands")? {
            if let Ok(cmds_list) = cmds.cast::<PyList>() {
                for item in cmds_list.iter() {
                    if let Ok(cmd_list) = item.cast::<PyList>() {
                        let cmd_type: String = match cmd_list.get_item(0) {
                            Ok(val) => match val.extract::<String>() {
                                Ok(s) => s,
                                Err(_) => continue,
                            },
                            Err(_) => continue,
                        };
                        let x: f64 = match cmd_list.get_item(1) {
                            Ok(val) => match val.extract::<f64>() {
                                Ok(v) => v,
                                Err(_) => continue,
                            },
                            Err(_) => continue,
                        };
                        let y: f64 = match cmd_list.get_item(2) {
                            Ok(val) => match val.extract::<f64>() {
                                Ok(v) => v,
                                Err(_) => continue,
                            },
                            Err(_) => continue,
                        };
                        let z: f64 = match cmd_list.get_item(3) {
                            Ok(val) => match val.extract::<f64>() {
                                Ok(v) => v,
                                Err(_) => continue,
                            },
                            Err(_) => continue,
                        };
                        match cmd_type.as_str() {
                            "M" => {
                                geo.inner.move_to(x, y, z);
                            }
                            "L" => {
                                geo.inner.line_to(x, y, z);
                            }
                            "A" => {
                                let i_val = cmd_list
                                    .get_item(4)
                                    .ok()
                                    .and_then(|v| v.extract::<f64>().ok());
                                let j_val = cmd_list
                                    .get_item(5)
                                    .ok()
                                    .and_then(|v| v.extract::<f64>().ok());
                                if let (Some(i_val), Some(j_val)) =
                                    (i_val, j_val)
                                {
                                    // 10-element format: [A, x, y, z, i, j, k, nx, ny, nz]
                                    let k_val = cmd_list
                                        .get_item(6)
                                        .ok()
                                        .and_then(|v| v.extract::<f64>().ok());
                                    if let (
                                        Some(k_val),
                                        Some(nx),
                                        Some(ny),
                                        Some(nz),
                                    ) = (
                                        k_val,
                                        cmd_list.get_item(7).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                        cmd_list.get_item(8).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                        cmd_list.get_item(9).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                    ) {
                                        geo.inner.arc_to_3d(
                                            x, y, z, i_val, j_val, k_val, nx,
                                            ny, nz,
                                        );
                                    } else {
                                        // Legacy 7-element format: [A, x, y, z, i, j, cw]
                                        let cw_val = cmd_list
                                            .get_item(6)
                                            .ok()
                                            .and_then(|v| {
                                                v.extract::<f64>().ok()
                                            })
                                            .unwrap_or(1.0);
                                        geo.inner.arc_to(
                                            x,
                                            y,
                                            i_val,
                                            j_val,
                                            cw_val > 0.5,
                                            z,
                                        );
                                    }
                                }
                            }
                            "B" if !Self::try_parse_bezier_current(
                                cmd_list, x, y, z, &mut geo,
                            ) =>
                            {
                                Self::try_parse_bezier_legacy(
                                    cmd_list, x, y, z, &mut geo,
                                );
                            }
                            _ => {}
                        }
                    } else if let Ok(cmd_dict) = item.cast::<PyDict>() {
                        let cmd_type: String = match cmd_dict.get_item("type") {
                            Ok(Some(val)) => match val.extract::<String>() {
                                Ok(s) => s,
                                Err(_) => continue,
                            },
                            _ => continue,
                        };
                        let end = match cmd_dict.get_item("end") {
                            Ok(Some(val)) => val.extract::<(f64, f64, f64)>(),
                            Err(_) => continue,
                            Ok(None) => continue,
                        };
                        let (x, y, z) = match end {
                            Ok(e) => e,
                            Err(_) => {
                                let end2 = match cmd_dict.get_item("end") {
                                    Ok(Some(val)) => {
                                        val.extract::<(f64, f64)>()
                                    }
                                    _ => continue,
                                };
                                match end2 {
                                    Ok((ex, ey)) => (ex, ey, 0.0),
                                    Err(_) => continue,
                                }
                            }
                        };
                        match cmd_type.as_str() {
                            "MoveToCommand" => {
                                geo.inner.move_to(x, y, z);
                            }
                            "LineToCommand" => {
                                geo.inner.line_to(x, y, z);
                            }
                            "CurveToCommand" | "BezierToCommand" => {
                                if let (Ok(Some(c1)), Ok(Some(c2))) = (
                                    cmd_dict.get_item("control1"),
                                    cmd_dict.get_item("control2"),
                                ) {
                                    let c1v = c1.extract::<(f64, f64, f64)>();
                                    let c2v = c2.extract::<(f64, f64, f64)>();
                                    if let (Ok(c1_3d), Ok(c2_3d)) = (c1v, c2v) {
                                        geo.inner.bezier_to(
                                            Point3D::new(
                                                c1_3d.0, c1_3d.1, c1_3d.2,
                                            ),
                                            Point3D::new(
                                                c2_3d.0, c2_3d.1, c2_3d.2,
                                            ),
                                            Point3D::new(x, y, z),
                                        );
                                    }
                                }
                            }
                            "ArcToCommand" => {
                                let i_val: Option<(f64, f64)> = match cmd_dict
                                    .get_item("center")
                                {
                                    Ok(Some(val)) => {
                                        val.extract::<(f64, f64)>().ok()
                                    }
                                    _ => None,
                                }
                                .or_else(|| {
                                    match cmd_dict.get_item("center_offset") {
                                        Ok(Some(val)) => {
                                            val.extract::<(f64, f64)>().ok()
                                        }
                                        _ => None,
                                    }
                                });
                                let cw_val: Option<bool> = match cmd_dict
                                    .get_item("clockwise")
                                {
                                    Ok(Some(val)) => val.extract::<bool>().ok(),
                                    _ => None,
                                };
                                if let (Some((ci, cj)), Some(cw)) =
                                    (i_val, cw_val)
                                {
                                    geo.inner.arc_to(x, y, ci, cj, cw, z);
                                }
                            }
                            "ClosePathCommand" => {
                                geo.inner.close_path();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(geo)
    }

    /// Create a Geometry from a sequence of points.
    ///
    /// :param points: A sequence of (x, y) or
    ///     (x, y, z) coordinate tuples.
    /// :param close: Whether to close the path.
    /// :returns: A new ``Geometry`` instance.
    /// :complexity: O(n) time, O(n) space
    #[classmethod]
    #[pyo3(signature = (points, close=true))]
    fn from_points<'py>(
        _cls: &Bound<'py, PyType>,
        points: &Bound<'py, PyAny>,
        close: bool,
    ) -> PyResult<Self> {
        let mut geo = Self::new();
        let points_vec: Vec<FlexPoint> = points
            .try_iter()?
            .map(|p| p?.extract::<FlexPoint>())
            .collect::<Result<Vec<_>, _>>()?;
        if points_vec.is_empty() {
            return Ok(geo);
        }
        let first = &points_vec[0];
        geo.inner.move_to(first.x, first.y, first.z);
        for p in &points_vec[1..] {
            geo.inner.line_to(p.x, p.y, p.z);
        }
        if close && points_vec.len() > 2 {
            geo.inner.close_path();
        }
        Ok(geo)
    }

    /// Return a new Geometry containing only commands at the given
    /// indices.
    ///
    /// :param indices: Set of command indices to keep.
    /// :returns: A new Geometry with the filtered commands.
    /// :complexity: O(n) time, O(n) space
    fn filter(&self, indices: std::collections::HashSet<usize>) -> Self {
        let mut core = CoreGeometry::new();
        core.data = self
            .inner
            .data
            .iter()
            .enumerate()
            .filter(|(i, _)| indices.contains(i))
            .map(|(_, cmd)| cmd.clone())
            .collect();
        Self { inner: core }
    }

    /// Check equality with another Geometry.
    #[gen_stub(skip)]
    fn __eq__(&self, other: &Geometry) -> bool {
        self.inner == other.inner
    }

    /// Simplify the geometry using Ramer-Douglas-Peucker.
    ///
    /// :param tolerance: Maximum deviation from original.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n log n) average time, O(n) space
    fn simplify(slf: Bound<'_, Self>, tolerance: f64) -> Bound<'_, Self> {
        let mut geo = slf.borrow_mut();
        if geo.inner.data.len() > 2 {
            let simplified = simplify_data(&geo.inner.data, tolerance);
            geo.inner.data = simplified;
        }
        drop(geo);
        slf
    }

    /// Convert all curves to line segments.
    ///
    /// :param tolerance: Maximum deviation from curves.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    fn linearize(slf: Bound<'_, Self>, tolerance: f64) -> Bound<'_, Self> {
        let mut geo = slf.borrow_mut();
        if !geo.inner.data.is_empty() {
            let linearized = linearize_data(&geo.inner.data, tolerance);
            geo.inner.data = linearized;
        }
        drop(geo);
        slf
    }

    /// Fit curves (beziers and arcs) to the linearized geometry.
    ///
    /// :param tolerance: Maximum deviation.
    /// :param beziers: Whether to fit bezier curves.
    /// :param arcs: Whether to fit arcs.
    /// :param on_progress: Optional progress callback called with ``(current, total)``.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n log n) average time, O(n) space
    #[pyo3(signature = (tolerance, beziers=true, arcs=true, on_progress=None))]
    fn fit_curves(
        slf: Bound<'_, Self>,
        tolerance: f64,
        beziers: bool,
        arcs: bool,
        on_progress: Option<pyo3::Py<pyo3::PyAny>>,
    ) -> Bound<'_, Self> {
        let on_progress_ref = on_progress.map(|cb| {
            let py = slf.py();
            move |current: usize, total: usize| {
                let args = (current as u64, total as u64);
                let _ = cb.call1(py, args);
            }
        });
        let cb = on_progress_ref.as_ref().map(|f| f as &dyn Fn(usize, usize));
        {
            let mut geo = slf.borrow_mut();
            if !geo.inner.data.is_empty() {
                let fitted =
                    fit_curves(&geo.inner.data, tolerance, beziers, arcs, cb);
                geo.inner.data = fitted;
            }
        }
        slf
    }

    /// Fit arcs only to the linearized geometry.
    ///
    /// :param tolerance: Maximum deviation.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n log n) average time, O(n) space
    fn fit_arcs(slf: Bound<'_, Self>, tolerance: f64) -> Bound<'_, Self> {
        Self::fit_curves(slf, tolerance, false, true, None)
    }

    /// Convert all arcs to bezier curves for uniform scaling.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    fn upgrade_to_scalable(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            if !geo.inner.data.is_empty() {
                let converted = convert_arcs_to_beziers(&geo.inner.data);
                geo.inner.data = converted;
                geo.inner.uniform_scalable = true;
            }
        }
        slf
    }

    /// Close gaps between sub-paths.
    ///
    /// :param tolerance: Max gap to close.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    #[pyo3(signature = (tolerance=None))]
    fn close_gaps(
        slf: Bound<'_, Self>,
        tolerance: Option<f64>,
    ) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            if !geo.inner.data.is_empty() {
                let closed = close_geometry_gaps_from_array(
                    &geo.inner.data,
                    tolerance.unwrap_or(0.5),
                );
                geo.inner.data = closed;
            }
        }
        slf
    }

    /// Remove duplicate segments from the geometry.
    ///
    /// :param tolerance: Maximum deviation for equality.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n log n) average time, O(n) space
    fn cleanup(slf: Bound<'_, Self>, tolerance: f64) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            if !geo.inner.data.is_empty() {
                let cleaned =
                    crate::geo::algo::cleanup::remove_duplicate_segments(
                        &geo.inner.data,
                        tolerance,
                    );
                geo.inner.data = cleaned;
            }
        }
        slf
    }

    /// Mirror the geometry along the X axis.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(1) space
    fn flip_x(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            for cmd in geo.inner.data.iter_mut() {
                match cmd {
                    CoreCommand::Move { ref mut end }
                    | CoreCommand::Line { ref mut end } => {
                        end.x = -end.x;
                    }
                    CoreCommand::Arc {
                        ref mut end,
                        ref mut center_offset,
                        ref mut normal,
                    } => {
                        end.x = -end.x;
                        center_offset.x = -center_offset.x;
                        // Reflection flips the winding direction (axial vector)
                        normal.z = -normal.z;
                    }
                    CoreCommand::Bezier {
                        ref mut end,
                        ref mut control1,
                        ref mut control2,
                    } => {
                        end.x = -end.x;
                        control1.x = -control1.x;
                        control2.x = -control2.x;
                    }
                }
            }
        }
        slf
    }

    /// Mirror the geometry along the Y axis.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(1) space
    fn flip_y(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            for cmd in geo.inner.data.iter_mut() {
                match cmd {
                    CoreCommand::Move { ref mut end }
                    | CoreCommand::Line { ref mut end } => {
                        end.y = -end.y;
                    }
                    CoreCommand::Arc {
                        ref mut end,
                        ref mut center_offset,
                        ref mut normal,
                    } => {
                        end.y = -end.y;
                        center_offset.y = -center_offset.y;
                        normal.z = -normal.z;
                    }
                    CoreCommand::Bezier {
                        ref mut end,
                        ref mut control1,
                        ref mut control2,
                    } => {
                        end.y = -end.y;
                        control1.y = -control1.y;
                        control2.y = -control2.y;
                    }
                }
            }
        }
        slf
    }

    /// Find the closest point on the path to (x, y).
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :returns: Tuple of (segment_index, t, point) or None.
    /// :complexity: O(n) time, O(1) space
    fn find_closest_point(
        &mut self,
        x: f64,
        y: f64,
    ) -> Option<(usize, f64, (f64, f64))> {
        if self.inner.data.is_empty() {
            return None;
        }
        find_closest_point_on_path_from_array(&self.inner.data, x, y)
            .map(|(a, b, p)| (a, b, (p.x, p.y)))
    }

    /// Get the point at parameter t on a segment.
    ///
    /// :param segment_index: Index of the segment.
    /// :param t: Parameter in [0, 1].
    /// :returns: The 3D point or None.
    /// :complexity: O(1) time, O(1) space
    fn get_point_at(
        &mut self,
        segment_index: usize,
        t: f64,
    ) -> Option<(f64, f64, f64)> {
        if self.inner.data.is_empty() {
            return None;
        }
        get_point_at_from_array(&self.inner.data, segment_index, t)
            .map(point3d_to_tuple)
    }

    /// Get the tangent vector at parameter t on a segment.
    ///
    /// :param segment_index: Index of the segment.
    /// :param t: Parameter in [0, 1].
    /// :returns: The normalized tangent vector or None.
    /// :complexity: O(1) time, O(1) space
    fn get_tangent_at(
        &mut self,
        segment_index: usize,
        t: f64,
    ) -> Option<(f64, f64)> {
        if self.inner.data.is_empty() {
            return None;
        }
        get_tangent_at_from_array(&self.inner.data, segment_index, t)
            .map(|p| (p.x, p.y))
    }

    /// Get the outward normal at parameter t on a segment.
    ///
    /// :param segment_index: Index of the segment.
    /// :param t: Parameter in [0, 1].
    /// :returns: Normal vector or None.
    /// :complexity: O(1) time, O(1) space
    fn get_outward_normal_at(
        &mut self,
        segment_index: usize,
        t: f64,
    ) -> Option<(f64, f64)> {
        if self.inner.data.is_empty() {
            return None;
        }
        get_outward_normal_at_from_array(&self.inner.data, segment_index, t)
            .map(|p| (p.x, p.y))
    }

    /// Draw an arc, converting it to bezier curves.
    ///
    /// :param x: End X coordinate.
    /// :param y: End Y coordinate.
    /// :param i: I offset to center.
    /// :param j: J offset to center.
    /// :param clockwise: Arc direction.
    /// :param z: End Z coordinate.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(1) time, O(1) space
    #[pyo3(signature = (x, y, i, j, clockwise=true, z=0.0))]
    fn arc_to_as_bezier(
        slf: Bound<'_, Self>,
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
    ) -> Bound<'_, Self> {
        let start_point = {
            let inner = slf.borrow();
            if let Some(last) = inner.inner.data().last() {
                last.end_point()
            } else {
                inner.inner.last_move_to
            }
        };
        let end_point = Point3D::new(x, y, z);
        let center_offset = Point::new(i, j);
        let beziers = convert_arc_to_beziers_from_array(
            start_point,
            end_point,
            center_offset,
            clockwise,
        );
        slf.borrow_mut().inner.data.extend(beziers);
        slf
    }

    /// Convert all Arc commands to Bezier curve approximations in-place.
    ///
    /// After this call, the geometry will only contain Move, Line, and Bezier
    /// commands.
    ///
    /// :complexity: O(n) time, O(n) space where n = number of commands
    fn convert_arcs_to_beziers(&mut self) -> PyResult<()> {
        self.inner.convert_arcs_to_beziers();
        Ok(())
    }

    /// Check if the geometry has self-intersections.
    ///
    /// :param fail_on_t_junction: Whether to fail on T-junctions.
    /// :returns: ``True`` if the geometry has self-intersections.
    /// :complexity: O(n²) worst-case time, O(1) space
    #[pyo3(signature = (fail_on_t_junction=false))]
    fn has_self_intersections(&mut self, fail_on_t_junction: bool) -> bool {
        if self.inner.data.is_empty() {
            return false;
        }
        check_self_intersection_from_array(&self.inner.data, fail_on_t_junction)
    }

    /// Check if this geometry intersects with another.
    ///
    /// :param other: The other geometry.
    /// :returns: ``True`` if the geometries intersect.
    /// :complexity: O(n * m) worst-case time, O(1) space where n and m are the number of segments in each geometry
    fn intersects_with(&mut self, other: &mut Geometry) -> bool {
        if self.inner.data.is_empty() || other.inner.data.is_empty() {
            return false;
        }
        check_intersection_from_array(
            &self.inner.data,
            &other.inner.data,
            false,
        )
    }

    /// Offset (grow/shrink) the geometry by the given amount.
    ///
    /// :param amount: Positive to grow, negative to shrink.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n log n) average time, O(n) space
    #[pyo3(signature = (amount))]
    fn grow(slf: Bound<'_, Self>, amount: f64) -> Bound<'_, Self> {
        let result = {
            let geo = slf.borrow();
            grow_geometry(&geo.inner, amount)
        };
        slf.borrow_mut().inner = result;
        slf
    }

    /// Check if this geometry encloses another.
    ///
    /// :param other: The potentially enclosed geometry.
    /// :returns: ``True`` if this geometry encloses the other.
    /// :complexity: O(n log n) average time, O(n) space
    fn encloses(&mut self, other: &mut Geometry) -> PyResult<bool> {
        Ok(crate::geo::algo::analysis::does_enclose(
            &self.inner,
            &other.inner,
        ))
    }

    /// Remove inner edges (shared between contours).
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    fn remove_inner_edges(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        let result = {
            let geo = slf.borrow();
            remove_inner_edges(&geo.inner)
        };
        slf.borrow_mut().inner = result;
        slf
    }

    /// Split contours into inner and outer groups.
    ///
    /// :returns: Tuple of ``(inner_contours, outer_contours)``, each a list of Geometry.
    /// :complexity: O(n log n) average time, O(n) space
    fn split_inner_and_outer_contours(
        &mut self,
    ) -> PyResult<(Vec<Geometry>, Vec<Geometry>)> {
        let contours = split_into_contours(&self.inner);
        let contour_refs: Vec<&crate::geo::geometry::Geometry> =
            contours.iter().collect();
        let (inner_indices, outer_indices) =
            split_inner_and_outer_contours(&contour_refs);
        let inner: Vec<Geometry> = inner_indices
            .into_iter()
            .map(|i| Geometry {
                inner: contours[i].copy(),
            })
            .collect();
        let outer: Vec<Geometry> = outer_indices
            .into_iter()
            .map(|i| Geometry {
                inner: contours[i].copy(),
            })
            .collect();
        Ok((inner, outer))
    }

    /// Map the geometry into a rectangular frame.
    ///
    /// :param origin: Frame origin (x, y).
    /// :param p_width: Frame width vector.
    /// :param p_height: Frame height vector.
    /// :param anchor_y: Y anchor position.
    /// :param stable_src_height: Stable source height for anchoring.
    /// :param anchor_x: X anchor position.
    /// :param stable_src_width: Stable source width for anchoring.
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    #[pyo3(signature = (origin, p_width, p_height, anchor_y=None, stable_src_height=None, anchor_x=None, stable_src_width=None))]
    #[allow(clippy::too_many_arguments)]
    fn map_to_frame(
        slf: Bound<'_, Self>,
        origin: (f64, f64),
        p_width: (f64, f64),
        p_height: (f64, f64),
        anchor_y: Option<f64>,
        stable_src_height: Option<f64>,
        anchor_x: Option<f64>,
        stable_src_width: Option<f64>,
    ) -> Bound<'_, Self> {
        let origin = Point::new(origin.0, origin.1);
        let p_width = Point::new(p_width.0, p_width.1);
        let p_height = Point::new(p_height.0, p_height.1);
        let result = {
            let geo = slf.borrow();
            map_geometry_to_frame(
                &geo.inner,
                origin,
                p_width,
                p_height,
                anchor_y,
                stable_src_height,
                anchor_x,
                stable_src_width,
            )
        };
        slf.borrow_mut().inner = result;
        slf
    }

    /// Split the geometry into individual contours.
    ///
    /// :returns: List of Geometry objects, one per contour.
    /// :complexity: O(n) time, O(n) space
    fn split_into_contours(&mut self) -> Vec<Geometry> {
        split_into_contours(&self.inner)
            .into_iter()
            .map(|g| Geometry { inner: g })
            .collect()
    }

    /// Split the geometry into connected components.
    ///
    /// :returns: List of Geometry objects, one per connected component.
    /// :complexity: O(n log n) average time, O(n) space
    fn split_into_components(&mut self) -> Vec<Geometry> {
        split_into_components(&self.inner)
            .into_iter()
            .map(|g| Geometry { inner: g })
            .collect()
    }

    /// Convert the geometry to a list of polygons.
    ///
    /// :param tolerance: Max deviation for linearization.
    /// :returns: List of polygons, each a list of ``(x, y)`` vertices.
    /// :complexity: O(n) time, O(n) space
    #[pyo3(signature = (tolerance=0.01))]
    fn to_polygons(&self, tolerance: f64) -> Vec<Vec<(f64, f64)>> {
        let mut linearized = self.inner.copy();
        if !linearized.data.is_empty() {
            let lin = linearize_data(&linearized.data, tolerance);
            linearized.data = lin;
        }
        let segs = linearized.segments();
        let mut result = Vec::new();
        for seg in &segs {
            if seg.len() < 3 {
                continue;
            }
            let poly: Vec<Point> =
                seg.iter().map(|p| Point::new(p.x, p.y)).collect();
            if let Some(cleaned) = crate::geo::shape::polygon::clean_polygon(
                &poly,
                0.01 * tolerance,
            ) {
                result.push(cleaned);
            } else if poly.len() >= 3 {
                result.push(poly);
            }
        }
        polygons_to_tuples(result)
    }

    /// Reverse the winding direction of all contours.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    fn reverse_contour(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        let result = {
            let geo = slf.borrow();
            reverse_contour(&geo.inner)
        };
        slf.borrow_mut().inner = result;
        slf
    }

    /// Close all open contours in the geometry.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n) time, O(n) space
    fn close_all_contours(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        let result = {
            let geo = slf.borrow();
            close_all_contours(&geo.inner)
        };
        slf.borrow_mut().inner = result;
        slf
    }

    /// Normalize winding orders (outer CCW, inner CW) of all contours.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n log n) average time, O(n) space
    fn normalize_winding_orders(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        let normalized = {
            let geo = slf.borrow();
            let contours = split_into_contours(&geo.inner);
            let contour_refs: Vec<&crate::geo::geometry::Geometry> =
                contours.iter().collect();
            normalize_winding_orders(&contour_refs)
        };
        let mut new_inner = CoreGeometry::new();
        for n in normalized {
            new_inner.extend(&n);
        }
        slf.borrow_mut().inner = new_inner;
        slf
    }

    /// Filter to only external (outermost) contours.
    ///
    /// :returns: The geometry (for method chaining).
    /// :complexity: O(n log n) average time, O(n) space
    fn filter_to_external_contours(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        let external = {
            let geo = slf.borrow();
            let contours = split_into_contours(&geo.inner);
            let contour_refs: Vec<&crate::geo::geometry::Geometry> =
                contours.iter().collect();
            filter_to_external_contours(&contour_refs)
        };
        let mut new_inner = CoreGeometry::new();
        for n in external {
            new_inner.extend(&n);
        }
        slf.borrow_mut().inner = new_inner;
        slf
    }

    /// Get valid contour data from the geometry's contours.
    ///
    /// :returns: List of dicts with keys "geo", "vertices",
    ///     "is_closed", "original_index".
    /// :complexity: O(n) time, O(n) space
    fn get_valid_contours_data<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        let contours = split_into_contours(&self.inner);
        let mut out: Vec<Bound<'py, PyDict>> = Vec::new();
        for (orig_idx, geo) in contours.iter().enumerate() {
            let single_result =
                get_valid_contours_data(std::slice::from_ref(geo));
            let Some((_, pts, closed)) = single_result.into_iter().next()
            else {
                continue;
            };
            let py_geo = Geometry { inner: geo.copy() };
            let dict = PyDict::new(py);
            dict.set_item("geo", py_geo)?;
            let py_pts: Vec<(f64, f64)> = points_to_tuples(pts);
            dict.set_item("vertices", py_pts)?;
            dict.set_item("is_closed", closed)?;
            dict.set_item("original_index", orig_idx)?;
            out.push(dict);
        }
        Ok(out)
    }

    /// Return a string representation of the geometry.
    ///
    /// :complexity: O(n) time, O(n) space
    fn __repr__(&mut self) -> String {
        let len = self.inner.len();
        let closed = self.inner.is_closed(1e-6);
        format!("<Geometry commands={} closed={}>", len, closed)
    }
}

impl Geometry {
    /// Parse a bezier from the current 10-element list format.
    ///
    /// Format: ``[B, end_x, end_y, end_z, c1x, c1y, c1z, c2x, c2y, c2z]``
    fn try_parse_bezier_current(
        cmd_list: &Bound<'_, PyList>,
        x: f64,
        y: f64,
        z: f64,
        geo: &mut Geometry,
    ) -> bool {
        if cmd_list.len() < 10 {
            return false;
        }
        let c1x: f64 = match cmd_list
            .get_item(4)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c1y: f64 = match cmd_list
            .get_item(5)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c1z: f64 = match cmd_list
            .get_item(6)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c2x: f64 = match cmd_list
            .get_item(7)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c2y: f64 = match cmd_list
            .get_item(8)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c2z: f64 = match cmd_list
            .get_item(9)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        geo.inner.bezier_to(
            Point3D::new(c1x, c1y, c1z),
            Point3D::new(c2x, c2y, c2z),
            Point3D::new(x, y, z),
        );
        true
    }

    /// Parse a bezier from the legacy 8-element list format.
    ///
    /// Format: ``[B, end_x, end_y, end_z, c1x, c1y, c2x, c2y]``
    /// (``c1z`` and ``c2z`` are implied 0.0).
    fn try_parse_bezier_legacy(
        cmd_list: &Bound<'_, PyList>,
        x: f64,
        y: f64,
        z: f64,
        geo: &mut Geometry,
    ) -> bool {
        if cmd_list.len() < 8 {
            return false;
        }
        let c1x: f64 = match cmd_list
            .get_item(4)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c1y: f64 = match cmd_list
            .get_item(5)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c2x: f64 = match cmd_list
            .get_item(6)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        let c2y: f64 = match cmd_list
            .get_item(7)
            .ok()
            .and_then(|v| v.extract::<f64>().ok())
        {
            Some(v) => v,
            None => return false,
        };
        geo.inner.bezier_to(
            Point3D::new(c1x, c1y, 0.0),
            Point3D::new(c2x, c2y, 0.0),
            Point3D::new(x, y, z),
        );
        true
    }
}
