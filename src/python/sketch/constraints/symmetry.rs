//! Bindings for the symmetry constraint.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use super::radius::dict_type;
use super::{surface, PyConstraintBase};
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, ScreenMap};
use crate::sketch::constraint::{PointContext, SketchConstraint};
use crate::sketch::constraints::symmetry::SymmetryConstraint as CoreConstraint;

/// Enforces two points are symmetric about a point or a line.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.symmetry",
    name = "SymmetryConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PySymmetryConstraint {
    /// First point.
    #[pyo3(get, set)]
    pub p1: i64,
    /// Second point.
    #[pyo3(get, set)]
    pub p2: i64,
    /// Symmetry center point, for point symmetry.
    #[pyo3(get, set)]
    pub center: Option<i64>,
    /// Symmetry axis line entity, for line symmetry.
    #[pyo3(get, set)]
    pub axis: Option<i64>,
    /// Reference user-visibility flag.
    #[pyo3(get, set)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySymmetryConstraint {
    /// Creates the symmetry constraint.
    #[new]
    #[pyo3(signature = (p1, p2, center = None, axis = None,
                        user_visible = true))]
    fn new(
        p1: i64,
        p2: i64,
        center: Option<i64>,
        axis: Option<i64>,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase::default()).add_subclass(
            PySymmetryConstraint {
                p1,
                p2,
                center,
                axis,
                user_visible,
                status: 1,
            },
        )
    }

    /// Rewrites every id reference through *id_map*.
    fn remap_references(
        &mut self,
        id_map: std::collections::HashMap<i64, i64>,
    ) {
        super::remap_id(&mut self.p1, &id_map);
        super::remap_id(&mut self.p2, &id_map);
        super::remap_opt(&mut self.center, &id_map);
        super::remap_opt(&mut self.axis, &id_map);
    }

    /// Returns the constraint error at the registry's point
    /// positions.
    #[pyo3(signature = (reg, params = None))]
    fn error(&self, reg: PointCtx<'_>, params: Option<Py<PyAny>>) -> Vec<f64> {
        let _ = params;
        self.clone().into_constraint().error(&reg)
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

    /// Serializes to the reference constraint dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict_type(&dict, "SymmetryConstraint")?;
        dict.set_item("p1", self.p1)?;
        dict.set_item("p2", self.p2)?;
        match self.center {
            Some(center) => dict.set_item("center", center)?,
            None => dict.set_item("center", py.None())?,
        }
        match self.axis {
            Some(axis) => dict.set_item("axis", axis)?,
            None => dict.set_item("axis", py.None())?,
        }
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
        kwargs.set_item("p1", data.get_item("p1")?)?;
        kwargs.set_item("p2", data.get_item("p2")?)?;
        kwargs.set_item(
            "center",
            match data.get_item("center")? {
                Some(value) => value,
                None => py.None().into_bound(py),
            },
        )?;
        kwargs.set_item(
            "axis",
            match data.get_item("axis")? {
                Some(value) => value,
                None => py.None().into_bound(py),
            },
        )?;
        kwargs.set_item(
            "user_visible",
            data.get_item("user_visible")?
                .and_then(|value| value.extract::<bool>().ok())
                .unwrap_or(true),
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
        registry: &Bound<'py, PyAny>,
        to_screen: ScreenMap<'py>,
        is_selected: bool,
        is_hovered: bool,
        point_radius: f64,
    ) -> PyResult<()> {
        super::constraint_draw(
            &self.clone().into_constraint(),
            &ctx,
            registry,
            &to_screen,
            is_selected,
            is_hovered,
            point_radius,
            self.status,
        )
    }

    /// The string key identifying this constraint type.
    #[classmethod]
    fn get_type_key(_cls: &Bound<'_, PyType>) -> &'static str {
        "symmetry"
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Symmetry")
    }

    /// Whether this constraint can be applied to the selection.
    #[classmethod]
    #[pyo3(signature = (selection, sketch = None))]
    fn can_apply_to(
        _cls: &Bound<'_, PyType>,
        selection: &Bound<'_, PyAny>,
        sketch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let (point_ids, entity_ids) = surface::selection_ids(selection)?;
        if point_ids.len() == 3 && entity_ids.is_empty() {
            return Ok(true);
        }
        if point_ids.len() == 2 && entity_ids.len() == 1 {
            let Some(entities) = surface::resolve_entities(selection, sketch)?
            else {
                return Ok(false);
            };
            return Ok(applies_check(&entities));
        }
        Ok(false)
    }

    /// Whether the operand is a line (the symmetry axis).
    #[classmethod]
    fn applies_to_entities(
        _cls: &Bound<'_, PyType>,
        entities: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let resolved = surface::entities_from_py(entities)?;
        Ok(applies_check(&resolved))
    }

    /// The human-readable title for this constraint.
    fn get_title(slf: &Bound<'_, Self>, _py: Python<'_>) -> PyResult<String> {
        surface::title_plain(slf.as_any())
    }

    /// The subtitle describing the constrained points.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let points = |id: i64| -> Option<(f64, f64)> {
            registry
                .get_point(id as u64)
                .map(|point| (point.x, point.y))
        };
        surface::subtitle_from_to(py, &points, self.p1, self.p2)
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

/// The reference `applies_to_entities` body: exactly one operand,
/// and it must be a line.
fn applies_check(
    entities: &[Option<Box<dyn crate::sketch::entities::entity::Entity>>],
) -> bool {
    entities.len() == 1
        && entities[0]
            .as_deref()
            .is_some_and(|entity| entity.type_name() == "line")
}

impl PySymmetryConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            p1: self.p1 as u64,
            p2: self.p2 as u64,
            center: self.center.map(|v| v as u64),
            axis: self.axis.map(|v| v as u64),
            user_visible: self.user_visible,
        }
    }
}

/// Draws the two symmetry arrow heads along the segment between
/// two screen-space points (the reference module-level helper).
#[gen_stub_pyfunction(module = "raygeo.sketch.constraints.symmetry")]
#[pyfunction]
fn draw_symmetry_arrows(
    ctx: Canvas<'_>,
    s1: (f64, f64),
    s2: (f64, f64),
) -> PyResult<()> {
    super::draw_with_cairo(&ctx, |canvas| {
        crate::sketch::draw::draw_symmetry_arrows(canvas, s1, s2)
    })
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySymmetryConstraint>()?;
    m.add_function(wrap_pyfunction!(self::draw_symmetry_arrows, m)?)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.symmetry", m)?;
    Ok(())
}
