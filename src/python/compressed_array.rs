use numpy::{IntoPyArray, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::compressed_array::{CompressedArray, Dtype};

/// A numpy-compatible array stored in compressed (zstd) form.
///
/// Created by raygeo's scene compiler and texture rasterizer to keep
/// large numpy buffers compressed in memory until they are needed
/// for GPU upload.  Call :meth:`to_numpy` to decompress on demand.
///
/// Arrays smaller than 4 KB are stored uncompressed to avoid overhead.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.compressed_array", name = "CompressedArray")]
pub struct PyCompressedArray {
    inner: CompressedArray,
}

impl PyCompressedArray {
    pub fn from_vec_f32(data: Vec<f32>) -> Self {
        Self {
            inner: CompressedArray::from_vec_f32(data),
        }
    }

    pub fn from_vec_i32(data: Vec<i32>) -> Self {
        Self {
            inner: CompressedArray::from_vec_i32(data),
        }
    }

    pub fn from_vec_u8(data: Vec<u8>, shape: Vec<usize>) -> Self {
        Self {
            inner: CompressedArray::from_vec_u8(data, shape),
        }
    }

    fn reshape_if_needed<'py>(
        &self,
        array: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if self.inner.shape.len() <= 1 {
            return Ok(array);
        }
        let shape: Vec<isize> =
            self.inner.shape.iter().map(|&v| v as isize).collect();
        array.call_method1("reshape", (shape.as_slice(),))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCompressedArray {
    /// Create a CompressedArray from a 1-D float32 numpy array.
    #[staticmethod]
    fn from_float32(data: PyReadonlyArray1<f32>) -> Self {
        let slice = data.as_slice().unwrap_or_default();
        PyCompressedArray::from_vec_f32(slice.to_vec())
    }

    /// Create a CompressedArray from a 1-D int32 numpy array.
    #[staticmethod]
    fn from_int32(data: PyReadonlyArray1<i32>) -> Self {
        let slice = data.as_slice().unwrap_or_default();
        PyCompressedArray::from_vec_i32(slice.to_vec())
    }

    /// Create a CompressedArray from a 2-D uint8 numpy array.
    #[staticmethod]
    fn from_uint8_2d(data: PyReadonlyArray2<u8>) -> Self {
        let array = data.as_array();
        let (rows, cols) = (array.shape()[0], array.shape()[1]);
        let flat: Vec<u8> = array.iter().copied().collect();
        PyCompressedArray::from_vec_u8(flat, vec![rows, cols])
    }

    /// Decompress and return a numpy array with the original dtype
    /// and shape.
    fn to_numpy<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        match self.inner.dtype {
            Dtype::Float32 => {
                let data = self
                    .inner
                    .decompress_to_vec_f32()
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                let array = data.into_pyarray(py);
                self.reshape_if_needed(array.into_any())
            }
            Dtype::Int32 => {
                let data = self
                    .inner
                    .decompress_to_vec_i32()
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                let array = data.into_pyarray(py);
                self.reshape_if_needed(array.into_any())
            }
            Dtype::UInt8 => {
                let data = self
                    .inner
                    .decompress_to_vec_u8()
                    .map_err(pyo3::exceptions::PyValueError::new_err)?;
                let array = data.into_pyarray(py);
                self.reshape_if_needed(array.into_any())
            }
        }
    }

    /// Size of the compressed (or raw) payload in bytes.
    #[getter]
    fn compressed_size(&self) -> usize {
        self.inner.data.len()
    }

    /// Size of the original uncompressed data in bytes.
    #[getter]
    fn uncompressed_size(&self) -> usize {
        self.inner.uncompressed_size
    }

    /// Compression ratio (compressed / uncompressed).
    #[getter]
    fn ratio(&self) -> f64 {
        if self.inner.uncompressed_size == 0 {
            return 1.0;
        }
        self.inner.data.len() as f64 / self.inner.uncompressed_size as f64
    }
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "compressed_array")?;
    m.add_class::<PyCompressedArray>()?;
    parent.add_submodule(&m)?;

    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.compressed_array", &m)?;

    Ok(())
}
