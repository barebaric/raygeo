use pyo3::prelude::*;

mod geo;
mod ops;

#[pymodule(gil_used = false)]
fn raygeo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    geo::register(m)?;
    ops::register(m)?;
    Ok(())
}
