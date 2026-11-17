//! Bindings for the parallelogram constraint.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::radius::dict_type;
use super::{surface, PyConstraintBase};

use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraint::{PointContext, SketchConstraint};
use crate::sketch::constraints::parallelogram::ParallelogramConstraint as CoreConstraint;

/// Enforces (p_width - p_origin) == (p4 - p_height).
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.parallelogram",
    name = "ParallelogramConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyParallelogramConstraint {
    /// Origin point.
    #[pyo3(get, set)]
    pub p_origin: i64,
    /// Width point.
    #[pyo3(get, set)]
    pub p_width: i64,
    /// Height point.
    #[pyo3(get, set)]
    pub p_height: i64,
    /// Fourth corner.
    #[pyo3(get, set)]
    pub p4: i64,
    /// Reference user-visibility flag.
    #[pyo3(get, set)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyParallelogramConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p_origin, p_width, p_height, p4,
                        user_visible = false))]
    fn new(
        p_origin: i64,
        p_width: i64,
        p_height: i64,
        p4: i64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyParallelogramConstraint {
                p_origin,
                p_width,
                p_height,
                p4,
                user_visible,
                status: 1,
            },
        )
    }

    /// The set of point ids this constraint references.
    fn get_referenced_point_ids(&self) -> HashSet<i64> {
        HashSet::from([self.p_origin, self.p_width, self.p_height, self.p4])
    }

    /// Partial derivatives of the error by point id.
    #[pyo3(signature = (reg, params = None))]
    fn gradient(
        &self,
        mut reg: PointCtx<'_>,
        params: Option<Py<PyAny>>,
    ) -> HashMap<u64, Vec<(f64, f64)>> {
        let _ = params;
        super::constraint_gradient(&self.clone().into_constraint(), &mut reg)
    }

    /// Point ids the constraint references.
    fn referenced_points(&self, registry: PointCtx<'_>) -> Vec<u64> {
        self.clone().into_constraint().referenced_points(&registry)
    }

    /// Returns the constraint error at the registry's point positions.
    #[pyo3(signature = (reg, params = None))]
    fn error(
        &self,
        reg: PointCtx<'_>,
        params: Option<Py<PyAny>>,
    ) -> (f64, f64) {
        let _ = params;
        let errors = self.clone().into_constraint().error(&reg);
        (
            errors.first().copied().unwrap_or(0.0),
            errors.get(1).copied().unwrap_or(0.0),
        )
    }

    /// Serializes to the reference constraint dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict_type(&dict, "ParallelogramConstraint")?;
        dict.set_item("p_origin", self.p_origin)?;
        dict.set_item("p_width", self.p_width)?;
        dict.set_item("p_height", self.p_height)?;
        dict.set_item("p4", self.p4)?;
        dict.set_item("user_visible", self.user_visible)?;
        Ok(dict)
    }

    /// Rebuilds the constraint from its reference dictionary shape.
    #[classmethod]
    fn from_dict(
        cls: &Bound<'_, PyType>,
        py: Python<'_>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("p_origin", data.get_item("p_origin")?)?;
        kwargs.set_item("p_width", data.get_item("p_width")?)?;
        kwargs.set_item("p_height", data.get_item("p_height")?)?;
        kwargs.set_item("p4", data.get_item("p4")?)?;
        kwargs.set_item(
            "user_visible",
            data.get_item("user_visible")?
                .and_then(|value| value.extract::<bool>().ok())
                .unwrap_or(false),
        )?;
        Ok(cls.as_any().call((), Some(&kwargs))?.unbind())
    }

    /// Draws the constraint's symbol onto the cairo context.
    #[pyo3(signature = (ctx, registry, to_screen, is_selected = false,
                        is_hovered = false, point_radius = 5.0))]
    #[allow(clippy::too_many_arguments)]
    fn draw<'py>(
        &self,
        ctx: Canvas<'py>,
        registry: DrawContext<'py>,
        to_screen: ScreenMap<'py>,
        is_selected: bool,
        is_hovered: bool,
        point_radius: f64,
    ) -> PyResult<()> {
        super::constraint_draw(
            &self.clone().into_constraint(),
            &ctx,
            &registry,
            &to_screen,
            is_selected,
            is_hovered,
            point_radius,
            self.status,
        )
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Parallelogram")
    }

    /// Whether this constraint can be applied to the selection.
    #[classmethod]
    #[pyo3(signature = (selection, _sketch = None))]
    fn can_apply_to(
        _cls: &Bound<'_, PyType>,
        selection: &Bound<'_, PyAny>,
        _sketch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let (point_ids, entity_ids) = surface::selection_ids(selection)?;
        Ok(point_ids.len() == 4 && entity_ids.is_empty())
    }

    /// The human-readable title for this constraint.
    fn get_title(slf: &Bound<'_, Self>, _py: Python<'_>) -> PyResult<String> {
        surface::title_plain(slf.as_any())
    }

    /// The subtitle describing the constrained points.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let Some(origin) = registry.get_point(self.p_origin as u64) else {
            return String::new();
        };
        let coord = surface::format_coord(py, origin.x, origin.y);
        surface::gettext_format(py, "Origin at {}", &[&coord])
    }

    /// Whether the constraint's rendered symbol is hit at (mx, my).
    #[pyo3(signature = (sx, sy, registry, to_screen, element,
                        threshold))]
    fn is_hit(
        &self,
        sx: f64,
        sy: f64,
        registry: PointCtx<'_>,
        to_screen: ScreenMap<'_>,
        element: Option<&Bound<'_, PyAny>>,
        threshold: f64,
    ) -> bool {
        let _ = element;
        let constraint = self.clone().into_constraint();
        super::constraint_is_hit(
            &constraint,
            sx,
            sy,
            threshold,
            &registry,
            Some(&to_screen),
        )
    }
}

impl PyParallelogramConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core = CoreConstraint::new(
            self.p_origin as u64,
            self.p_width as u64,
            self.p_height as u64,
            self.p4 as u64,
        );
        core.user_visible = self.user_visible;
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParallelogramConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.parallelogram", m)?;
    Ok(())
}
