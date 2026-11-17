//! Bindings for the point-on-curve constraint.

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
use crate::sketch::constraints::point_on_curve::PointOnCurveConstraint as CoreConstraint;

/// Enforces a point lies on a cubic Bezier curve.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_curve",
    name = "PointOnCurveConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointOnCurveConstraint {
    /// The constrained point.
    #[pyo3(get, set)]
    pub point_id: i64,
    /// The constrained curve.
    #[pyo3(get, set)]
    pub shape_id: i64,
    /// Reference user-visibility flag.
    #[pyo3(get, set)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointOnCurveConstraint {
    /// Creates the point-on-curve constraint.
    #[new]
    #[pyo3(signature = (point_id, shape_id, user_visible = true))]
    fn new(
        point_id: i64,
        shape_id: i64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase::default()).add_subclass(
            PyPointOnCurveConstraint {
                point_id,
                shape_id,
                user_visible,
                status: 1,
            },
        )
    }

    /// Returns the constraint error at the registry's point
    /// positions.
    #[pyo3(signature = (reg, params = None))]
    fn error(&self, reg: PointCtx<'_>, params: Option<Py<PyAny>>) -> f64 {
        let _ = params;
        self.clone()
            .into_constraint()
            .error(&reg)
            .first()
            .copied()
            .unwrap_or(0.0)
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
        dict_type(&dict, "PointOnCurveConstraint")?;
        dict.set_item("point_id", self.point_id)?;
        dict.set_item("shape_id", self.shape_id)?;
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
        kwargs.set_item("point_id", data.get_item("point_id")?)?;
        kwargs.set_item("shape_id", data.get_item("shape_id")?)?;
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

    /// The point that can be dragged to manipulate this constraint.
    fn get_draggable_point(&self) -> i64 {
        self.point_id
    }

    /// The string key identifying this constraint type.
    #[classmethod]
    fn get_type_key(_cls: &Bound<'_, PyType>) -> &'static str {
        "point_on_curve"
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Point on Curve")
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
        if point_ids.len() != 1 || entity_ids.len() != 1 {
            return Ok(false);
        }
        let Some(entities) = surface::resolve_entities(selection, sketch)?
        else {
            return Ok(false);
        };
        if !applies_check(&entities) {
            return Ok(false);
        }
        let Some(entity) = entities[0].as_deref() else {
            return Ok(false);
        };
        let point_id = point_ids[0] as u64;
        Ok(!entity.get_endpoint_ids().contains(&point_id))
    }

    /// Whether the operand supports point-on-curve constraints.
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

    /// The subtitle describing the constrained point.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let points = |id: i64| -> Option<(f64, f64)> {
            registry
                .get_point(id as u64)
                .map(|point| (point.x, point.y))
        };
        surface::subtitle_point_at(py, &points, self.point_id)
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
/// and it must support point-on-curve constraints.
fn applies_check(
    entities: &[Option<Box<dyn crate::sketch::entities::Entity>>],
) -> bool {
    entities.len() == 1
        && entities[0]
            .as_deref()
            .is_some_and(|entity| entity.supports_point_on_curve())
}

impl PyPointOnCurveConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            point_id: self.point_id as u64,
            shape_id: self.shape_id as u64,
            user_visible: self.user_visible,
        }
    }
}

/// Finds the point on a cubic Bezier curve closest to `(px, py)`,
/// returning `(t, bx, by)`: the curve parameter and the coordinates
/// of the closest curve point (the reference module-level helper).
#[gen_stub_pyfunction(module = "raygeo.sketch.constraints.point_on_curve")]
#[pyfunction]
fn closest_point_on_bezier(
    xs: (f64, f64, f64, f64),
    ys: (f64, f64, f64, f64),
    px: f64,
    py: f64,
) -> PyResult<(f64, f64, f64)> {
    let closest = crate::geo::shape::bezier::get_bezier_closest_point(
        crate::geo::types::Point3D::new(xs.3, ys.3, 0.0),
        crate::geo::types::Point3D::new(xs.1, ys.1, 0.0),
        crate::geo::types::Point3D::new(xs.2, ys.2, 0.0),
        crate::geo::types::Point3D::new(xs.0, ys.0, 0.0),
        px,
        py,
    );
    match closest {
        Some((t, point, _)) => Ok((t, point.x, point.y)),
        None => Ok((0.0, xs.0, ys.0)),
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPointOnCurveConstraint>()?;
    m.add_function(wrap_pyfunction!(self::closest_point_on_bezier, m)?)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.point_on_curve", m)?;
    Ok(())
}
