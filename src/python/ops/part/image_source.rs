//! Python bindings for [`WholeImageSource`] and [`VipsChunkSource`].
//!
//! Python code constructs a [`PyWholeImageSource`] from a 2-D uint8
//! numpy array and either passes it to [`Part`](crate::python::ops::part::part::PyPart)
//! via `part.image_source = WholeImageSource(...)` or retrieves it
//! back via the `part.image_source` getter.
//!
//! [`PyVipsChunkSource`] wraps a ``pyvips.Image`` so multi-GB surfaces
//! stay out of memory: slabs are cropped and materialised on demand.
//!
//! The Rust [`ImageSource`](crate::ops::part::image_source::ImageSource)
//! trait stays Rust-internal: Python code interacts only with the
//! concrete classes.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
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

// ---------------------------------------------------------------------------
// VipsChunkSource — lazy pyvips-backed ImageSource
// ---------------------------------------------------------------------------

/// Default threshold (256 MB) above which `read_all` returns `None`.
const DEFAULT_THRESHOLD_MB: usize = 256;

/// Lazy [`ImageSource`] backed by a ``pyvips.Image``.
///
/// Holds a Python reference to the pyvips image and materialises
/// horizontal slabs on demand via `crop().write_to_memory()`.
/// `read_all()` returns `None` when the full image exceeds
/// `in_memory_threshold`, forcing callers to use slab-by-slab access.
///
/// The struct is `Send + Sync` because `Py<PyAny>` is `Send + Sync`
/// (reference-counted GIL-safe handle). GIL is reacquired inside
/// `read_slab` / `read_all`.
#[derive(Clone)]
pub struct VipsChunkSource {
    vips_image: Py<PyAny>,
    width: u32,
    height: u32,
    in_memory_threshold: usize,
}

impl std::fmt::Debug for VipsChunkSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VipsChunkSource")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("in_memory_threshold", &self.in_memory_threshold)
            .finish()
    }
}

impl ImageSource for VipsChunkSource {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn read_slab(&self, y_start: u32, y_end: u32, dst: &mut [u8]) -> u32 {
        let slab_h = y_end.min(self.height).saturating_sub(y_start);
        if slab_h == 0 {
            return 0;
        }
        let needed = slab_h as usize * self.width as usize;
        if dst.len() < needed {
            return 0;
        }

        let result = Python::attach(|py| -> PyResult<()> {
            let bound = self.vips_image.bind(py);
            let slab = bound.call_method1(
                "crop",
                (0i64, y_start as i64, self.width as i64, slab_h as i64),
            )?;
            let data: Vec<u8> =
                slab.call_method0("write_to_memory")?.extract()?;
            if data.len() < needed {
                return Err(PyRuntimeError::new_err(format!(
                    "VipsChunkSource::read_slab: expected at least {} bytes \
                     from pyvips, got {}",
                    needed,
                    data.len(),
                )));
            }
            dst[..needed].copy_from_slice(&data[..needed]);
            Ok(())
        });

        if result.is_ok() {
            slab_h
        } else {
            0
        }
    }

