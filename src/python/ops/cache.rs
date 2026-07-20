pyo3_stub_gen::module_doc!("raygeo.ops.cache", "{}", MODULE_DOC);

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::cache::CacheKey;

pub(crate) const MODULE_DOC: &str = "\
Cache key type used by the Cacheable trait.

--- 

:py:class:`CacheKey` is a caller-provided ``tag`` plus an
assembler-computed hash of the spec and face-state fields that the
component actually reads.  ``AssemblyOutput`` (the assembler's cached
output) lives at :py:mod:`raygeo.ops.assembly`.
";

/// An assembler-computed cache key.
///
/// Pair of a caller-provided ``tag`` (used for prefix-based pruning)
/// and an assembler-computed hash of the spec plus face-state fields
/// that the assembler actually reads.
///
/// Returned by
/// :meth:`Assembler.cache_key() <raygeo.ops.assembly.Assembler.cache_key>`.
/// The consumer does not interpret the ``payload_hash`` — it only
/// compares it for equality.
#[gen_stub_pyclass(module = "raygeo.ops.cache")]
#[pyclass(name = "CacheKey", module = "raygeo.ops.cache", skip_from_py_object)]
#[derive(Clone)]
pub struct PyCacheKey {
    pub inner: CacheKey,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCacheKey {
    #[new]
    fn new(tag: String, payload_hash: u64) -> Self {
        PyCacheKey {
            inner: CacheKey { tag, payload_hash },
        }
    }

    /// Caller-provided identifier used for prefix-based pruning.
    #[getter]
    fn tag(&self) -> &str {
        &self.inner.tag
    }

    /// Assembler-computed hash of its read-set fields.
    #[getter]
    fn payload_hash(&self) -> u64 {
        self.inner.payload_hash
    }

    fn __repr__(&self) -> String {
        format!(
            "CacheKey(tag={:?}, payload_hash={})",
            self.inner.tag, self.inner.payload_hash
        )
    }
}

impl From<&PyCacheKey> for CacheKey {
    fn from(py: &PyCacheKey) -> Self {
        py.inner.clone()
    }
}

pub fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "cache")?;
    m.setattr("__doc__", "Cache key type used by the Cacheable trait.")?;
    m.add_class::<PyCacheKey>()?;
    ops_mod.add_submodule(&m)
}
