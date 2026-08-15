use numpy::IntoPyArray;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::image::pbr;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def generate_brdf_lut(
        size: int = 32,
        sample_count: int = 1024,
    ) -> numpy.typing.NDArray[numpy.float32]:
        """Integrate the Cook-Torrance BRDF into a split-sum LUT.

        For each ``(NdotV, roughness)`` texel the GGX distribution is
        importance-sampled (Hammersley sequence) and the Smith
        geometry term integrated, giving the Fresnel scale/bias pair
        such that the specular IBL response is ``F0 * scale + bias``.

        Deterministic: repeated calls return identical data.

        :param size: LUT resolution in both axes.
        :param sample_count: Importance samples per texel.
        :returns: Float32 array of shape (size, size, 2) indexed as
            ``lut[roughness, NdotV] = (scale, bias)``.
        :complexity: O(size^2 * sample_count)
        """
"#,
    module = "raygeo.image.pbr"
)]
#[pyfunction(name = "generate_brdf_lut")]
#[pyo3(signature = (size = 32, sample_count = 1024))]
fn py_generate_brdf_lut(
    py: Python<'_>,
    size: usize,
    sample_count: usize,
) -> PyResult<Py<PyAny>> {
    if size == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "size must be positive",
        ));
    }
    if sample_count == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "sample_count must be positive",
        ));
    }
    let lut = py.detach(|| pbr::generate_brdf_lut(size, sample_count));
    let array = lut.into_pyarray(py);
    let reshaped = array.call_method1("reshape", (size, size, 2i64))?;
    Ok(reshaped.unbind())
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "pbr")?;
    m.setattr("__doc__", "PBR BRDF integration for IBL.")?;
    m.add_function(pyo3::wrap_pyfunction!(py_generate_brdf_lut, m.clone())?)?;
    parent.add_submodule(&m)?;
    let sys_modules = parent.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.image.pbr", &m)?;
    Ok(())
}