    fn read_all(&self) -> Option<Vec<u8>> {
        let total = self.width as usize * self.height as usize;
        if total > self.in_memory_threshold {
            return None;
        }

        Python::attach(|py| -> PyResult<Option<Vec<u8>>> {
            let bound = self.vips_image.bind(py);
            let data: Vec<u8> =
                bound.call_method0("write_to_memory")?.extract()?;
            let needed = self.width as usize * self.height as usize;
            if data.len() < needed {
                return Err(PyRuntimeError::new_err(format!(
                    "VipsChunkSource::read_all: expected at least {} bytes \
                     from pyvips, got {}",
                    needed,
                    data.len(),
                )));
            }
            Ok(Some(data[..needed].to_vec()))
        })
        .ok()
        .flatten()
    }

    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Lazy ``ImageSource`` wrapping a ``pyvips.Image`` for bounded peak RSS.
///
/// Unlike [`WholeImageSource`], which eagerly copies the entire numpy
/// array into Rust memory, `VipsChunkSource` holds only a reference to
/// the pyvips image and materialises horizontal slabs on demand via
/// ``image.crop(0, y, w, h).write_to_memory()``.
///
/// For images below *in_memory_threshold_mb* (default 256 MB),
/// [`read_all`][PyVipsChunkSource::read_all] materialises the full
/// buffer so callers that need random access (raster, shrinkwrap) work
/// unchanged. Above the threshold, `read_all` returns `None`, forcing
/// the caller to fall back to slab-by-slab reads.
///
/// The pyvips image **must** be single-band uchar. Convert before
/// constructing:
///
/// ```python
/// img = image.colourspace("b-w").cast("uchar")
/// src = VipsChunkSource(img)
/// ```
#[gen_stub_pyclass(module = "raygeo.ops.part.image_source")]
#[pyclass(
    name = "VipsChunkSource",
    module = "raygeo.ops.part.image_source",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyVipsChunkSource {
    pub inner: VipsChunkSource,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyVipsChunkSource {
    /// Build a `VipsChunkSource` from a single-band uchar ``pyvips.Image``.
    ///
    /// :param vips_image: A ``pyvips.Image`` with exactly one band and
    ///     8-bit unsigned interpretation. Convert with
    ///     ``image.colourspace("b-w").cast("uchar")`` if necessary.
    /// :param in_memory_threshold_mb: Maximum image size (in MB) for
    ///     which `read_all` will materialise the full buffer. Defaults
    ///     to 256. Images above this threshold require slab-by-slab
    ///     access.
    #[new]
    #[pyo3(signature = (vips_image, in_memory_threshold_mb = DEFAULT_THRESHOLD_MB))]
    fn new(
        vips_image: &Bound<'_, PyAny>,
        in_memory_threshold_mb: usize,
    ) -> PyResult<Self> {
        let bands: u32 = vips_image.getattr("bands")?.extract()?;
        if bands != 1 {
            return Err(PyValueError::new_err(format!(
                "VipsChunkSource expects a single-band image, \
                 got {bands} bands — convert with \
                 image.colourspace(\"b-w\").cast(\"uchar\")"
            )));
        }

        let width: u32 = vips_image.getattr("width")?.extract()?;
        let height: u32 = vips_image.getattr("height")?.extract()?;

        if width == 0 || height == 0 {
            return Err(PyValueError::new_err(
                "VipsChunkSource: image has zero dimension",
            ));
        }

        Ok(PyVipsChunkSource {
            inner: VipsChunkSource {
                vips_image: vips_image.clone().unbind(),
                width,
                height,
                in_memory_threshold: in_memory_threshold_mb * 1024 * 1024,
            },
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
    ///     ``(y_end_clamped - y_start) * width``.
    fn read_slab(&self, y_start: u32, y_end: u32) -> Vec<u8> {
        let clamped_end = y_end.min(self.inner.height);
        if clamped_end <= y_start {
            return Vec::new();
        }
        let rows = (clamped_end - y_start) as usize;
        let mut dst = vec![0u8; rows * self.inner.width as usize];
        self.inner.read_slab(y_start, y_end, &mut dst);
        dst
    }

    /// Return the full image as flat row-major ``bytes``, or ``None``
    /// when the source cannot materialise the full buffer (image above
    /// the configured threshold).
    fn read_all(&self) -> Option<Vec<u8>> {
        self.inner.read_all()
    }

    /// Cancellation probe. `VipsChunkSource` is never cancelled by
    /// itself — the assembler polls its own callbacks.
    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn __repr__(&self) -> String {
        format!(
            "VipsChunkSource(width={}, height={})",
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
    m.add_class::<PyVipsChunkSource>()?;
    part_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.part.image_source", &m)?;

    Ok(())
}
