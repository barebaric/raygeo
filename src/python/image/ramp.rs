use numpy::IntoPyArray;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::ramp;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def edge_ramp(
        binary: numpy.typing.NDArray[numpy.uint8],
        ramp_length_mm: float,
        pixels_per_mm: tuple[float, float] = (1.0, 1.0),
    ) -> numpy.typing.NDArray[numpy.float32]:
        """Distance of each burned pixel to the edge of the burned area.

        For every nonzero pixel of ``binary``, returns its distance to
        the edge of the burned area divided by ``ramp_length_mm`` and
        clamped to 1, so 0 is the edge and 1 is at or beyond the ramp
        length; zero pixels get 0. Pixels outside the image count as
        not burned. A stroke narrower than twice the ramp length never
        reaches 1.

        The distance is Euclidean and in millimetres, so non-square
        pixels are handled. It runs from the pixel centre to the
        centre of the nearest unburned pixel, less the part of that
        line inside the unburned pixel: exact along the axes, and
        within half a pixel otherwise.

        Used for sloped edges in relief and stamp engraving, where the
        power ramps up from the edge over the ramp length.

        :param binary: 2D uint8 array, nonzero marks a burned pixel.
        :param ramp_length_mm: Ramp length in mm; 0 returns 0/1 floats.
        :param pixels_per_mm: (x, y) image resolution in pixels per mm.
        :returns: 2D float32 array of ramp fractions in [0, 1].
        :raises ValueError: If the ramp length or resolution is invalid.
        :complexity: O(w*h)
        """
"#,
    module = "raygeo.image.ramp"
)]
#[pyfunction(name = "edge_ramp")]
#[pyo3(signature = (binary, ramp_length_mm, pixels_per_mm=(1.0, 1.0)))]
fn py_edge_ramp(
    py: Python<'_>,
    binary: &Bound<'_, PyAny>,
    ramp_length_mm: f64,
    pixels_per_mm: (f64, f64),
) -> PyResult<Py<PyAny>> {
    if ramp_length_mm < 0.0 {
        return Err(PyValueError::new_err(format!(
            "ramp_length_mm must not be negative, got {ramp_length_mm}"
        )));
    }
    if pixels_per_mm.0 <= 0.0 || pixels_per_mm.1 <= 0.0 {
        return Err(PyValueError::new_err(format!(
            "pixels_per_mm must be positive, got {:?}",
            pixels_per_mm
        )));
    }
    let numpy = py.import("numpy")?;
    let arr = numpy.call_method1("asarray", (binary, "uint8"))?;
    let (height, width) = arr.getattr("shape")?.extract::<(usize, usize)>()?;
    let flat: Vec<u8> = arr
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;

    let mut output = vec![0.0f32; height * width];
    ramp::edge_ramp(
        &flat,
        width,
        height,
        ramp_length_mm,
        pixels_per_mm.0,
        pixels_per_mm.1,
        &mut output,
    );
    let result = output.into_pyarray(py);
    Ok(result.call_method1("reshape", (height, width))?.unbind())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "ramp")?;
    sub_mod.add_function(wrap_pyfunction!(py_edge_ramp, sub_mod.clone())?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.ramp", &sub_mod)?;
    Ok(())
}
