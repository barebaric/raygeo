use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass(module = "raygeo.pipeline.cache")]
#[pyclass(
    name = "CacheKey",
    module = "raygeo.pipeline.cache",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCacheKey {
    pub tag: String,
    pub payload_hash: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCacheKey {
    #[new]
    fn new(tag: String, payload_hash: u64) -> Self {
        PyCacheKey { tag, payload_hash }
    }

    #[getter]
    fn tag(&self) -> &str {
        &self.tag
    }

    #[getter]
    fn payload_hash(&self) -> u64 {
        self.payload_hash
    }

    fn __repr__(&self) -> String {
        format!(
            "CacheKey(tag={:?}, payload_hash={})",
            self.tag, self.payload_hash
        )
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let cache_mod = PyModule::new(py, "cache")?;
    cache_mod
        .setattr("__doc__", "Cache key type used by the assembler cache.")?;
    cache_mod.add_class::<PyCacheKey>()?;
    m.add_submodule(&cache_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline.cache", &cache_mod)?;

    Ok(())
}
