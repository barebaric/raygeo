use numpy::ndarray;
use numpy::{IntoPyArray, PyArray2, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyType};
use pyo3_stub_gen::derive::{
    gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods,
};
use pyo3_stub_gen::inventory::submit;
use pyo3_stub_gen::{PyStubType, TypeInfo};

use crate::geo::algo::fitting::convert_arc_to_beziers_from_array;
use crate::geo::algo::analysis::get_point_and_tangent_at_from_array;
use crate::geo::algo::topology::get_valid_contours_data;
use crate::geo::math::map_geometry_to_frame;
use crate::{
    check_intersection_from_array, check_self_intersection_from_array,
    close_all_contours, close_geometry_gaps_from_array,
    convert_arcs_to_beziers, filter_to_external_contours,
    find_closest_point_on_path_from_array, fit_curves,
    get_outward_normal_at_from_array, grow_geometry, linearize_data,
    normalize_winding_orders, remove_inner_edges, reverse_contour,
    simplify_data, split_inner_and_outer_contours, split_into_components,
    split_into_contours, Command as CoreCommand, CommandRow,
    Geometry as CoreGeometry, Point, CMD_TYPE_ARC, CMD_TYPE_BEZIER,
    CMD_TYPE_LINE, CMD_TYPE_MOVE, COL_C1X, COL_C1Y, COL_C2X, COL_C2Y, COL_CW,
    COL_I, COL_J, COL_TYPE, COL_X, COL_Y, COL_Z,
};

#[pyclass(module = "raygeo.geo.path", frozen, eq, skip_from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyCommand {
    Move {
        end: (f64, f64, f64),
    },
    Line {
        end: (f64, f64, f64),
    },
    Arc {
        end: (f64, f64, f64),
        center_offset: (f64, f64),
        clockwise: bool,
    },
    Bezier {
        end: (f64, f64, f64),
        control1: (f64, f64),
        control2: (f64, f64),
    },
}

