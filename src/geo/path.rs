use crate::geo::flex_point::PyPoint3D;
use crate::geo::geometry::{Geometry, PyCommand};
use numpy::PyArray2;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use raygeo_core::geo::algo::fitting::{
    convert_arc_to_beziers_from_array, create_arc_cmd, create_line_cmd,
    flatten_to_points, linearize_geometry,
};
use raygeo_core::geo::analysis::{
    get_point_and_tangent_at_from_array, get_subpath_area_from_array,
    get_subpath_vertices_from_array, partial_segment_from_row,
    segment_length_from_row_flat,
};
use raygeo_core::geo::algo::cleanup::get_segment_key;
use raygeo_core::geo::shape::point::are_points_equal;
use raygeo_core::{
    check_intersection_from_array, check_self_intersection_from_array,
    fit_curves, remove_duplicate_segments, Point, CMD_TYPE_ARC,
    CMD_TYPE_BEZIER, CMD_TYPE_LINE,
};

fn to_data_array(data: Vec<Vec<f64>>) -> Vec<[f64; 8]> {
    data.into_iter()
        .map(|r| {
            let mut a = [0.0; 8];
            let len = r.len().min(8);
            a[..len].copy_from_slice(&r[..len]);
            a
        })
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    def get_bounding_rect_from_array(
        data: Sequence[Sequence[float]],
    ) -> Rect:
        """Compute the bounding rectangle of path data.

        :param data: Array of command data (rows of 8 floats).
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn get_bounding_rect_from_array(data: Vec<Vec<f64>>) -> (f64, f64, f64, f64) {
    let arr = to_data_array(data);
    raygeo_core::get_bounding_rect_from_array(&arr)
}

#[gen_stub_pyfunction(
    python = r#"
    def get_total_distance_from_array(
        data: Sequence[Sequence[float]],
    ) -> float:
        """Compute the total distance of a path.

        :param data: Array of command data.
        :returns: Total path length.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn get_total_distance_from_array(data: Vec<Vec<f64>>) -> f64 {
    let arr = to_data_array(data);
    raygeo_core::get_total_distance_from_array(&arr)
}

#[gen_stub_pyfunction(
    python = r#"
    def extract_overcut_rows(
        data: Optional[Sequence[Sequence[float]]],
        max_length: float,
    ) -> Optional[Any]:
        """Extract rows that exceed a maximum length.

        :param data: Array of command data or None.
        :param max_length: Maximum allowed segment length.
        :returns: Numpy array of overcut rows, or None.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn extract_overcut_rows(
    py: Python<'_>,
    data: Option<Vec<Vec<f64>>>,
    max_length: f64,
) -> Option<Bound<'_, pyo3::types::PyAny>> {
    let data = data?;
    let arr = to_data_array(data);
    raygeo_core::extract_overcut_rows(&arr, max_length).map(|rows| {
        let vecs: Vec<Vec<f64>> =
            rows.into_iter().map(|r| r.to_vec()).collect();
        PyArray2::<f64>::from_vec2(py, &vecs)
            .expect("failed to create numpy array")
            .as_any()
            .clone()
    })
}

#[gen_stub_pyfunction(
    python = r#"
    def get_subpath_vertices_from_array(
        data: Sequence[Sequence[float]],
        subpath_index: int,
    ) -> Polygon:
        """Get the vertices of a subpath.

        :param data: Array of command data.
        :param subpath_index: Index of the subpath.
        :returns: List of vertex points (x, y).
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "get_subpath_vertices_from_array")]
fn get_subpath_vertices_from_array_py(
    data: Vec<Vec<f64>>,
    subpath_index: usize,
) -> Vec<Point> {
    let arr = to_data_array(data);
    get_subpath_vertices_from_array(&arr, subpath_index)
}

