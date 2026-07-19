//! Python bindings for [`WholeImageSource`].
//!
//! Python code constructs a [`PyWholeImageSource`] from a 2-D uint8
//! numpy array and either passes it to [`Part`](crate::python::ops::part::part::PyPart)
//! via `part.image_source = WholeImageSource(...)` or retrieves it
//! back via the `part.image_source` getter.
//!
//! The Rust [`ImageSource`](crate::ops::part::image_source::ImageSource)
//! trait stays Rust-internal for now: Python code interacts only with
//! the concrete `WholeImageSource` class. Future slices will widen
//! this to a Python-side `Protocol` so any duck-typed object with the
//! right methods can be passed.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::image::types::PixelImage;
use crate::ops::part::image_source::{ImageSource, WholeImageSource};

/// Extract a flat u8 buffer from a numpy array, returning (data, height, width).
fn extract_flat_u8(
    py: Python<'_>,
    obj: &Bound<'_, PyAny>,
) -> PyResult<(Vec<u8>, usize, usize)> {
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (obj,))?;
    let shape: (usize, usize) = arr.getattr("shape")?.extract()?;
    let flat: Vec<u8> = arr
        .call_method("astype", ("uint8",), None)?
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    Ok((flat, shape.0, shape.1))
}

/// In-memory `ImageSource` wrapping a 2-D uint8 raster buffer.
///
/// Constructed from a numpy array and read lazily by assemblers via
/// the Rust-side `ImageSource` trait. May be attached to a
/// `Part` via ``part.image_source = WholeImageSource(array)``; the
/// ``part.image`` property is a convenience shim that constructs a
/// `WholeImageSource` on assignment.
#[gen_stub_pyclass(module = "raygeo.ops.part.image_source")]
#[pyclass(
    name = "WholeImageSource",
    module = "raygeo.ops.part.image_source",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyWholeImageSource {
    pub inner: WholeImageSource,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyWholeImageSource {
    /// Build a `WholeImageSource` from a 2-D uint8 numpy array.
    ///
    /// :param image: 2-D uint8 numpy array of shape ``(height, width)``.
    #[new]
    fn new(image: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = image.py();
        let (data, height, width) = extract_flat_u8(py, image)?;
        if height == 0 || width == 0 {
            return Err(PyValueError::new_err(
                "WholeImageSource: image has zero dimension",
            ));
        }
        Ok(PyWholeImageSource {
            inner: WholeImageSource::new(PixelImage {
                data,
                height,
                width,
            }),
        })
    }

    /// Pixel dimensions as ``(width, height)``.
    #[getter]
    fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }

    /// Pixel width.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width
    }

    /// Pixel height.
    #[getter]
    fn height(&self) -> u32 {
        self.inner.height
    }

    /// Pull a horizontal slab ``[y_start, y_end)`` and return it as
    /// a flat ``bytes`` object of length ``rows * width``.
    ///
    /// :param y_start: First row to read (inclusive).
    /// :param y_end:   Last row to read (exclusive); clipped to
    ///     image height.
    /// :returns: ``bytes`` of length
    ///     ``(y_end_clamped - y_start) * width``. Returns ``b""`` when
    ///     ``y_start >= height`` or ``y_end <= y_start``.
    fn read_slab(&self, y_start: u32, y_end: u32) -> Vec<u8> {
        let clamped_end = y_end.min(self.inner.height);
        if clamped_end <= y_start {
            return Vec::new();
        }
        let row_bytes = self.inner.width as usize;
        let rows = (clamped_end - y_start) as usize;
        let mut dst = vec![0u8; rows * row_bytes];
        self.inner.read_slab(y_start, y_end, &mut dst);
        dst
    }

    /// Return the full image as flat row-major ``bytes``, or ``None``
    /// when the source cannot materialise a full buffer.
    ///
    /// `WholeImageSource` always returns ``Some``.
    fn read_all(&self) -> Option<Vec<u8>> {
        self.inner.read_all()
    }

    /// Cancellation probe. `WholeImageSource` is never cancelled.
    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn __repr__(&self) -> String {
        format!(
            "WholeImageSource(width={}, height={})",
            self.inner.width, self.inner.height,
        )
    }
}

pub fn register(part_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = part_mod.py();
    let m = PyModule::new(py, "image_source")?;
    m.setattr(
        "__doc__",
        "In-memory and lazy sources of pixel data for raster/shrinkwrap assemblers.",
    )?;
    m.add_class::<PyWholeImageSource>()?;
    part_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.part.image_source", &m)?;

    Ok(())
}
