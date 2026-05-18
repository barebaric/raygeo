use pyo3::prelude::*;

mod geo;

#[pymodule(gil_used = false)]
fn raygeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    geo::register(m)?;
    Ok(())
}