#[gen_stub_pyfunction(
    python = r#"
    def get_subpath_area_from_array(
        data: Sequence[Sequence[float]],
        subpath_index: int,
    ) -> float:
        """Get the signed area of a subpath.

        :param data: Array of command data.
        :param subpath_index: Index of the subpath.
        :returns: Signed area of the subpath.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "get_subpath_area_from_array")]
fn get_subpath_area_from_array_py(
    data: Vec<Vec<f64>>,
    subpath_index: usize,
) -> f64 {
    let arr = to_data_array(data);
    get_subpath_area_from_array(&arr, subpath_index)
}

#[gen_stub_pyfunction(
    python = r#"
    def get_area_from_array(data: Sequence[Sequence[float]]) -> float:
        """Compute the total area of path data.

        :param data: Array of command data.
        :returns: Total signed area.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn get_area_from_array(data: Vec<Vec<f64>>) -> f64 {
    let arr = to_data_array(data);
    raygeo_core::get_area_from_array(&arr)
}

#[gen_stub_pyfunction(
    python = r#"
    def get_path_winding_order_from_array(
        data: Sequence[Sequence[float]],
        start_cmd_index: int,
    ) -> str:
        """Get the winding order (CW/CCW) of a path.

        :param data: Array of command data.
        :param start_cmd_index: Index of the first command of the path.
        :returns: Winding order as "CW" or "CCW".
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn get_path_winding_order_from_array(
    data: Vec<Vec<f64>>,
    start_cmd_index: usize,
) -> String {
    let arr = to_data_array(data);
    raygeo_core::geo::analysis::get_path_winding_order_from_array(
        &arr,
        start_cmd_index,
    )
    .to_string()
}

#[gen_stub_pyfunction(
    python = r#"
    def get_point_and_tangent_at(
        data: Sequence[Sequence[float]],
        row_index: int,
        t: float,
    ) -> Optional[tuple[Point, Point]]:
        """Get the point and tangent at a parameter t on a segment.

        :param data: Array of command data.
        :param row_index: Row index of the segment.
        :param t: Parameter value along the segment (0..1).
        :returns: Tuple of (point, tangent) or None.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "get_point_and_tangent_at")]
fn get_point_and_tangent_at_py(
    data: Vec<Vec<f64>>,
    row_index: usize,
    t: f64,
) -> Option<(Point, Point)> {
    let arr = to_data_array(data);
    get_point_and_tangent_at_from_array(&arr, row_index, t)
}

#[gen_stub_pyfunction(
    python = r#"
    def optimize_path_from_array(
        data: Optional[Sequence[Sequence[float]]],
        tolerance: float,
        fit_arcs: bool,
    ) -> Any:
        """Optimize a path by fitting arcs.

        :param data: Array of command data or None.
        :param tolerance: Fitting tolerance.
        :param fit_arcs: Whether to fit arcs (vs. only lines).
        :returns: Numpy array of optimized path data.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn optimize_path_from_array(
    py: Python<'_>,
    data: Option<Vec<Vec<f64>>>,
    tolerance: f64,
    fit_arcs: bool,
) -> Bound<'_, pyo3::types::PyAny> {
    let Some(data) = data else {
        return PyArray2::<f64>::zeros(py, [0, 0], false).as_any().clone();
    };
    if data.is_empty() {
        return PyArray2::<f64>::zeros(py, [0, 0], false).as_any().clone();
    }
    let arr = to_data_array(data);
    let result = raygeo_core::geo::algo::fitting::optimize_path_from_array(
        &arr, tolerance, fit_arcs,
    );
    let vecs: Vec<Vec<f64>> = result.into_iter().map(|r| r.to_vec()).collect();
    if vecs.is_empty() {
        return PyArray2::<f64>::zeros(py, [0, 0], false).as_any().clone();
    }
    let np_arr = PyArray2::<f64>::from_vec2(py, &vecs)
        .expect("failed to create numpy array");
    np_arr.as_any().clone()
}

#[gen_stub_pyfunction(
    python = r#"
    def fit_arcs(
        data: Optional[Sequence[Sequence[float]]],
        tolerance: float,
        progress_callback: Optional[Callable[[float], None]] = None,
    ) -> Optional[list[list[float]]]:
        """Fit arcs to a path.

        :param data: Array of command data or None.
        :param tolerance: Fitting tolerance.
        :param progress_callback: Optional progress callback.
        :returns: Fitted arc data or None.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
