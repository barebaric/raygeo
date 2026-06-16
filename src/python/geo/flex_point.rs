use numpy::{PyArray2, PyArrayMethods};
use pyo3::prelude::*;
use pyo3_stub_gen::{PyStubType, TypeInfo};

use crate::{Point, Point3D};

use super::types::{Edge2D, Edge3D};

/// A 2D point that accepts tuples `(x, y)`, `(x, y, z)`,
/// or lists `[x, y]`, `[x, y, z]`, discarding the z coordinate.
#[derive(Clone, Copy, Debug)]
pub struct PyPoint2D(pub f64, pub f64);

impl<'a, 'py> FromPyObject<'a, 'py> for PyPoint2D {
    type Error = PyErr;
    fn extract(
        ob: pyo3::Borrowed<'a, 'py, pyo3::types::PyAny>,
    ) -> Result<Self, Self::Error> {
        if let Ok((x, y)) = ob.extract::<(f64, f64)>() {
            return Ok(PyPoint2D(x, y));
        }
        if let Ok((x, y, _)) = ob.extract::<(f64, f64, f64)>() {
            return Ok(PyPoint2D(x, y));
        }
        let iter = ob.try_iter()?;
        let items: Vec<f64> = iter
            .take(3)
            .map(|i| i?.extract::<f64>())
            .collect::<Result<Vec<_>, _>>()?;
        if items.len() >= 2 {
            return Ok(PyPoint2D(items[0], items[1]));
        }
        Err(pyo3::exceptions::PyValueError::new_err(
            "expected a sequence of 2 or 3 floats",
        ))
    }
}

impl From<PyPoint2D> for (f64, f64) {
    fn from(p: PyPoint2D) -> Self {
        (p.0, p.1)
    }
}

impl From<&PyPoint2D> for (f64, f64) {
    fn from(p: &PyPoint2D) -> Self {
        (p.0, p.1)
    }
}

pub fn poly_to_points(poly: Vec<PyPoint2D>) -> Vec<Point> {
    poly.into_iter().map(|p| Point::new(p.0, p.1)).collect()
}

// --- Conversion helpers for Python boundary (Point ↔ (f64, f64) tuples) ---

pub fn point_to_tuple(p: Point) -> (f64, f64) {
    (p.x, p.y)
}

pub fn points_to_tuples(v: Vec<Point>) -> Vec<(f64, f64)> {
    v.into_iter().map(|p| (p.x, p.y)).collect()
}

pub fn polygons_to_tuples(v: Vec<Vec<Point>>) -> Vec<Vec<(f64, f64)>> {
    v.into_iter().map(points_to_tuples).collect()
}

pub fn edge_pairs_to_tuples(v: Vec<(Point, Point)>) -> Vec<Edge2D> {
    v.into_iter()
        .map(|(a, b)| ((a.x, a.y), (b.x, b.y)))
        .collect()
}

pub fn option_point_to_tuple(p: Option<Point>) -> Option<(f64, f64)> {
    p.map(|p| (p.x, p.y))
}

pub fn tuples_to_points(v: Vec<(f64, f64)>) -> Vec<Point> {
    v.into_iter().map(|(x, y)| Point::new(x, y)).collect()
}

pub fn polygons_from_tuples(v: Vec<Vec<(f64, f64)>>) -> Vec<Vec<Point>> {
    v.into_iter().map(tuples_to_points).collect()
}

/// A 3D point that accepts both 2-tuple `(x, y)` (z defaults to 0.0)
/// and 3-tuple `(x, y, z)`.
#[derive(Clone, Copy, Debug)]
pub struct PyPoint3D(pub f64, pub f64, pub f64);

impl<'a, 'py> FromPyObject<'a, 'py> for PyPoint3D {
    type Error = PyErr;
    fn extract(
        ob: pyo3::Borrowed<'a, 'py, pyo3::types::PyAny>,
    ) -> Result<Self, Self::Error> {
        if let Ok((x, y, z)) = ob.extract::<(f64, f64, f64)>() {
            return Ok(PyPoint3D(x, y, z));
        }
        if let Ok((x, y)) = ob.extract::<(f64, f64)>() {
            return Ok(PyPoint3D(x, y, 0.0));
        }
        let iter = ob.try_iter()?;
        let items: Vec<f64> = iter
            .take(3)
            .map(|i| i?.extract::<f64>())
            .collect::<Result<Vec<_>, _>>()?;
        match items.len() {
            3 => Ok(PyPoint3D(items[0], items[1], items[2])),
            2 => Ok(PyPoint3D(items[0], items[1], 0.0)),
            _ => Err(pyo3::exceptions::PyValueError::new_err(
                "expected a sequence of 2 or 3 floats",
            )),
        }
    }
}

impl From<PyPoint3D> for (f64, f64, f64) {
    fn from(p: PyPoint3D) -> Self {
        (p.0, p.1, p.2)
    }
}

/// Extract a single polygon (list of 2D points) from a Python object.
/// Accepts either a list of (x, y) tuples or an (N, 2) numpy array.
/// Returns `Vec<Point>` directly, avoiding `PyPoint2D` intermediate.
pub fn extract_polygon(ob: &Bound<'_, PyAny>) -> PyResult<Vec<Point>> {
    if let Ok(arr) = ob.extract::<Bound<'_, PyArray2<f64>>>() {
        return Ok(polygon_from_numpy(&arr));
    }
    let mut points = Vec::new();
    for item in ob.try_iter()? {
        let item = item?;
        if let Ok(p) = item.extract::<PyPoint2D>() {
            points.push(Point::new(p.0, p.1));
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "polygon elements must be (x, y) tuples or numpy array",
            ));
        }
    }
    Ok(points)
}

/// Extract a list of polygons from a Python object.
/// Accepts either a list of lists of (x, y) tuples or a list of (N, 2)
/// numpy arrays.
pub fn extract_polygons(ob: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<Point>>> {
    let mut result = Vec::new();
    for item in ob.try_iter()? {
        let item = item?;
        result.push(extract_polygon(&item)?);
    }
    Ok(result)
}

impl PyStubType for PyPoint2D {
    fn type_output() -> TypeInfo {
        TypeInfo::with_module("tuple[float, float]", "builtins".into())
    }
}

// --- 3D conversion helpers (Point3D ↔ (f64, f64, f64) tuples) ---

pub fn point3d_to_tuple(p: Point3D) -> (f64, f64, f64) {
    (p.x, p.y, p.z)
}

pub fn points3d_to_tuples(v: Vec<Point3D>) -> Vec<(f64, f64, f64)> {
    v.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}

pub fn tuple_to_point3d(p: (f64, f64, f64)) -> Point3D {
    Point3D::new(p.0, p.1, p.2)
}

pub fn edge_pairs3d_to_tuples(v: Vec<(Point3D, Point3D)>) -> Vec<Edge3D> {
    v.into_iter()
        .map(|(a, b)| ((a.x, a.y, a.z), (b.x, b.y, b.z)))
        .collect()
}

impl PyStubType for PyPoint3D {
    fn type_output() -> TypeInfo {
        TypeInfo::with_module("tuple[float, float, float]", "builtins".into())
    }
}

/// Zero-copy-friendly extraction of a polygon from a numpy (N, 2) array.
fn polygon_from_numpy(arr: &Bound<'_, PyArray2<f64>>) -> Vec<Point> {
    let readonly = arr.readonly();
    let view = readonly.as_array();
    let nrows = view.nrows();
    let mut points = Vec::with_capacity(nrows);
    for i in 0..nrows {
        points.push(Point::new(view[[i, 0]], view[[i, 1]]));
    }
    points
}
