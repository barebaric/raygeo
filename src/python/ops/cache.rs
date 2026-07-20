pyo3_stub_gen::module_doc!("raygeo.ops.cache", "{}", MODULE_DOC);

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::cache::{AssemblyOutput, CacheKey};
use crate::python::geo::flex_point::{
    polygons_from_tuples, polygons_to_tuples,
};
use crate::python::ops::container::PyOps;

pub(crate) const MODULE_DOC: &str = "\
Assembler-output caching types (CacheKey, AssemblyOutput).

Types used by the Cacheable trait: cache keys for identifying
entries and the packaged assembler output (AssemblyOutput).
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
/// ``payload_hash`` — it only compares it for equality.
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

/// The output of an assembler, packaged for caching.
///
/// Produced by
/// :meth:`Assembler.store_cache() <raygeo.ops.assembly.Assembler.store_cache>`
/// and consumed by
/// :meth:`Assembler.restore_cache() <raygeo.ops.assembly.Assembler.restore_cache>`.
///
/// Carries the assembled ``Ops``, metadata, and optional post-assembly
/// cleared fragments for face-state restoration on cache hit.
#[gen_stub_pyclass(module = "raygeo.ops.cache")]
#[pyclass(
    name = "AssemblyOutput",
    module = "raygeo.ops.cache",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAssemblyOutput {
    pub inner: AssemblyOutput,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAssemblyOutput {
    #[new]
    #[pyo3(signature = (ops, is_scalable = false, source_dimensions = None, cleared_fragments = None))]
    fn new(
        ops: &PyOps,
        is_scalable: bool,
        source_dimensions: Option<(f64, f64)>,
        cleared_fragments: Option<Vec<Vec<(f64, f64)>>>,
    ) -> Self {
        let frags = cleared_fragments.map(polygons_from_tuples);
        PyAssemblyOutput {
            inner: AssemblyOutput {
                ops: ops.inner.clone(),
                is_scalable,
                source_dimensions,
                cleared_fragments: frags,
            },
        }
    }

    /// The assembled Ops.
    #[getter]
    fn ops(&self) -> PyOps {
        PyOps {
            inner: self.inner.ops.clone(),
        }
    }

    /// Whether the Ops may be uniformly scaled during aggregation.
    #[getter]
    fn is_scalable(&self) -> bool {
        self.inner.is_scalable
    }

    /// Source ``(width_mm, height_mm)`` of the part that produced the Ops.
    #[getter]
    fn source_dimensions(&self) -> Option<(f64, f64)> {
        self.inner.source_dimensions
    }

    /// Post-assembly cleared fragments (``list[list[(x, y)]]``), or
    /// ``None`` for assemblers that don't touch ``FaceState.cleared``.
    #[getter]
    fn cleared_fragments(&self) -> Option<Vec<Vec<(f64, f64)>>> {
        self.inner
            .cleared_fragments
            .as_ref()
            .map(|frags| polygons_to_tuples(frags.clone()))
    }

    fn __repr__(&self) -> String {
        let n_frags = self
            .inner
            .cleared_fragments
            .as_ref()
            .map(|f| f.len())
            .unwrap_or(0);
        format!(
            "AssemblyOutput(ops_len={}, is_scalable={}, source_dimensions={:?}, n_fragments={})",
            self.inner.ops.len(),
            self.inner.is_scalable,
            self.inner.source_dimensions,
            n_frags,
        )
    }
}

pub fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "cache")?;
    m.setattr(
        "__doc__",
        "Assembler-output caching types (CacheKey, AssemblyOutput).",
    )?;
    m.add_class::<PyCacheKey>()?;
    m.add_class::<PyAssemblyOutput>()?;
    ops_mod.add_submodule(&m)
}