#[pyo3(signature = (data, tolerance, progress_callback=None))]
fn fit_arcs(
    data: Option<Vec<Vec<f64>>>,
    tolerance: f64,
    progress_callback: Option<Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<Vec<f64>>>> {
    match data {
        Some(rows) => {
            let arr = to_data_array(rows);
            let result = raygeo_core::fit_arcs(&arr, tolerance);
            if let Some(ref cb) = progress_callback {
                let _ = cb.call1((1.0f64,));
            }
            Ok(Some(result.iter().map(|r| r.to_vec()).collect()))
        }
        None => Ok(None),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    def check_self_intersection(
        data: Optional[Sequence[Sequence[float]]],
        fail_on_t_junction: bool,
    ) -> bool:
        """Check if a path has self-intersections.

        :param data: Array of command data or None.
        :param fail_on_t_junction: Whether T-junctions count as intersections.
        :returns: True if self-intersections are found.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn check_self_intersection(
    data: Option<Vec<Vec<f64>>>,
    fail_on_t_junction: bool,
) -> PyResult<bool> {
    match data {
        Some(rows) => {
            let arr = to_data_array(rows);
            Ok(check_self_intersection_from_array(&arr, fail_on_t_junction))
        }
        None => Ok(false),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    def check_intersection(
        data1: Optional[Sequence[Sequence[float]]],
        data2: Optional[Sequence[Sequence[float]]],
        fail_on_t_junction: bool,
    ) -> bool:
        """Check if two paths intersect.

        :param data1: First array of command data or None.
        :param data2: Second array of command data or None.
        :param fail_on_t_junction: Whether T-junctions count as intersections.
        :returns: True if intersections are found.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction]
fn check_intersection(
    data1: Option<Vec<Vec<f64>>>,
    data2: Option<Vec<Vec<f64>>>,
    fail_on_t_junction: bool,
) -> PyResult<bool> {
    match (data1, data2) {
        (Some(rows1), Some(rows2)) => {
            let arr1 = to_data_array(rows1);
            let arr2 = to_data_array(rows2);
            Ok(check_intersection_from_array(
                &arr1,
                &arr2,
                fail_on_t_junction,
            ))
        }
        _ => Ok(false),
    }
}

#[pyfunction]
fn _partial_segment_from_row(
    row: Vec<f64>,
    start_point: (f64, f64, f64),
    t: f64,
) -> Option<Vec<f64>> {
    let arr = to_data_array(vec![row]);
    partial_segment_from_row(&arr[0], start_point, t).map(|r| r.to_vec())
}

#[pyfunction]
fn _segment_length_from_row(
    row: Vec<f64>,
    start_point: (f64, f64, f64),
) -> f64 {
    let arr = to_data_array(vec![row]);
    segment_length_from_row_flat(&arr[0], start_point)
}

#[gen_stub_pyfunction(
    python = r#"
    def remove_duplicate_segments(
        data: Optional[Sequence[Sequence[float]]],
        tolerance: float = 1e-6,
    ) -> Any:
        """Remove duplicate segments from path data.

        :param data: Array of command data or None.
        :param tolerance: Comparison tolerance.
        :returns: Numpy array with duplicates removed.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "remove_duplicate_segments")]
#[pyo3(signature = (data, tolerance=1e-6))]
fn remove_duplicate_segments_py(
    py: Python<'_>,
    data: Option<Vec<Vec<f64>>>,
    tolerance: f64,
) -> Py<PyAny> {
    let Some(data) = data else {
        return py.None();
    };
    if data.is_empty() {
        return PyArray2::<f64>::zeros(py, [0, 0], false)
            .as_any()
            .clone()
            .unbind();
    }
    let arr = to_data_array(data);
    let result = remove_duplicate_segments(&arr, tolerance);
    let vecs: Vec<Vec<f64>> = result.into_iter().map(|r| r.to_vec()).collect();
    let np_arr = PyArray2::<f64>::from_vec2(py, &vecs)
        .expect("failed to create numpy array");
    np_arr.as_any().clone().unbind()
}

#[gen_stub_pyfunction(
    python = r#"
    def flatten_to_points(
        data: Optional[Sequence[Sequence[float]]],
        tolerance: float,
    ) -> list[list[Point3D]]:
        """Flatten curves into linear segments.

        :param data: Array of command data or None.
        :param tolerance: Flattening tolerance.
        :returns: List of flattened point segments.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "flatten_to_points")]
