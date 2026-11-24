use numpy::PyReadonlyArray2;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::image::pack::{PackMask, Packer};

/// Greedy first-fit packer for binary canvases.
///
/// The canvas is a 2-D uint8 array where nonzero pixels are occupied.
/// Each call to :meth:`place` tests every supplied footprint (one per
/// allowed rotation), records the first collision-free position in
/// row-major order, and chooses the footprint whose placement yields
/// the smallest combined bounding box. The chosen footprint is then
/// marked on the canvas.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.image.pack", name = "Packer")]
pub struct PyPacker {
    inner: Packer,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPacker {
    /// Creates a packer from a 2-D uint8 canvas.
    ///
    /// :param canvas: 2-D uint8 array; nonzero pixels are occupied.
    #[new]
    fn new(canvas: PyReadonlyArray2<'_, u8>) -> Self {
        let array = canvas.as_array();
        let height = array.shape()[0];
        let width = array.shape()[1];
        let flat: Vec<u8> = array.iter().copied().collect();
        Self {
            inner: Packer::new(&flat, width, height),
        }
    }

    /// Places one item and marks it on the canvas.
    ///
    /// :param masks: One 2-D uint8 footprint per allowed rotation;
    ///     nonzero pixels are part of the footprint.
    /// :returns: ``(variant_index, y, x)`` for the best placement, or
    ///     ``None`` when no variant fits.
    /// :complexity: O(k * w * h * log(w * h)) for k variants
    fn place(
        &mut self,
        masks: Vec<PyReadonlyArray2<'_, u8>>,
    ) -> Option<(usize, usize, usize)> {
        let masks: Vec<PackMask> = masks
            .iter()
            .map(|mask| {
                let array = mask.as_array();
                let height = array.shape()[0];
                let width = array.shape()[1];
                PackMask {
                    width,
                    height,
                    data: array.iter().copied().collect(),
                }
            })
            .collect();
        self.inner.place(&masks)
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "pack")?;
    sub_mod.add_class::<PyPacker>()?;
    m.add_submodule(&sub_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.pack", &sub_mod)?;
    Ok(())
}
