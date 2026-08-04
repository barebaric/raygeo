use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyTuple, PyType};
use pyo3_stub_gen::derive::{
    gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods,
};
use pyo3_stub_gen::inventory::submit;

use crate::geo::matrix::Matrix as CoreMatrix;

fn parse_matrix_data(data: &Bound<'_, PyAny>) -> PyResult<CoreMatrix> {
    // Try extracting as another Matrix (copy constructor)
    if let Ok(other) = data.extract::<Matrix>() {
        return Ok(other.inner);
    }

    // Try 3x3 list of lists
    if let Ok(rows) = data.extract::<Vec<Vec<f64>>>() {
        if rows.len() == 4 && rows[0].len() == 4 {
            // 4x4 matrix: extract 2D affine part
            return Ok(CoreMatrix::from_cols_arrays([
                [rows[0][0], rows[0][1], rows[0][3]],
                [rows[1][0], rows[1][1], rows[1][3]],
                [0.0, 0.0, 1.0],
            ]));
        }
        if rows.len() != 3 {
            return Err(PyValueError::new_err(
                "Matrix data must be a 3x3 or 4x4 sequence",
            ));
        }
        for row in &rows {
            if row.len() != 3 {
                return Err(PyValueError::new_err(
                    "Matrix data must be a 3x3 sequence",
                ));
            }
        }
        return Ok(CoreMatrix::from_cols_arrays([
            [rows[0][0], rows[0][1], rows[0][2]],
            [rows[1][0], rows[1][1], rows[1][2]],
            [rows[2][0], rows[2][1], rows[2][2]],
        ]));
    }

    Err(PyValueError::new_err(
        "Matrix data must be a Matrix, or a 3x3 or 4x4 sequence",
    ))
}

/// A 3x3 affine transformation matrix for 2D graphics.
///
/// Provides an object-oriented interface for matrix operations, including
/// translations, rotations, and scaling.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.geo", name = "Matrix", from_py_object)]
#[derive(Clone, Debug)]
pub struct Matrix {
    pub(crate) inner: CoreMatrix,
}