#[pyo3(signature = (data, tolerance))]
fn flatten_to_points_py(
    data: Option<Vec<Vec<f64>>>,
    tolerance: f64,
) -> Vec<Vec<(f64, f64, f64)>> {
    match data {
        Some(rows) => {
            let arr = to_data_array(rows);
            flatten_to_points(&arr, tolerance)
        }
        None => Vec::new(),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    def linearize_geometry(
        data: Optional[Sequence[Sequence[float]]],
        tolerance: float,
    ) -> Any:
        """Linearize geometry data into line segments.

        :param data: Array of command data or None.
        :param tolerance: Linearization tolerance.
        :returns: Numpy array of linearized segments.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "linearize_geometry")]
#[pyo3(signature = (data, tolerance))]
fn linearize_geometry_py(
    py: Python<'_>,
    data: Option<Vec<Vec<f64>>>,
    tolerance: f64,
) -> Bound<'_, pyo3::types::PyAny> {
    let Some(data) = data else {
        return PyArray2::<f64>::zeros(py, [0, 8], false).as_any().clone();
    };
    if data.is_empty() {
        return PyArray2::<f64>::zeros(py, [0, 8], false).as_any().clone();
    }
    let arr = to_data_array(data);
    let result = linearize_geometry(&arr, tolerance);
    let vecs: Vec<Vec<f64>> = result.into_iter().map(|r| r.to_vec()).collect();
    let np_arr = PyArray2::<f64>::from_vec2(py, &vecs)
        .expect("failed to create numpy array");
    np_arr.as_any().clone()
}

#[gen_stub_pyfunction(
    python = r#"
    def create_line_cmd(end_point: Point2DOr3D) -> list[float]:
        """Create a line command array from an end point.

        :param end_point: End point (x, y) or (x, y, z).
        :returns: Line command array (8 floats).
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "create_line_cmd")]
fn create_line_cmd_py(end_point: PyPoint3D) -> Vec<f64> {
    create_line_cmd((end_point.0, end_point.1, end_point.2)).to_vec()
}

#[gen_stub_pyfunction(
    python = r#"
    def create_arc_cmd(
        end: Point3D,
        center: Point,
        start: Point3D,
    ) -> list[float]:
        """Create an arc command array.

        :param end: End point (x, y, z).
        :param center: Center offset (dx, dy).
        :param start: Start point (x, y, z).
        :returns: Arc command array (8 floats).
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "create_arc_cmd")]
fn create_arc_cmd_py(
    end: (f64, f64, f64),
    center: (f64, f64),
    start: (f64, f64, f64),
) -> Vec<f64> {
    create_arc_cmd(end, center, start).to_vec()
}

#[gen_stub_pyfunction(
    python = r#"
    def convert_arc_to_beziers_from_array(
        start: Point3D,
        end: Point3D,
        center_offset: Point,
        clockwise: bool,
    ) -> list[list[float]]:
        """Convert an arc to bezier curves.

        :param start: Start point (x, y, z).
        :param end: End point (x, y, z).
        :param center_offset: Center offset (dx, dy).
        :param clockwise: Whether the arc is clockwise.
        :returns: List of bezier command rows.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "convert_arc_to_beziers_from_array")]
