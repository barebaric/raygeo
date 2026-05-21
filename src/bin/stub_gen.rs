use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = raygeo::stub_info()?;
    stub.generate()?;
    Ok(())
}