submit! {
    gen_methods_from_python! {
        r#"
        import numpy
        import numpy.typing
        from raygeo.geo import types

        class Matrix:
            def __matmul__(self, other: Matrix) -> Matrix:
                """Multiply two matrices (self @ other)."""
                ...
            def __eq__(self, other: object) -> bool:
                """Check equality with another Matrix."""
                ...
            def transform_point(self, *args: float | tuple[float, float]) -> tuple[float, float]:
                """Apply the affine transformation to a 2D point."""
                ...
            def transform_vector(self, *args: float | tuple[float, float]) -> tuple[float, float]:
                """Apply the transformation to a 2D vector, ignoring translation."""
                ...
            def transform_rectangle(self, *args: float | tuple[float, float, float, float]) -> tuple[float, float, float, float]:
                """Transform a rectangle and return the new axis-aligned bounding box."""
                ...
            def to_numpy(self) -> numpy.typing.NDArray[numpy.float64]:
                """Return the matrix as a 3x3 numpy array."""
                ...
            def to_4x4_numpy(self) -> numpy.typing.NDArray[numpy.float64]:
                """Return the matrix as a 4x4 numpy array (affine, Z-preserving)."""
                ...
            def get(self, row: int, col: int) -> float:
                """Get a single element from the 3x3 matrix (row-major indexing)."""
                ...
            def __copy__(self) -> Matrix:
                """Return a copy of this matrix."""
                ...
            def __deepcopy__(self, memo: dict) -> Matrix:
                """Return a deep copy of this matrix."""
                ...
        "#
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Matrix {
    /// Create a new Matrix.
    ///
    /// :param data: Optional 3x3 or 4x4 sequence (list of lists) to initialize
    ///     from. If None, creates an identity matrix.
    #[new]
    #[pyo3(signature = (data=None))]
    fn new(data: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        match data {
            Some(d) => {
                let inner = parse_matrix_data(d)?;
                Ok(Matrix { inner })
            }
            None => Ok(Matrix {
                inner: CoreMatrix::identity(),
            }),
        }
    }

    /// Create an identity matrix.
    #[classmethod]
    fn identity(_cls: &Bound<'_, PyType>) -> Self {
        Matrix {
            inner: CoreMatrix::identity(),
        }
    }

    /// Create a translation matrix.
    ///
    /// :param tx: Translation in x.
    /// :param ty: Translation in y.
    #[classmethod]
    fn translation(_cls: &Bound<'_, PyType>, tx: f64, ty: f64) -> Self {
        Matrix {
            inner: CoreMatrix::from_translation(tx, ty),
        }
    }

    /// Create a scaling matrix.
    ///
    /// :param sx: Scale factor for x-axis.
    /// :param sy: Scale factor for y-axis.
    /// :param center: Optional (x, y) center point to scale around.
    #[classmethod]
    #[pyo3(signature = (sx, sy, center=None))]
    fn scale(
        _cls: &Bound<'_, PyType>,
        sx: f64,
        sy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let m = CoreMatrix::from_scale(sx, sy);
        Matrix {
            inner: match center {
                Some((cx, cy)) => {
                    let t1 = CoreMatrix::from_translation(-cx, -cy);
                    let t2 = CoreMatrix::from_translation(cx, cy);
                    t2 * m * t1
                }
                None => m,
            },
        }
    }

    /// Create a rotation matrix.
    ///
    /// :param angle_deg: Rotation angle in degrees.
    /// :param center: Optional (x, y) center point to rotate around.
    #[classmethod]
    #[pyo3(signature = (angle_deg, center=None))]
    fn rotation(
        _cls: &Bound<'_, PyType>,
        angle_deg: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let m = CoreMatrix::from_rotation(angle_deg);
        Matrix {
            inner: match center {
                Some((cx, cy)) => {
                    let t1 = CoreMatrix::from_translation(-cx, -cy);
                    let t2 = CoreMatrix::from_translation(cx, cy);
                    t2 * m * t1
                }
                None => m,
            },
        }
    }

    /// Create a shearing matrix.
    ///
    /// :param shx: Shear factor for x-axis.
    /// :param shy: Shear factor for y-axis.
    /// :param center: Optional (x, y) center point to shear around.
    #[classmethod]
    #[pyo3(signature = (shx, shy, center=None))]
    fn shear(
        _cls: &Bound<'_, PyType>,
        shx: f64,
        shy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        let m = CoreMatrix::from_shear(shx, shy);
        Matrix {
            inner: match center {
                Some((cx, cy)) => {
                    let t1 = CoreMatrix::from_translation(-cx, -cy);
                    let t2 = CoreMatrix::from_translation(cx, cy);
                    t2 * m * t1
                }
                None => m,
            },
        }
    }

    /// Compose a matrix from translation, rotation, scale, and skew.
    ///
    /// :param tx: Translation x.
    /// :param ty: Translation y.
    /// :param angle_deg: Rotation angle in degrees.
    /// :param sx: Scale x.
    /// :param sy: Scale y.
    /// :param skew_angle_deg: Skew angle in degrees.
    /// :returns: A new Matrix.
    #[classmethod]
    fn compose(
        _cls: &Bound<'_, PyType>,
        tx: f64,
        ty: f64,
        angle_deg: f64,
        sx: f64,
        sy: f64,
        skew_angle_deg: f64,
    ) -> Self {
        Matrix {
            inner: CoreMatrix::from_compose(
                tx,
                ty,
                angle_deg,
                sx,
                sy,
                skew_angle_deg,
            ),
        }
    }

    /// Create a horizontal flip matrix.
    #[classmethod]
    #[pyo3(signature = (center=None))]
    fn flip_horizontal(
        _cls: &Bound<'_, PyType>,
        center: Option<(f64, f64)>,
    ) -> Self {
        Matrix {
            inner: CoreMatrix::flip_horizontal(center),
        }
    }

    /// Create a vertical flip matrix.
    #[classmethod]
    #[pyo3(signature = (center=None))]
    fn flip_vertical(
        _cls: &Bound<'_, PyType>,
        center: Option<(f64, f64)>,
    ) -> Self {
        Matrix {
            inner: CoreMatrix::flip_vertical(center),
        }
    }

    /// Return a deep copy of this matrix.
    fn copy(&self) -> Self {
        self.clone()
    }

    /// Multiply two matrices (self @ other).
    #[gen_stub(skip)]
    fn __matmul__(&self, other: &Self) -> Self {
        Matrix {
            inner: self.inner * other.inner,
        }
    }

    /// Check equality with another Matrix.
    #[gen_stub(skip)]
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(other_m) = other.extract::<Matrix>() {
            self.inner == other_m.inner
        } else {
            false
        }
    }

    #[gen_stub(skip)]
    fn __reduce_ex__<'py>(
        slf: &Bound<'py, Self>,
        _protocol: i32,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, pyo3::types::PyTuple>> {
        let from_list = slf.get_type().getattr("from_list")?;
        let data = slf.borrow().to_list();
        let data_tuple = pyo3::types::PyTuple::new(py, [data])?;
        pyo3::types::PyTuple::new(py, [from_list, data_tuple.as_any().clone()])
    }

    #[gen_stub(skip)]
    fn __copy__(&self) -> Self {
        self.clone()
    }

    #[gen_stub(skip)]
    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        self.clone()
    }

    /// String representation for developers.
    fn __repr__(&self) -> String {
        let m = self.inner.to_cols_array();
        format!(
            "Matrix([[{}, {}, {}], [{}, {}, {}], [0.0, 0.0, 1.0]])",
            m[0], m[1], m[2], m[3], m[4], m[5]
        )
    }

    /// Human-readable string representation.
    fn __str__(&self) -> String {
        let m = self.inner.to_cols_array();
        format!(
            "[[{:8.4} {:8.4} {:8.4}]\n [{:8.4} {:8.4} {:8.4}]\n [ 0.0000   0.0000   1.0000]]",
            m[0], m[1], m[2], m[3], m[4], m[5]
        )
    }

    /// Returns a copy of the matrix data as a list of lists.
    fn to_list(&self) -> Vec<Vec<f64>> {
        let m = self.inner.to_cols_array();
        vec![
            vec![m[0], m[1], m[2]],
            vec![m[3], m[4], m[5]],
            vec![0.0, 0.0, 1.0],
        ]
    }

    /// Converts the matrix to a 4x4 list of lists (row-major).
    /// The 3x3 Matrix is placed in the top-left 2x2 of the 4x4,
    /// with translation in the last column, and Z preserved.
    fn to_4x4_list(&self) -> Vec<Vec<f64>> {
        let m44 = self.inner.to_4x4();
        // DMat4 is column-major: x_axis=col0, y_axis=col1, z_axis=col2, w_axis=col3
        // Output in row-major format:
        vec![
            vec![
                m44.x_axis.x, // [0,0]
                m44.y_axis.x, // [0,1]
                m44.z_axis.x, // [0,2]
                m44.w_axis.x, // [0,3] = tx
            ],
            vec![
                m44.x_axis.y, // [1,0]
                m44.y_axis.y, // [1,1]
                m44.z_axis.y, // [1,2]
                m44.w_axis.y, // [1,3] = ty
            ],
            vec![
                m44.x_axis.z, // [2,0]
                m44.y_axis.z, // [2,1]
                m44.z_axis.z, // [2,2]
                m44.w_axis.z, // [2,3]
            ],
            vec![
                m44.x_axis.w, // [3,0]
                m44.y_axis.w, // [3,1]
                m44.z_axis.w, // [3,2]
                m44.w_axis.w, // [3,3]
            ],
        ]
    }

    /// Returns the matrix components in Cairo order (xx, yx, xy, yy, x0, y0).
    fn for_cairo(&self) -> (f64, f64, f64, f64, f64, f64) {
        self.inner.for_cairo()
    }

    /// Returns (xx, yx, xy, yy, x0, y0) for GTK/Graphene.
    fn to_graphene(&self) -> (f64, f64, f64, f64, f64, f64) {
        self.inner.for_cairo()
    }

    /// Extract the translation component (tx, ty).
    fn get_translation(&self) -> (f64, f64) {
        self.inner.translation()
    }

    /// Returns a new matrix with the same rotation/scale/shear but new
    /// translation.
    ///
    /// :param tx: New translation x.
    /// :param ty: New translation y.
    fn set_translation(&self, tx: f64, ty: f64) -> Self {
        Matrix {
            inner: self.inner.set_translation(tx, ty),
        }
    }

    /// Returns a new matrix with translation set to zero.
    fn without_translation(&self) -> Self {
        Matrix {
            inner: self.inner.without_translation(),
        }
    }

    /// Extract the signed scale components (sx, sy).
    fn get_scale(&self) -> (f64, f64) {
        self.inner.scale()
    }

    /// Extract the absolute scale components (|sx|, |sy|).
    fn get_abs_scale(&self) -> (f64, f64) {
        self.inner.abs_scale()
    }

    /// Extract the rotation angle in degrees.
    fn get_rotation(&self) -> f64 {
        self.inner.rotation()
    }

    /// Calculate the angle of the transformed X-axis in degrees.
    fn get_x_axis_angle(&self) -> f64 {
        self.inner.x_axis_angle()
    }

    /// Calculate the angle of the transformed Y-axis in degrees.
    fn get_y_axis_angle(&self) -> f64 {
        self.inner.y_axis_angle()
    }

    /// Calculate the determinant of the top-left 2x2 sub-matrix.
    fn get_determinant_2x2(&self) -> f64 {
        self.inner.determinant_2x2()
    }

    /// Check if the matrix is an identity matrix.
    fn is_identity(&self) -> bool {
        self.inner.is_identity()
    }

    /// Check if the matrix includes a reflection (flip).
    fn is_flipped(&self) -> bool {
        self.inner.is_flipped()
    }

    /// Check if the matrix has zero scale on any axis.
    ///
    /// :param tolerance: Threshold below which a scale is considered zero.
    #[pyo3(signature = (tolerance=1e-6))]
    fn has_zero_scale(&self, tolerance: f64) -> bool {
        self.inner.has_zero_scale(tolerance)
    }

    /// Check if two matrices are effectively equal within tolerance.
    ///
    /// :param other: The matrix to compare against.
    /// :param tol: The absolute tolerance parameter.
    #[pyo3(signature = (other, tol=1e-6))]
    fn is_close(&self, other: &Self, tol: f64) -> bool {
        self.inner.is_close(&other.inner, tol)
    }

    /// Apply a translation before this matrix's transformation.
    fn pre_translate(&self, tx: f64, ty: f64) -> Self {
        Matrix {
            inner: self.inner.translate_pre(tx, ty),
        }
    }

    /// Apply a translation after this matrix's transformation.
    fn post_translate(&self, tx: f64, ty: f64) -> Self {
        Matrix {
            inner: self.inner.translate_post(tx, ty),
        }
    }

    /// Apply a scale before this matrix's transformation.
    ///
    /// :param sx: Scale factor for x-axis.
    /// :param sy: Scale factor for y-axis.
    /// :param center: Optional center point to scale around.
    #[pyo3(signature = (sx, sy, center=None))]
    fn pre_scale(&self, sx: f64, sy: f64, center: Option<(f64, f64)>) -> Self {
        Matrix {
            inner: self.inner.scale_pre(sx, sy, center),
        }
    }

    /// Apply a scale after this matrix's transformation.
    ///
    /// :param sx: Scale factor for x-axis.
    /// :param sy: Scale factor for y-axis.
    /// :param center: Optional center point to scale around.
    #[pyo3(signature = (sx, sy, center=None))]
    fn post_scale(&self, sx: f64, sy: f64, center: Option<(f64, f64)>) -> Self {
        Matrix {
            inner: self.inner.scale_post(sx, sy, center),
        }
    }

    /// Apply a rotation before this matrix's transformation.
    ///
    /// :param angle_deg: Rotation angle in degrees.
    /// :param center: Optional center point to rotate around.
    #[pyo3(signature = (angle_deg, center=None))]
    fn pre_rotate(&self, angle_deg: f64, center: Option<(f64, f64)>) -> Self {
        Matrix {
            inner: self.inner.rotate_pre(angle_deg, center),
        }
    }

    /// Apply a rotation after this matrix's transformation.
    ///
    /// :param angle_deg: Rotation angle in degrees.
    /// :param center: Optional center point to rotate around.
    #[pyo3(signature = (angle_deg, center=None))]
    fn post_rotate(&self, angle_deg: f64, center: Option<(f64, f64)>) -> Self {
        Matrix {
            inner: self.inner.rotate_post(angle_deg, center),
        }
    }

    /// Apply a shear before this matrix's transformation.
    ///
    /// :param shx: Shear factor for x-axis.
    /// :param shy: Shear factor for y-axis.
    /// :param center: Optional center point to shear around.
    #[pyo3(signature = (shx, shy, center=None))]
    fn pre_shear(
        &self,
        shx: f64,
        shy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        Matrix {
            inner: self.inner.shear_pre(shx, shy, center),
        }
    }

    /// Apply a shear after this matrix's transformation.
    ///
    /// :param shx: Shear factor for x-axis.
    /// :param shy: Shear factor for y-axis.
    /// :param center: Optional center point to shear around.
    #[pyo3(signature = (shx, shy, center=None))]
    fn post_shear(
        &self,
        shx: f64,
        shy: f64,
        center: Option<(f64, f64)>,
    ) -> Self {
        Matrix {
            inner: self.inner.shear_post(shx, shy, center),
        }
    }

    /// Compute the inverse of the matrix.
    ///
    /// Will raise an error if the matrix is singular.
    fn invert(&self) -> PyResult<Self> {
        if self.inner.has_zero_scale(1e-12) {
            return Err(PyValueError::new_err(
                "Matrix is singular (zero scale) and cannot be inverted",
            ));
        }
        Ok(Matrix {
            inner: self.inner.invert(),
        })
    }

    /// Apply the affine transformation to a 2D point.
    ///
    /// Accepts either two floats ``(x, y)`` or a single ``(x, y)`` tuple.
    /// :returns: The transformed ``(x, y)`` tuple.
    #[gen_stub(skip)]
    #[pyo3(signature = (*args))]
    fn transform_point(
        &self,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<(f64, f64)> {
        let (x, y) = if args.len() == 1 {
            args.get_item(0)?.extract::<(f64, f64)>()?
        } else if args.len() == 2 {
            (
                args.get_item(0)?.extract::<f64>()?,
                args.get_item(1)?.extract::<f64>()?,
            )
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "expected (x, y) or ((x, y),)",
            ));
        };
        Ok(self.inner.transform_point(x, y))
    }

    /// Apply the transformation to a 2D vector, ignoring translation.
    ///
    /// Accepts either two floats ``(vx, vy)`` or a single ``(vx, vy)`` tuple.
    /// :returns: The transformed ``(vx, vy)`` tuple.
    #[gen_stub(skip)]
    #[pyo3(signature = (*args))]
    fn transform_vector(
        &self,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<(f64, f64)> {
        let (x, y) = if args.len() == 1 {
            args.get_item(0)?.extract::<(f64, f64)>()?
        } else if args.len() == 2 {
            (
                args.get_item(0)?.extract::<f64>()?,
                args.get_item(1)?.extract::<f64>()?,
            )
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "expected (vx, vy) or ((vx, vy),)",
            ));
        };
        Ok(self.inner.transform_vector(x, y))
    }

    /// Transform a rectangle and return the new axis-aligned bounding box.
    ///
    /// Accepts either four floats ``(x, y, w, h)`` or a single
    /// ``(x, y, w, h)`` tuple.
    /// :returns: ``(x, y, w, h)`` of the resulting bounding box.
    #[gen_stub(skip)]
    #[pyo3(signature = (*args))]
    fn transform_rectangle(
        &self,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<(f64, f64, f64, f64)> {
        let (x, y, w, h) = if args.len() == 1 {
            args.get_item(0)?.extract::<(f64, f64, f64, f64)>()?
        } else if args.len() == 4 {
            (
                args.get_item(0)?.extract::<f64>()?,
                args.get_item(1)?.extract::<f64>()?,
                args.get_item(2)?.extract::<f64>()?,
                args.get_item(3)?.extract::<f64>()?,
            )
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "expected (x, y, w, h) or ((x, y, w, h),)",
            ));
        };
        Ok(self.inner.transform_rectangle(x, y, w, h))
    }

    /// Return the matrix as a 3x3 numpy array.
    #[gen_stub(skip)]
    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let arr = self.inner.to_cols_array();
        let data = vec![
            vec![arr[0], arr[1], arr[2]],
            vec![arr[3], arr[4], arr[5]],
            vec![arr[6], arr[7], arr[8]],
        ];
        let numpy = py.import("numpy")?;
        numpy.call_method1("array", (data,))
    }

    /// Return the matrix as a 4x4 numpy array (affine, Z-preserving).
    #[gen_stub(skip)]
    fn to_4x4_numpy<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let m44 = self.inner.to_4x4();
        let data = vec![
            vec![m44.x_axis.x, m44.y_axis.x, m44.z_axis.x, m44.w_axis.x],
            vec![m44.x_axis.y, m44.y_axis.y, m44.z_axis.y, m44.w_axis.y],
            vec![m44.x_axis.z, m44.y_axis.z, m44.z_axis.z, m44.w_axis.z],
            vec![m44.x_axis.w, m44.y_axis.w, m44.z_axis.w, m44.w_axis.w],
        ];
        let numpy = py.import("numpy")?;
        numpy.call_method1("array", (data,))
    }

    /// Get a single element from the 3x3 matrix (row-major indexing).
    ///
    /// :param row: Row index (0, 1, or 2).
    /// :param col: Column index (0, 1, or 2).
    #[gen_stub(skip)]
    fn get(&self, row: usize, col: usize) -> f64 {
        self.inner.get(row, col)
    }

    /// Decompose the matrix into translation, rotation, scale, and skew.
    ///
    /// :returns: (tx, ty, angle_deg, sx, sy, skew_angle_deg).
    fn decompose(&self) -> (f64, f64, f64, f64, f64, f64) {
        self.inner.decompose()
    }

    /// Create a Matrix from a list of lists.
    #[classmethod]
    fn from_list(
        _cls: &Bound<'_, PyType>,
        data: Vec<Vec<f64>>,
    ) -> PyResult<Self> {
        let py_any = data.into_pyobject(_cls.py())?;
        Matrix::new(Some(&py_any))
    }
}