impl PyStubType for PyCommand {
    fn type_output() -> TypeInfo {
        TypeInfo::with_module("raygeo.PyCommand", "raygeo.geo".into())
    }
    fn type_input() -> TypeInfo {
        TypeInfo::with_module("raygeo.PyCommand", "raygeo.geo".into())
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

impl From<CoreCommand> for PyCommand {
    fn from(cmd: CoreCommand) -> Self {
        match cmd {
            CoreCommand::Move { end } => PyCommand::Move { end },
            CoreCommand::Line { end } => PyCommand::Line { end },
            CoreCommand::Arc {
                end,
                center_offset,
                clockwise,
            } => PyCommand::Arc {
                end,
                center_offset,
                clockwise,
            },
            CoreCommand::Bezier {
                end,
                control1,
                control2,
            } => PyCommand::Bezier {
                end,
                control1,
                control2,
            },
        }
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

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo", skip_from_py_object)]
#[derive(Clone)]
pub struct Geometry {
    pub(crate) inner: CoreGeometry,
}

submit! {
    gen_methods_from_python! {
        r#"
        class Geometry:
            def transform(self, matrix: geo.types.TransformMatrix) -> Geometry:
                """Apply a 4x4 affine transformation matrix.

                See ``raygeo.geo.types.TransformMatrix`` for the matrix layout.

                :param matrix: A 4x4 affine transformation matrix.
                :returns: A new transformed Geometry.
                """
                ...
        "#
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Geometry {
    #[classattr]
    const COL_TYPE: usize = crate::COL_TYPE;

    #[classattr]
    const COL_X: usize = crate::COL_X;

    #[classattr]
    const COL_Y: usize = crate::COL_Y;

    #[classattr]
    const COL_Z: usize = crate::COL_Z;

    #[classattr]
    const COL_I: usize = crate::COL_I;

    #[classattr]
    const COL_J: usize = crate::COL_J;

    #[classattr]
    const COL_CW: usize = crate::COL_CW;

    #[classattr]
    const COL_C1X: usize = crate::COL_C1X;

    #[classattr]
    const COL_C1Y: usize = crate::COL_C1Y;

    #[classattr]
    const COL_C2X: usize = crate::COL_C2X;

    #[classattr]
    const COL_C2Y: usize = crate::COL_C2Y;

    #[classattr]
    const GEO_ARRAY_COLS: usize = crate::GEO_ARRAY_COLS;

    #[classattr]
    const CMD_TYPE_MOVE: f64 = crate::CMD_TYPE_MOVE as f64;

    #[classattr]
    const CMD_TYPE_LINE: f64 = crate::CMD_TYPE_LINE as f64;

    #[classattr]
    const CMD_TYPE_ARC: f64 = crate::CMD_TYPE_ARC as f64;

    #[classattr]
    const CMD_TYPE_BEZIER: f64 = crate::CMD_TYPE_BEZIER as f64;

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
        let mut borrowed = slf.borrow_mut();
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
    #[pyo3(signature = (x, y, z=0.0))]
    fn move_to(&mut self, x: f64, y: f64, z: f64) {
        self.inner.move_to(x, y, z);
    }

    /// Draw a line to the given coordinates.
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :param z: Z coordinate (default 0.0).
    #[pyo3(signature = (x, y, z=0.0))]
    fn line_to(&mut self, x: f64, y: f64, z: f64) {
        self.inner.line_to(x, y, z);
    }

    /// Close the current sub-path.
    fn close_path(&mut self) {
        self.inner.close_path();
    }

    /// Draw an arc to the given coordinates.
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :param i: I offset from current point to center.
    /// :param j: J offset from current point to center.
    /// :param clockwise: Whether the arc is clockwise.
    /// :param z: Z coordinate (default 0.0).
    #[pyo3(signature = (x, y, i=0.0, j=0.0, clockwise=true, z=0.0))]
    fn arc_to(
        &mut self,
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
    ) {
        self.inner.arc_to(x, y, i, j, clockwise, z);
    }

    /// Draw a cubic bezier curve.
    ///
    /// :param x: End X coordinate.
    /// :param y: End Y coordinate.
    /// :param c1x: First control point X.
    /// :param c1y: First control point Y.
    /// :param c2x: Second control point X.
    /// :param c2y: Second control point Y.
    /// :param z: End Z coordinate (default 0.0).
    #[pyo3(signature = (x, y, c1x, c1y, c2x, c2y, z=0.0))]
    fn bezier_to(
        &mut self,
        x: f64,
        y: f64,
        c1x: f64,
        c1y: f64,
        c2x: f64,
        c2y: f64,
        z: f64,
    ) {
        self.inner.bezier_to(((c1x, c1y), (c2x, c2y), (x, y)), z);
    }

    /// Return the number of commands.
    fn __len__(&mut self) -> usize {
        self.inner.len()
    }

    /// Return a hash of the geometry data.
    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        for row in self.inner.data() {
            for &val in row.iter() {
                let normalized = if val == 0.0 { 0.0 } else { val };
                let bits = if normalized.is_nan() {
                    f64::NAN.to_bits()
                } else {
                    normalized.to_bits()
                };
                bits.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Check if the geometry has no commands.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Remove all commands from the geometry.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// The coordinates of the last move-to command.
    #[getter]
    fn last_move_to(&self) -> (f64, f64, f64) {
        self.inner.last_move_to
    }

    #[setter]
    fn set_last_move_to(&mut self, value: (f64, f64, f64)) {
        self.inner.last_move_to = value;
    }

    /// Whether the geometry uses uniform scalable arcs.
    #[getter]
    fn uniform_scalable(&self) -> bool {
        self.inner.uniform_scalable
    }

    #[setter(uniform_scalable)]
    fn set_uniform_scalable(&mut self, value: bool) {
        self.inner.uniform_scalable = value;
    }

    /// Return a deep copy of this geometry.
    fn copy(&self) -> Self {
        Geometry {
            inner: self.inner.copy(),
        }
    }

    /// Get the last point in the geometry.
    fn get_last_point(&self) -> (f64, f64, f64) {
        let data = self.inner.data();
        if let Some(last) = data.last() {
            return (last[COL_X], last[COL_Y], last[COL_Z]);
        }
        (0.0, 0.0, 0.0)
    }

    /// Apply a 4x4 affine transformation matrix.
    ///
    /// :param matrix: A 4x4 transformation matrix as list of lists.
    #[gen_stub(skip)]
    fn transform(
        slf: Bound<'_, Self>,
        matrix: Vec<Vec<f64>>,
    ) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            let mat: [[f64; 4]; 4] = [
                [matrix[0][0], matrix[0][1], matrix[0][2], matrix[0][3]],
                [matrix[1][0], matrix[1][1], matrix[1][2], matrix[1][3]],
                [matrix[2][0], matrix[2][1], matrix[2][2], matrix[2][3]],
                [matrix[3][0], matrix[3][1], matrix[3][2], matrix[3][3]],
            ];
            geo.inner.transform(&mat);
        }
        slf
    }

    /// Append another geometry's commands to this one.
    ///
    /// :param other: The geometry to append.
    fn extend(&mut self, other: &Geometry) {
        self.inner.extend(&other.inner);
    }

    /// Return the bounding rectangle (x_min, x_max, y_min, y_max).
    fn rect(&mut self) -> (f64, f64, f64, f64) {
        self.inner.rect()
    }

    /// Return the total path distance.
    fn distance(&mut self) -> f64 {
        self.inner.distance()
    }

    /// Return the signed area of the geometry.
    fn area(&mut self) -> f64 {
        self.inner.area()
    }

    /// Check if the geometry forms a closed path.
    ///
    /// :param tolerance: Max gap between start and end point.
    #[pyo3(signature = (tolerance=1e-6))]
    fn is_closed(&mut self, tolerance: f64) -> bool {
        self.inner.is_closed(tolerance)
    }

    /// Return the geometry split into segments of connected commands.
    fn segments(&mut self) -> Vec<Vec<(f64, f64, f64)>> {
        self.inner.segments()
    }

    /// The command data as a numpy array of shape
    /// (N, 8), or None if empty.
    #[getter]
    fn data<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyArray2<f64>>>> {
        let data = self.inner.synced_data();
        if data.is_empty() {
            return Ok(None);
        }
        let rows = data.len();
        let flat: Vec<f64> = data.iter().flatten().copied().collect();
        match ndarray::Array2::from_shape_vec((rows, 8usize), flat) {
            Ok(arr) => Ok(Some(arr.into_pyarray(py))),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("{}", e),
            )),
        }
    }

    #[setter(data)]
    fn set_data(
        &mut self,
        value: Option<Bound<'_, PyArray2<f64>>>,
    ) -> PyResult<()> {
        let data = self.inner.synced_data_mut();
        let Some(arr) = value else {
            data.clear();
            return Ok(());
        };
        let readonly = arr.readonly();
        let view = readonly.as_array();
        data.clear();
        for row in view.rows() {
            let mut chunk = [0.0; 8];
            let row_slice: &[f64] = row.as_slice().unwrap();
            chunk.copy_from_slice(row_slice);
            data.push(chunk);
        }
        Ok(())
    }

    /// Get the command at the given index as a raw tuple.
    ///
    /// :param index: Command index (negative returns None).
    fn get_command_at(&mut self, index: isize) -> Option<CommandRow> {
        if index < 0 {
            return None;
        }
        let data = self.inner.synced_data();
        data.get(index as usize)
            .map(|r| (r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7]))
    }

    /// Iterate over all commands as raw tuples.
    fn iter_commands(&mut self) -> Vec<CommandRow> {
        let data = self.inner.synced_data();
        data.iter()
            .map(|r| (r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7]))
            .collect()
    }

    /// Iterate over all commands as typed PyCommand objects.
    fn iter_typed_commands(&mut self) -> PyResult<Vec<PyCommand>> {
        let data = self.inner.synced_data();
        data.iter()
            .map(|r| {
                CoreCommand::from_row(r).map(PyCommand::from).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(e.to_string())
                })
            })
            .collect()
    }