fn convert_arc_to_beziers_from_array_py(
    start: (f64, f64, f64),
    end: (f64, f64, f64),
    center_offset: (f64, f64),
    clockwise: bool,
) -> Vec<Vec<f64>> {
    convert_arc_to_beziers_from_array(start, end, center_offset, clockwise)
        .into_iter()
        .map(|r| r.to_vec())
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    def fit_curves(
        data: Optional[Sequence[Sequence[float]]],
        tolerance: float,
        preserve_beziers: bool,
        preserve_arcs: bool,
    ) -> Any:
        """Fit curves (lines, arcs, beziers) to path data.

        :param data: Array of command data or None.
        :param tolerance: Fitting tolerance.
        :param preserve_beziers: Whether to preserve existing beziers.
        :param preserve_arcs: Whether to preserve existing arcs.
        :returns: Numpy array of fitted curve data.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "fit_curves")]
fn fit_curves_py(
    py: Python<'_>,
    data: Option<Vec<Vec<f64>>>,
    tolerance: f64,
    preserve_beziers: bool,
    preserve_arcs: bool,
) -> Bound<'_, pyo3::types::PyAny> {
    let Some(data) = data else {
        return PyArray2::<f64>::zeros(py, [0, 0], false).as_any().clone();
    };
    if data.is_empty() {
        return PyArray2::<f64>::zeros(py, [0, 0], false).as_any().clone();
    }
    let arr = to_data_array(data);
    let result = fit_curves(&arr, tolerance, preserve_beziers, preserve_arcs);
    let vecs: Vec<Vec<f64>> = result.into_iter().map(|r| r.to_vec()).collect();
    let np_arr = PyArray2::<f64>::from_vec2(py, &vecs)
        .expect("failed to create numpy array");
    np_arr.as_any().clone()
}

#[gen_stub_pyfunction(
    python = r#"
    def get_segment_key(
        data: Sequence[Sequence[float]],
        index: int,
        tolerance: float,
    ) -> Optional[Any]:
        """Get a segment key for comparison.

        :param data: Array of command data.
        :param index: Row index.
        :param tolerance: Tolerance for comparison.
        :returns: Tuple key or None.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "get_segment_key")]
fn get_segment_key_py(
    py: Python<'_>,
    data: Vec<Vec<f64>>,
    index: usize,
    _tolerance: f64,
) -> Option<Py<PyAny>> {
    let row = data.get(index)?;
    let arr = to_data_array(vec![row.clone()]);
    let internal = get_segment_key(&arr[0])?;
    let cmd_type = internal.0;
    let end = (internal.1[0], internal.1[1], internal.1[2]);
    let params = internal.2;
    let result: Py<PyAny> = if cmd_type == CMD_TYPE_LINE as u32 {
        let a: [Bound<'_, PyAny>; 2] = [
            "LINE".into_pyobject(py).unwrap().into_any(),
            end.into_pyobject(py).unwrap().into_any(),
        ];
        PyTuple::new(py, a).unwrap().into_any().unbind()
    } else if cmd_type == CMD_TYPE_ARC as u32 {
        let a: [Bound<'_, PyAny>; 4] = [
            "ARC".into_pyobject(py).unwrap().into_any(),
            end.into_pyobject(py).unwrap().into_any(),
            (params[0], params[1]).into_pyobject(py).unwrap().into_any(),
            pyo3::types::PyBool::new(py, params[2] > 0.5)
                .as_any()
                .clone(),
        ];
        PyTuple::new(py, a).unwrap().into_any().unbind()
    } else if cmd_type == CMD_TYPE_BEZIER as u32 {
        let a: [Bound<'_, PyAny>; 4] = [
            "BEZIER".into_pyobject(py).unwrap().into_any(),
            end.into_pyobject(py).unwrap().into_any(),
            (params[0], params[1]).into_pyobject(py).unwrap().into_any(),
            (params[2], params[3]).into_pyobject(py).unwrap().into_any(),
        ];
        PyTuple::new(py, a).unwrap().into_any().unbind()
    } else {
        return None;
    };
    Some(result)
}

