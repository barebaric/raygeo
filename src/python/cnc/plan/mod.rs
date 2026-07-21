pyo3_stub_gen::module_doc!("raygeo.cnc.plan", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Plan-time description of machining operations.

Plans are produced by planners and consumed by Rayforge to derive its
own Step classes.  They are never executed directly.
";

pub(crate) mod clearing;
pub(crate) mod entry;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::cnc::plan::plan::{self, PlanStep};
use crate::types::{Point, Polygon};

/// One step in a Plan: a face_id and an assembler spec.
#[gen_stub_pyclass(module = "raygeo.cnc.plan")]
#[pyclass(name = "PlanStep", module = "raygeo.cnc.plan", skip_from_py_object)]
#[derive(Debug)]
pub struct PyPlanStep {
    pub inner: PlanStep,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPlanStep {
    /// The face this step targets.
    #[getter]
    fn face_id(&self) -> &str {
        &self.inner.face_id
    }

    /// The assembler kind (e.g. ``"helix"``, ``"adaptive_clearing"``).
    #[getter]
    fn kind(&self) -> &'static str {
        self.inner.spec.name()
    }

    /// All spec parameters as a Python dict.
    fn spec_params<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        d.set_item("kind", self.inner.spec.name())?;

        let spec = &self.inner.spec;

        if let Some(h) = spec.as_any().downcast_ref::<crate::ops::assembly::helix::HelixSpec>() {
            d.set_item("center", (h.center.x, h.center.y))?;
            d.set_item("start_radius", h.start_radius)?;
            d.set_item("z_start", h.z_start)?;
            d.set_item("z_end", h.z_end)?;
            d.set_item("pitch", h.pitch)?;
            d.set_item("direction", if matches!(h.direction, crate::geo::algo::helix::HelixDirection::Cw) { "CW" } else { "CCW" })?;
            d.set_item("angular_step", h.angular_step)?;
        } else if let Some(s) = spec.as_any().downcast_ref::<crate::ops::assembly::spiral::SpiralSpec>() {
            d.set_item("center", (s.center.x, s.center.y))?;
            d.set_item("z", s.z)?;
            d.set_item("start_radius", s.start_radius)?;
            d.set_item("end_radius", s.end_radius)?;
            d.set_item("revolutions", s.revolutions)?;
            d.set_item("direction", if matches!(s.direction, crate::geo::algo::helix::HelixDirection::Cw) { "CW" } else { "CCW" })?;
            d.set_item("angular_step", s.angular_step)?;
            d.set_item("start_angle", s.start_angle)?;
        } else if let Some(r) = spec.as_any().downcast_ref::<crate::ops::assembly::ramp::RampSpec>() {
            d.set_item("start", (r.start.x, r.start.y))?;
            d.set_item("end", (r.end.x, r.end.y))?;
            d.set_item("z_start", r.z_start)?;
            d.set_item("z_end", r.z_end)?;
            d.set_item("max_ramp_angle_deg", r.max_ramp_angle_deg)?;
            d.set_item("lateral_amplitude", r.lateral_amplitude)?;
        } else if let Some(t) = spec.as_any().downcast_ref::<crate::ops::assembly::toroid::ToroidalClearSpec>() {
            let carrier: Vec<(f64, f64)> = t.carrier.iter().map(|p| (p.x, p.y)).collect();
            d.set_item("carrier", carrier)?;
            d.set_item("start", (t.start.x, t.start.y, t.start.z))?;
            d.set_item("target_z", t.target_z)?;
            d.set_item("tool_radius", t.tool_radius)?;
            d.set_item("step_over", t.step_over)?;
            d.set_item("max_ramp_angle_deg", t.max_ramp_angle_deg)?;
            d.set_item("direction", if matches!(t.direction, crate::geo::algo::helix::HelixDirection::Cw) { "CW" } else { "CCW" })?;
            d.set_item("angular_step", t.angular_step)?;
        } else if let Some(sl) = spec.as_any().downcast_ref::<crate::ops::assembly::slot::SlotSpec>() {
            let carrier: Vec<(f64, f64)> = sl.carrier.iter().map(|p| (p.x, p.y)).collect();
            d.set_item("carrier", carrier)?;
            d.set_item("tool_radius", sl.tool_radius)?;
            d.set_item("target_z", sl.target_z)?;
        } else if let Some(a) = spec.as_any().downcast_ref::<crate::ops::assembly::adaptive::AdaptiveClearingSpec>() {
            d.set_item("tool_radius", a.tool_radius)?;
            d.set_item("step_over", a.step_over)?;
            d.set_item("step_length", a.step_length)?;
            d.set_item("target_z", a.target_z)?;
            d.set_item("safe_z", a.safe_z)?;
            d.set_item("max_deflection_deg", a.max_deflection_deg)?;
            d.set_item("wall_margin", a.wall_margin)?;
            d.set_item("area_tolerance", a.area_tolerance)?;
            d.set_item("angular_step", a.tolerance)?;
        } else if let Some(p) = spec.as_any().downcast_ref::<crate::ops::assembly::profile::ProfileSpec>() {
            d.set_item("kind_name", format!("{:?}", p.kind))?;
            d.set_item("tool_radius", p.tool_radius)?;
            d.set_item("step_over", p.step_over)?;
            d.set_item("step_length", p.step_length)?;
            d.set_item("target_z", p.target_z)?;
            d.set_item("safe_z", p.safe_z)?;
            d.set_item("wall_margin", p.wall_margin)?;
            d.set_item("stock_to_leave", p.stock_to_leave)?;
        }

        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "PlanStep(face_id={}, spec={})",
            self.inner.face_id,
            self.inner.spec.name(),
        )
    }
}