    /// Get the typed command at the given index.
    ///
    /// :param index: Command index.
    fn get_typed_command_at(
        &mut self,
        index: isize,
    ) -> PyResult<Option<PyCommand>> {
        if index < 0 {
            return Ok(None);
        }
        let data = self.inner.synced_data();
        match data.get(index as usize) {
            Some(row) => Ok(Some(
                CoreCommand::from_row(row).map(PyCommand::from).map_err(
                    |e| pyo3::exceptions::PyValueError::new_err(e.to_string()),
                )?,
            )),
            None => Ok(None),
        }
    }

    /// Serialize the geometry to a dictionary.
    fn dump<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let last_move_to = self.inner.last_move_to;
        let uniform_scalable = self.inner.uniform_scalable;
        let data = self.inner.synced_data();
        let dict = PyDict::new(py);
        dict.set_item(
            "last_move_to",
            vec![last_move_to.0, last_move_to.1, last_move_to.2],
        )?;
        dict.set_item("uniform_scalable", uniform_scalable)?;
        let commands = PyList::empty(py);
        for row in data {
            let cmd_type = row[0] as i32;
            let cmd = PyList::empty(py);
            if cmd_type == CMD_TYPE_MOVE as i32 {
                cmd.append("M")?;
                cmd.append(row[1])?;
                cmd.append(row[2])?;
                cmd.append(row[3])?;
            } else if cmd_type == CMD_TYPE_LINE as i32 {
                cmd.append("L")?;
                cmd.append(row[1])?;
                cmd.append(row[2])?;
                cmd.append(row[3])?;
            } else if cmd_type == CMD_TYPE_ARC as i32 {
                cmd.append("A")?;
                cmd.append(row[1])?;
                cmd.append(row[2])?;
                cmd.append(row[3])?;
                cmd.append(row[4])?;
                cmd.append(row[5])?;
                cmd.append(row[6])?;
            } else if cmd_type == CMD_TYPE_BEZIER as i32 {
                cmd.append("B")?;
                cmd.append(row[1])?;
                cmd.append(row[2])?;
                cmd.append(row[3])?;
                cmd.append(row[4])?;
                cmd.append(row[5])?;
                cmd.append(row[6])?;
                cmd.append(row[7])?;
            }
            commands.append(cmd)?;
        }
        dict.set_item("commands", commands)?;
        Ok(dict)
    }

    /// Serialize the geometry to a dictionary.
    fn to_dict<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        self.dump(py)
    }

    /// Load a geometry from a dictionary.
    ///
    /// :param data: A dictionary as produced by
    ///     :meth:`to_dict`.
    #[classmethod]
    fn load<'py>(
        _cls: &Bound<'py, PyType>,
        data: &Bound<'py, PyDict>,
    ) -> PyResult<Self> {
        let mut geo = Self::new();
        if let Some(lmt) = data.get_item("last_move_to")? {
            if let Ok(lmt_list) = lmt.extract::<Vec<f64>>() {
                if lmt_list.len() >= 3 {
                    geo.inner.last_move_to =
                        (lmt_list[0], lmt_list[1], lmt_list[2]);
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
                            "M" => geo.inner.move_to(x, y, z),
                            "L" => geo.inner.line_to(x, y, z),
                            "A" => {
                                if let (
                                    Some(i_val),
                                    Some(j_val),
                                    Some(cw_val),
                                ) =
                                    (
                                        cmd_list.get_item(4).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                        cmd_list.get_item(5).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                        cmd_list.get_item(6).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                    )
                                {
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
                            "B" => {
                                if let (
                                    Some(c1x),
                                    Some(c1y),
                                    Some(c2x),
                                    Some(c2y),
                                ) =
                                    (
                                        cmd_list.get_item(4).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                        cmd_list.get_item(5).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                        cmd_list.get_item(6).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                        cmd_list.get_item(7).ok().and_then(
                                            |v| v.extract::<f64>().ok(),
                                        ),
                                    )
                                {
                                    geo.inner.bezier_to(
                                        ((c1x, c1y), (c2x, c2y), (x, y)),
                                        z,
                                    );
                                }
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
                            "MoveToCommand" => geo.inner.move_to(x, y, z),
                            "LineToCommand" => geo.inner.line_to(x, y, z),
                            "CurveToCommand" | "BezierToCommand" => {
                                if let (Ok(Some(c1)), Ok(Some(c2))) = (
                                    cmd_dict.get_item("control1"),
                                    cmd_dict.get_item("control2"),
                                ) {
                                    let c1v = c1.extract::<(f64, f64)>();
                                    let c2v = c2.extract::<(f64, f64)>();
                                    if let (Ok((c1x, c1y)), Ok((c2x, c2y))) =
                                        (c1v, c2v)
                                    {
                                        geo.inner.bezier_to(
                                            ((c1x, c1y), (c2x, c2y), (x, y)),
                                            z,
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
                            "ClosePathCommand" => geo.inner.close_path(),
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(geo)
    }

    /// Create a Geometry from a dictionary.
    ///
    /// :param data: A dictionary as produced by
    ///     :meth:`to_dict`.
    #[classmethod]
    fn from_dict<'py>(
        _cls: &Bound<'py, PyType>,
        data: &Bound<'py, PyDict>,
    ) -> PyResult<Self> {
        Self::load(_cls, data)
    }

    /// Create a Geometry from a sequence of points.
    ///
    /// :param points: A sequence of (x, y) or
    ///     (x, y, z) coordinate tuples.
    /// :param close: Whether to close the path.
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
        geo.move_to(first.x, first.y, first.z);
        for p in &points_vec[1..] {
            geo.line_to(p.x, p.y, p.z);
        }
        if close && points_vec.len() > 2 {
            geo.close_path();
        }
        Ok(geo)
    }

    /// Check equality with another Geometry.
    fn __eq__(&self, other: &Geometry) -> bool {
        self.inner == other.inner
    }

    /// Simplify the geometry using Ramer-Douglas-Peucker.
    ///
    /// :param tolerance: Maximum deviation from original.
    fn simplify(&mut self, tolerance: f64) -> Self {
        let data = self.inner.synced_data();
        if data.len() > 2 {
            let simplified = simplify_data(data, tolerance);
            *self.inner.synced_data_mut() = simplified;
        }
        self.clone()
    }

    /// Convert all curves to line segments.
    ///
    /// :param tolerance: Maximum deviation from curves.
    fn linearize(&mut self, tolerance: f64) -> Self {
        let data = self.inner.synced_data();
        if !data.is_empty() {
            let linearized = linearize_data(data, tolerance);
            *self.inner.synced_data_mut() = linearized;
        }
        self.clone()
    }

    /// Fit curves (beziers and arcs) to the linearized geometry.
    ///
    /// :param tolerance: Maximum deviation.
    /// :param beziers: Whether to fit bezier curves.
    /// :param arcs: Whether to fit arcs.
    /// :param on_progress: Optional progress callback.
    #[pyo3(signature = (tolerance, beziers=true, arcs=true, on_progress=None))]
    fn fit_curves(
        &mut self,
        tolerance: f64,
        beziers: bool,
        arcs: bool,
        on_progress: Option<pyo3::Py<pyo3::PyAny>>,
    ) -> Self {
        let _ = on_progress;
        let data = self.inner.synced_data();
        if !data.is_empty() {
            let fitted = fit_curves(data, tolerance, beziers, arcs);
            *self.inner.synced_data_mut() = fitted;
        }
        self.clone()
    }

    /// Fit arcs only to the linearized geometry.
    ///
    /// :param tolerance: Maximum deviation.
    fn fit_arcs(&mut self, tolerance: f64) -> Self {
        self.fit_curves(tolerance, false, true, None)
    }

    /// Convert all arcs to bezier curves for uniform scaling.
    fn upgrade_to_scalable(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            let data = geo.inner.synced_data();
            if !data.is_empty() {
                let converted = convert_arcs_to_beziers(data);
                *geo.inner.synced_data_mut() = converted;
                geo.inner.uniform_scalable = true;
            }
        }
        slf
    }

    /// Close gaps between sub-paths.
    ///
    /// :param tolerance: Max gap to close.
    #[pyo3(signature = (tolerance=None))]
    fn close_gaps(
        slf: Bound<'_, Self>,
        tolerance: Option<f64>,
    ) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            let data = geo.inner.synced_data();
            if !data.is_empty() {
                let closed = close_geometry_gaps_from_array(
                    data,
                    tolerance.unwrap_or(0.5),
                );
                *geo.inner.synced_data_mut() = closed;
            }
        }
        slf
    }

    /// Remove duplicate segments from the geometry.
    ///
    /// :param tolerance: Maximum deviation for equality.
    fn cleanup(slf: Bound<'_, Self>, tolerance: f64) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            let data = geo.inner.synced_data();
            if !data.is_empty() {
                let cleaned =
                    crate::remove_duplicate_segments(data, tolerance);
                *geo.inner.synced_data_mut() = cleaned;
            }
        }
        slf
    }

    /// Append rows of command data to the geometry.
    ///
    /// :param rows: A numpy array of shape (N, 8)
    ///     containing command rows, or None.
    fn append_data<'py>(
        &mut self,
        py: Python<'py>,
        rows: Option<Py<PyArray2<f64>>>,
    ) -> PyResult<()> {
        let Some(arr) = rows else {
            return Ok(());
        };
        let arr = arr.bind(py);
        let data = self.inner.synced_data_mut();
        let readonly: numpy::PyReadonlyArray<f64, numpy::ndarray::Ix2> =
            arr.readonly();
        let view: numpy::ndarray::ArrayView2<f64> = readonly.as_array();
        for row in view.rows() {
            let mut chunk = [0.0; 8];
            let row_slice: &[f64] = row.as_slice().unwrap();
            chunk.copy_from_slice(row_slice);
            data.push(chunk);
        }
        Ok(())
    }

    /// Mirror the geometry along the X axis.
    fn flip_x(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            for row in geo.inner.synced_data_mut().iter_mut() {
                row[COL_X] = -row[COL_X];
                let cmd_type = row[COL_TYPE] as i32;
                if cmd_type == CMD_TYPE_BEZIER as i32 {
                    row[COL_C1X] = -row[COL_C1X];
                    row[COL_C2X] = -row[COL_C2X];
                } else if cmd_type == CMD_TYPE_ARC as i32 {
                    row[COL_I] = -row[COL_I];
                    row[COL_CW] = if row[COL_CW] != 0.0 { 0.0 } else { 1.0 };
                }
            }
        }
        slf
    }

    /// Mirror the geometry along the Y axis.
    fn flip_y(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        {
            let mut geo = slf.borrow_mut();
            for row in geo.inner.synced_data_mut().iter_mut() {
                row[COL_Y] = -row[COL_Y];
                let cmd_type = row[COL_TYPE] as i32;
                if cmd_type == CMD_TYPE_BEZIER as i32 {
                    row[COL_C1Y] = -row[COL_C1Y];
                    row[COL_C2Y] = -row[COL_C2Y];
                } else if cmd_type == CMD_TYPE_ARC as i32 {
                    row[COL_J] = -row[COL_J];
                    row[COL_CW] = if row[COL_CW] != 0.0 { 0.0 } else { 1.0 };
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
    fn find_closest_point(
        &mut self,
        x: f64,
        y: f64,
    ) -> Option<(usize, f64, (f64, f64))> {
        let data = self.inner.synced_data();
        if data.is_empty() {
            return None;
        }
        find_closest_point_on_path_from_array(data, x, y)
    }

    /// Get the point and tangent vector at parameter t on a segment.
    ///
    /// :param segment_index: Index of the segment.
    /// :param t: Parameter in [0, 1].
    /// :returns: Tuple of (point, tangent) or None.
    fn get_point_and_tangent_at(
        &mut self,
        segment_index: usize,
        t: f64,
    ) -> Option<((f64, f64), (f64, f64))> {
        let data = self.inner.synced_data();
        if data.is_empty() {
            return None;
        }
        get_point_and_tangent_at_from_array(data, segment_index, t)
    }

    /// Get the outward normal at parameter t on a segment.
    ///
    /// :param segment_index: Index of the segment.
    /// :param t: Parameter in [0, 1].
    /// :returns: Normal vector or None.
    fn get_outward_normal_at(
        &mut self,
        segment_index: usize,
        t: f64,
    ) -> Option<(f64, f64)> {
        let data = self.inner.synced_data();
        if data.is_empty() {
            return None;
        }
        get_outward_normal_at_from_array(data, segment_index, t)
    }

    /// Draw an arc, converting it to bezier curves.
    ///
    /// :param x: End X coordinate.
    /// :param y: End Y coordinate.
    /// :param i: I offset to center.
    /// :param j: J offset to center.
    /// :param clockwise: Arc direction.
    /// :param z: End Z coordinate.
    #[pyo3(signature = (x, y, i, j, clockwise=true, z=0.0))]
    fn arc_to_as_bezier(
        &mut self,
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
    ) {
        let start_point = if let Some(last) = self.inner.data().last() {
            (last[COL_X], last[COL_Y], last[COL_Z])
        } else {
            self.inner.last_move_to
        };
        let end_point = (x, y, z);
        let center_offset = (i, j);
        let bezier_rows = convert_arc_to_beziers_from_array(
            start_point,
            end_point,
            center_offset,
            clockwise,
        );
        let data = self.inner.synced_data_mut();
        for row in bezier_rows {
            data.push(row);
        }
    }

    /// Check if the geometry has self-intersections.
    ///
    /// :param fail_on_t_junction: Whether to fail on T-junctions.
    #[pyo3(signature = (fail_on_t_junction=false))]
    fn has_self_intersections(&mut self, fail_on_t_junction: bool) -> bool {
        let data = self.inner.synced_data();
        if data.is_empty() {
            return false;
        }
        check_self_intersection_from_array(data, fail_on_t_junction)
    }

    /// Check if this geometry intersects with another.
    ///
    /// :param other: The other geometry.
    fn intersects_with(&mut self, other: &mut Geometry) -> bool {
        let data = self.inner.synced_data();
        let other_data = other.inner.synced_data();
        if data.is_empty() || other_data.is_empty() {
            return false;
        }
        check_intersection_from_array(data, other_data, false)
    }

    /// Offset (grow/shrink) the geometry by the given amount.
    ///
    /// :param amount: Positive to grow, negative to shrink.
    #[pyo3(signature = (amount))]
    fn grow(&self, amount: f64) -> Self {
        let result = grow_geometry(&self.inner, amount);
        Geometry { inner: result }
    }

    /// Check if this geometry encloses another.
    ///
    /// :param other: The potentially enclosed geometry.
    fn encloses(&mut self, other: &mut Geometry) -> PyResult<bool> {
        Ok(crate::does_enclose(&self.inner, &other.inner))
    }

    /// Remove inner edges (shared between contours).
    fn remove_inner_edges(&mut self) -> PyResult<Geometry> {
        Ok(Geometry {
            inner: remove_inner_edges(&self.inner),
        })
    }

    /// Split contours into inner and outer groups.
    fn split_inner_and_outer_contours(
        &mut self,
    ) -> PyResult<(Vec<Geometry>, Vec<Geometry>)> {
        let contours = split_into_contours(&self.inner);
        let (inner_indices, outer_indices) =
            split_inner_and_outer_contours(&contours);
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
    #[pyo3(signature = (origin, p_width, p_height, anchor_y=None, stable_src_height=None, anchor_x=None, stable_src_width=None))]
    fn map_to_frame(
        &self,
        origin: (f64, f64),
        p_width: (f64, f64),
        p_height: (f64, f64),
        anchor_y: Option<f64>,
        stable_src_height: Option<f64>,
        anchor_x: Option<f64>,
        stable_src_width: Option<f64>,
    ) -> Geometry {
        let result = map_geometry_to_frame(
            &self.inner,
            origin,
            p_width,
            p_height,
            anchor_y,
            stable_src_height,
            anchor_x,
            stable_src_width,
        );
        Geometry { inner: result }
    }

    /// Split the geometry into individual contours.
    fn split_into_contours(&mut self) -> Vec<Geometry> {
        split_into_contours(&self.inner)
            .into_iter()
            .map(|g| Geometry { inner: g })
            .collect()
    }

    /// Split the geometry into connected components.
    fn split_into_components(&mut self) -> Vec<Geometry> {
        split_into_components(&self.inner)
            .into_iter()
            .map(|g| Geometry { inner: g })
            .collect()
    }

    /// Convert the geometry to a list of polygons.
    ///
    /// :param tolerance: Max deviation for linearization.
    #[pyo3(signature = (tolerance=0.01))]
    fn to_polygons(&self, tolerance: f64) -> Vec<Vec<Point>> {
        let mut linearized = self.inner.copy();
        if !linearized.data().is_empty() {
            let lin = linearize_data(linearized.synced_data(), tolerance);
            *linearized.synced_data_mut() = lin;
        }
        let segs = linearized.segments();
        let mut result = Vec::new();
        for seg in &segs {
            if seg.len() < 3 {
                continue;
            }
            let poly: Vec<Point> = seg.iter().map(|p| (p.0, p.1)).collect();
            if let Some(cleaned) =
                crate::clean_polygon(&poly, 0.01 * tolerance)
            {
                result.push(cleaned);
            } else if poly.len() >= 3 {
                result.push(poly);
            }
        }
        result
    }

    /// Reverse the winding direction of all contours.
    fn reverse_contour(&self) -> Geometry {
        Geometry {
            inner: reverse_contour(&self.inner),
        }
    }

    /// Close all open contours in the geometry.
    fn close_all_contours(&self) -> Geometry {
        Geometry {
            inner: close_all_contours(&self.inner),
        }
    }

    /// Normalize winding orders (outer CCW, inner CW) of all contours.
    fn normalize_winding_orders(&self) -> Vec<Geometry> {
        let contours = split_into_contours(&self.inner);
        normalize_winding_orders(&contours)
            .into_iter()
            .map(|g| Geometry { inner: g })
            .collect()
    }

    /// Filter to only external (outermost) contours.
    fn filter_to_external_contours(&self) -> Vec<Geometry> {
        let contours = split_into_contours(&self.inner);
        filter_to_external_contours(&contours)
            .into_iter()
            .map(|g| Geometry { inner: g })
            .collect()
    }

    /// Get valid contour data from the geometry's contours.
    ///
    /// :returns: List of dicts with keys "geo", "vertices",
    ///     "is_closed", "original_index".
    fn get_valid_contours_data<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        let contours = split_into_contours(&self.inner);
        let mut out: Vec<Bound<'py, PyDict>> = Vec::new();
        for (orig_idx, geo) in contours.iter().enumerate() {
            let single_result =
                get_valid_contours_data(std::slice::from_ref(geo));
            if single_result.is_empty() {
                continue;
            }
            let (_, pts, closed) = single_result.into_iter().next().unwrap();
            let py_geo = Geometry { inner: geo.copy() };
            let dict = PyDict::new(py);
            dict.set_item("geo", py_geo)?;
            let py_pts: Vec<(f64, f64)> = pts;
            dict.set_item("vertices", py_pts)?;
            dict.set_item("is_closed", closed)?;
            dict.set_item("original_index", orig_idx)?;
            out.push(dict);
        }
        Ok(out)
    }

    /// Return a string representation of the geometry.
    fn __repr__(&mut self) -> String {
        let len = self.inner.len();
        let closed = self.inner.is_closed(1e-6);
        format!("<Geometry commands={} closed={}>", len, closed)
    }
}