fn _extract_point3(key: &Bound<'_, PyAny>, idx: usize) -> PyResult<[f64; 3]> {
    let p: PyPoint3D = key.get_item(idx)?.extract()?;
    Ok([p.0, p.1, p.2])
}

fn _extract_point2(key: &Bound<'_, PyAny>, idx: usize) -> PyResult<(f64, f64)> {
    let p: (f64, f64) = key.get_item(idx)?.extract()?;
    Ok(p)
}

#[gen_stub_pyfunction(
    python = r#"
    def are_segments_equal(
        key1: Any,
        key2: Any,
        tolerance: float,
    ) -> bool:
        """Check if two segment keys are equal within tolerance.

        :param key1: First segment key tuple.
        :param key2: Second segment key tuple.
        :param tolerance: Maximum allowed difference.
        :returns: True if segment keys are equal within tolerance.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "are_segments_equal")]
fn are_segments_equal_py(
    key1: &Bound<'_, PyAny>,
    key2: &Bound<'_, PyAny>,
    tolerance: f64,
) -> PyResult<bool> {
    let type1: String = key1.get_item(0)?.extract()?;
    let type2: String = key2.get_item(0)?.extract()?;
    if type1 != type2 {
        return Ok(false);
    }
    match type1.as_str() {
        "LINE" => {
            let a1 = _extract_point3(key1, 1)?;
            let a2 = _extract_point3(key2, 1)?;
            let b1 = _extract_point3(key1, 2)?;
            let b2 = _extract_point3(key2, 2)?;
            Ok(are_points_equal(&a1, &a2, tolerance)
                && are_points_equal(&b1, &b2, tolerance))
        }
        "ARC" => {
            let key1_len = key1.len()?;
            let key2_len = key2.len()?;
            if key1_len == 4 && key2_len == 4 {
                let p1a = _extract_point3(key1, 1)?;
                let p1b = _extract_point3(key2, 1)?;
                let ca = _extract_point2(key1, 2)?;
                let cb = _extract_point2(key2, 2)?;
                let cwa: bool = key1.get_item(3)?.extract()?;
                let cwb: bool = key2.get_item(3)?.extract()?;
                Ok(are_points_equal(&p1a, &p1b, tolerance)
                    && (ca.0 - cb.0).abs() < tolerance
                    && (ca.1 - cb.1).abs() < tolerance
                    && cwa == cwb)
            } else {
                let a1 = _extract_point3(key1, 1)?;
                let a2 = _extract_point3(key2, 1)?;
                let b1 = _extract_point3(key1, 2)?;
                let b2 = _extract_point3(key2, 2)?;
                let ca = _extract_point2(key1, 3)?;
                let cb = _extract_point2(key2, 3)?;
                let cwa: bool = key1.get_item(4)?.extract()?;
                let cwb: bool = key2.get_item(4)?.extract()?;
                Ok(are_points_equal(&a1, &a2, tolerance)
                    && are_points_equal(&b1, &b2, tolerance)
                    && (ca.0 - cb.0).abs() < tolerance
                    && (ca.1 - cb.1).abs() < tolerance
                    && cwa == cwb)
            }
        }
        "BEZIER" => {
            let p1a = _extract_point3(key1, 1)?;
            let p1b = _extract_point3(key2, 1)?;
            let c1a: (f64, f64) = key1.get_item(2)?.extract()?;
            let c1b: (f64, f64) = key2.get_item(2)?.extract()?;
            let c2a: (f64, f64) = key1.get_item(3)?.extract()?;
            let c2b: (f64, f64) = key2.get_item(3)?.extract()?;
            Ok(are_points_equal(&p1a, &p1b, tolerance)
                && (c1a.0 - c1b.0).abs() < tolerance
                && (c1a.1 - c1b.1).abs() < tolerance
                && (c2a.0 - c2b.0).abs() < tolerance
                && (c2a.1 - c2b.1).abs() < tolerance)
        }
        _ => Ok(false),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    def is_closed(
        commands: Sequence[Sequence[float]],
        tolerance: float = 1e-6,
    ) -> bool:
        """Check if a path is closed.

        :param commands: Array of command data.
        :param tolerance: Tolerance for end-to-start distance check.
        :returns: True if the path is closed.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "is_closed")]