/// A descriptive Plan produced by planners.
#[gen_stub_pyclass(module = "raygeo.cnc.plan")]
#[pyclass(name = "Plan", module = "raygeo.cnc.plan", skip_from_py_object)]
#[derive(Debug)]
pub struct PyPlan {
    pub inner: plan::Plan,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPlan {
    /// Create a Plan from a pocket boundary with optional islands.
    #[new]
    #[pyo3(signature = (pocket_boundary, islands = None, safe_z = 2.0))]
    fn __new__(
        pocket_boundary: Vec<(f64, f64)>,
        islands: Option<Vec<Vec<(f64, f64)>>>,
        safe_z: f64,
    ) -> Self {
        let boundary: Polygon = pocket_boundary
            .into_iter()
            .map(|(x, y)| Point::new(x, y))
            .collect();
        let islands_vec: Vec<Polygon> = islands
            .unwrap_or_default()
            .into_iter()
            .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        PyPlan {
            inner: plan::Plan::new(boundary, islands_vec, safe_z),
        }
    }

    /// Number of PlanSteps in this plan.
    #[getter]
    fn step_count(&self) -> usize {
        self.inner.steps.len()
    }

    /// The safe Z height for retract moves.
    #[getter]
    fn safe_z(&self) -> f64 {
        self.inner.safe_z
    }

    /// The list of PlanSteps in this plan.
    #[getter]
    fn steps(&self, py: Python<'_>) -> PyResult<Vec<Py<PyPlanStep>>> {
        let mut result = Vec::with_capacity(self.inner.steps.len());
        for step in &self.inner.steps {
            let cloned = plan::PlanStep {
                face_id: step.face_id.clone(),
                spec: step.spec.boxed_clone().into(),
                region_boundary: None,
            };
            result.push(Py::new(py, PyPlanStep { inner: cloned })?);
        }
        Ok(result)
    }

    /// Append PlanSteps to this plan.
    fn extend(&mut self, steps: Vec<PyRef<'_, PyPlanStep>>) {
        for s in steps {
            self.inner.steps.push(s.inner.clone());
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Plan(steps={}, safe_z={})",
            self.inner.steps.len(),
            self.inner.safe_z
        )
    }
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let plan_mod = PyModule::new(py, "plan")?;
    plan_mod.setattr("__doc__", MODULE_DOC)?;
    plan_mod.add_class::<PyPlan>()?;
    plan_mod.add_class::<PyPlanStep>()?;
    clearing::register(&plan_mod)?;
    entry::register(&plan_mod)?;
    parent.add_submodule(&plan_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.plan", &plan_mod)?;

    Ok(())
}
