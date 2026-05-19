use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    raygeo::stub_info()?.generate()?;
    Ok(())
}
