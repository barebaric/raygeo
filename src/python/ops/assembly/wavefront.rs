use crate::ops::assembly::wavefront;
use crate::ops::assembly::Tracelet;
use crate::ops::state::State;
use crate::prof::prof_report;
use crate::python::ops::assembly::result::PyAssemblyResult;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "wavefront")?;
    register_functions!(
        m,
        adaptive_wavefronts_py,
        adaptive_wavefronts_multi_pocket_py,
    );
    m.add_class::<PyAdaptiveWavefrontSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.wavefront", &m)?;

    Ok(())
}

/// Parameters for the inside-out adaptive wavefront assembler.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.wavefront",
    name = "AdaptiveWavefrontSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyAdaptiveWavefrontSpec {
    #[pyo3(get)]
    pub step_over: f64,
    #[pyo3(get)]
    pub z: f64,
    #[pyo3(get)]
    pub area_tolerance: f64,
    #[pyo3(get)]
    pub precision: f64,
}

impl PyAdaptiveWavefrontSpec {
    pub fn into_core(self) -> wavefront::AdaptiveWavefrontSpec {
        wavefront::AdaptiveWavefrontSpec {
            step_over: self.step_over,
            z: self.z,
            area_tolerance: self.area_tolerance,
            precision: self.precision,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyAdaptiveWavefrontSpec {
    #[new]
    #[pyo3(signature = (
        step_over = 2.0,
        z = 0.0,
        area_tolerance = 1.0,
        precision = 0.0,
    ))]
    fn new(
        step_over: f64,
        z: f64,
        area_tolerance: f64,
        precision: f64,
    ) -> Self {
        PyAdaptiveWavefrontSpec {
            step_over,
            z,
            area_tolerance,
            precision,
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def adaptive_wavefronts(
        part: raygeo.ops.part.Part,
        step_over: float = 2.0,
        z: float = 0.0,
        area_tolerance: float = 1.0,
        precision: float = 0.0,
        cut_feed_rate: int = 1200,
        cut_power: float = 1.0,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Inside-out adaptive wavefronts.

        Finds the largest inscribed circle inside *part*'s boundary,
        seeds the cleared area with concentric rings spaced
        *step_over* apart, then iteratively expands the frontier
        outward by *step_over*, clipping to the boundary.  The loop
        terminates when the newly added area drops below
        *area_tolerance*.

        Each ring fragment is emitted as ``MoveTo`` + ``LineTo`` at
        height *z* with *cut_feed_rate* applied.

        :param part: The part whose ``stock_region`` defines the
                     pocket boundary and islands.
        :param step_over: Radial expansion per iteration (default 2.0).
        :param z: Z height for generated commands (default 0.0).
        :param area_tolerance: Minimum area increase to continue (default 1.0).
        :param precision: Edge tolerance for frontier simplification and vertex
                          resampling; smaller values produce denser edges
                          (default 0.0 = use internal default).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param cut_power: Laser power for cutting moves (0.0-1.0, default 1.0).
        :returns: An :class:`AssemblyResult` with wavefront cutting commands.
        """
    "#,
    module = "raygeo.ops.assembly.wavefront"
)]
#[pyfunction(name = "adaptive_wavefronts")]
#[pyo3(signature = (
    part,
    step_over = 2.0,
    z = 0.0,
    area_tolerance = 1.0,
    precision = 0.0,
    cut_feed_rate = 1200,
    cut_power = 1.0,
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_wavefronts_py(
    part: &mut crate::python::ops::part::part::PyPart,
    step_over: f64,
    z: f64,
    area_tolerance: f64,
    precision: f64,
    cut_feed_rate: i32,
    cut_power: f64,
) -> PyResult<PyAssemblyResult> {
    let opts = wavefront::AdaptiveWavefrontSpec {
        step_over,
        z,
        area_tolerance,
        precision,
    };

    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let mut trace = Tracelet::new();
    let face = part.inner.face_mut("");
    let meta =
        wavefront::adaptive_wavefronts(face, &mut trace, &opts, &cut_state)?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def adaptive_wavefronts_multi_pocket(
        part: raygeo.ops.part.Part,
        step_over: float = 2.0,
        offset_mm: float = 0.0,
        area_tolerance: float = 0.01,
        precision: float = 0.0,
        cut_feed_rate: int = 1200,
        cut_power: float = 1.0,
        profile: bool = False,
    ) -> raygeo.ops.assembly.AssemblyResult:
        """Multi-pocket adaptive wavefronts.

        Extracts all pockets from *part.geometry*, optionally offsets
        the boundary inward by *offset_mm*, seeds each pocket with
        concentric rings spaced *step_over* apart, and runs wavefront
        expansion inside each pocket.  Returns the combined result.

        :param part: The part whose geometry defines the pockets.
        :param step_over: Radial expansion per iteration (default 2.0).
        :param offset_mm: Inward offset applied to all contours (default 0.0).
        :param area_tolerance: Minimum area increase to continue (default 0.01).
        :param precision: Edge tolerance for frontier simplification (default 0.0).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param cut_power: Laser power for cutting moves (0.0-1.0, default 1.0).
        :param profile: Print a profiling report to stdout (default False).
        :returns: An :class:`AssemblyResult` with combined wavefront paths.
        :raises ValueError: If the part has no geometry or no closed contours.
        """
    "#,
    module = "raygeo.ops.assembly.wavefront"
)]
#[pyfunction(name = "adaptive_wavefronts_multi_pocket")]
#[pyo3(signature = (
    part,
    step_over = 2.0,
    offset_mm = 0.0,
    area_tolerance = 0.01,
    precision = 0.0,
    cut_feed_rate = 1200,
    cut_power = 1.0,
    profile = false,
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_wavefronts_multi_pocket_py(
    part: &crate::python::ops::part::part::PyPart,
    step_over: f64,
    offset_mm: f64,
    area_tolerance: f64,
    precision: f64,
    cut_feed_rate: i32,
    cut_power: f64,
    profile: bool,
) -> PyResult<PyAssemblyResult> {
    let face = part.inner.face("").ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err("no default face")
    })?;
    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };
    let (ops, meta) = wavefront::adaptive_wavefronts_multi_pocket(
        face,
        step_over,
        offset_mm,
        area_tolerance,
        precision,
        &cut_state,
    )?;
    if profile {
        prof_report();
    }
    Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
}
