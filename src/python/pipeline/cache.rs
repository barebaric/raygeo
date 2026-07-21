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
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCacheKey {
    #[new]
    fn new(tag: String) -> Self {
        PyCacheKey { tag }
    }

    #[getter]
    fn tag(&self) -> &str {
        &self.tag
    }

    fn __repr__(&self) -> String {
        format!("CacheKey(tag={:?})", self.tag)
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let cache_mod = PyModule::new(py, "cache")?;
    cache_mod
        .setattr("__doc__", "Cache key type used by the pipeline cache.")?;
    cache_mod.add_class::<PyCacheKey>()?;
    m.add_submodule(&cache_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline.cache", &cache_mod)?;

    Ok(())
}