#[pyo3(signature = (commands, tolerance=1e-6))]
fn is_closed_py(commands: Vec<Vec<f64>>, tolerance: f64) -> bool {
    let arr = to_data_array(commands);
    raygeo_core::geo::analysis::is_closed(&arr, tolerance)
}

#[gen_stub_pyfunction(
    python = r#"
    def remove_duplicates(points: Sequence[Point]) -> Polygon:
        """Remove duplicate points from a sequence.

        :param points: Sequence of (x, y) points.
        :returns: List of unique points.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "remove_duplicates")]
fn remove_duplicates_py(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    raygeo_core::geo::analysis::remove_duplicates(&points)
}

#[gen_stub_pyfunction(
    python = r#"
    def get_outward_normal_at_from_array(
        data: Sequence[Sequence[float]],
        row_index: int,
        t: float,
    ) -> Optional[Point]:
        """Get the outward normal at a point on a segment.

        :param data: Array of command data.
        :param row_index: Row index of the segment.
        :param t: Parameter value along the segment (0..1).
        :returns: Outward normal vector (nx, ny) or None.
        """
"#,
    module = "raygeo.geo.path"
)]
#[pyfunction(name = "get_outward_normal_at_from_array")]
fn get_outward_normal_at_from_array_py(
    data: Vec<Vec<f64>>,
    row_index: usize,
    t: f64,
) -> Option<(f64, f64)> {
    let arr = to_data_array(data);
    raygeo_core::geo::analysis::get_outward_normal_at_from_array(
        &arr, row_index, t,
    )
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let path_mod = PyModule::new(py, "path")?;

    path_mod.add_class::<Geometry>()?;
    path_mod.add_class::<PyCommand>()?;

    path_mod.add_function(wrap_pyfunction!(
        remove_duplicate_segments_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        flatten_to_points_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        linearize_geometry_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        create_line_cmd_py,
        path_mod.clone()
    )?)?;
    path_mod
        .add_function(wrap_pyfunction!(create_arc_cmd_py, path_mod.clone())?)?;
    path_mod.add_function(wrap_pyfunction!(
        convert_arc_to_beziers_from_array_py,
        path_mod.clone()
    )?)?;
    path_mod
        .add_function(wrap_pyfunction!(fit_curves_py, path_mod.clone())?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_segment_key_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        are_segments_equal_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_bounding_rect_from_array,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_total_distance_from_array,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        extract_overcut_rows,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_subpath_vertices_from_array_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_subpath_area_from_array_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_area_from_array,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_path_winding_order_from_array,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_point_and_tangent_at_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        optimize_path_from_array,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(fit_arcs, path_mod.clone())?)?;
    path_mod.add_function(wrap_pyfunction!(
        check_self_intersection,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        check_intersection,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        _partial_segment_from_row,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        _segment_length_from_row,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(
        remove_duplicates_py,
        path_mod.clone()
    )?)?;
    path_mod.add_function(wrap_pyfunction!(is_closed_py, path_mod.clone())?)?;
    path_mod.add_function(wrap_pyfunction!(
        get_outward_normal_at_from_array_py,
        path_mod.clone()
    )?)?;

    m.add_submodule(&path_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.path", &path_mod)?;
    sys_modules.set_item("raygeo.path", &path_mod)?;

    Ok(())
}
