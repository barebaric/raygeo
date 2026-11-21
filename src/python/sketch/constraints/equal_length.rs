//! Bindings for the equal-length constraint.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::radius::dict_type;
use super::{surface, PyConstraintBase};

use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, ScreenMap};
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::equal_length::EqualLengthConstraint as CoreConstraint;

/// Enforces equal characteristic lengths, one residual per
/// `(base_a, base_b, other_a, other_b)` tuple.  The caller expands
/// the entities' characteristic-length pairs (including the
/// shorter-side fallback) into flat tuples.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.equal_length",
    name = "EqualLengthConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyEqualLengthConstraint {
    /// The constrained entities.
    #[pyo3(get, set)]
    pub entity_ids: Vec<i64>,
    /// Reference user-visibility flag.
    #[pyo3(get, set)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEqualLengthConstraint {
    /// Creates the constraint for the given entities.
    #[new]
    #[pyo3(signature = (entity_ids, user_visible = true))]
    fn new(
        entity_ids: Vec<i64>,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase::default()).add_subclass(
            PyEqualLengthConstraint {
                entity_ids,
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
        super::remap_ids(&mut self.entity_ids, &id_map);
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

    /// The set of entity ids this constraint references.
    fn get_referenced_entity_ids(&self) -> HashSet<i64> {
        self.entity_ids.iter().copied().collect()
    }

    /// Serializes to the reference constraint dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict_type(&dict, "EqualLengthConstraint")?;
        dict.set_item("entity_ids", self.entity_ids.clone())?;
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
        kwargs.set_item("entity_ids", data.get_item("entity_ids")?)?;
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
        "equal"
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Equal Length")
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
        if !point_ids.is_empty() || entity_ids.len() < 2 {
            return Ok(false);
        }
        let Some(entities) = surface::resolve_entities(selection, sketch)?
        else {
            return Ok(false);
        };
        Ok(applies_check(&entities))
    }

    /// Whether all operands define a characteristic length.
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

    /// The subtitle describing the constrained entities.
    fn get_subtitle(&self, py: Python<'_>, _registry: PointCtx<'_>) -> String {
        if self.entity_ids.len() >= 2 {
            let count = self.entity_ids.len().to_string();
            return surface::gettext_format(py, "{} entities", &[&count]);
        }
        String::new()
    }

    /// Whether this constraint restricts the length of the given
    /// entity.
    fn targets_segment(
        &self,
        _p1: i64,
        _p2: i64,
        entity_id: Option<i64>,
    ) -> bool {
        entity_id.is_some_and(|eid| self.entity_ids.contains(&eid))
    }

    /// Whether this constraint explicitly defines the radius/length
    /// of the given entity.
    fn constrains_radius(
        &self,
        _registry: PointCtx<'_>,
        entity_id: i64,
    ) -> bool {
        self.entity_ids.contains(&entity_id)
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

/// The reference `applies_to_entities` body: at least two operands,
/// all defining characteristic-length pairs.
fn applies_check(
    entities: &[Option<Box<dyn crate::sketch::entities::entity::Entity>>],
) -> bool {
    entities.len() >= 2
        && entities.iter().all(|entity| {
            entity.as_deref().is_some_and(|entity| {
                !entity.characteristic_length_pairs().is_empty()
            })
        })
}

impl PyEqualLengthConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            entity_ids: self.entity_ids.iter().map(|&id| id as u64).collect(),
            user_visible: self.user_visible,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEqualLengthConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.equal_length", m)?;
    Ok(())
}
