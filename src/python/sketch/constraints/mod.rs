//! Python bindings for the sketch constraints, mirroring the
//! `sketch::constraints` module tree: one leaf module per constraint.

pub(crate) mod angle;
pub(crate) mod aspect_ratio;
pub(crate) mod coincident;
pub(crate) mod collinear;
pub(crate) mod diameter;
pub(crate) mod distance;
pub(crate) mod drag;
pub(crate) mod equal_distance;
pub(crate) mod equal_length;
pub(crate) mod horizontal;
pub(crate) mod parallelogram;
pub(crate) mod perpendicular;
pub(crate) mod point_on_curve;
pub(crate) mod point_on_line;
pub(crate) mod radius;
pub(crate) mod rotational;
pub(crate) mod symmetry;
pub(crate) mod tangent;
pub(crate) mod vertical;

use pyo3::call::PyCallArgs;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use serde_json::Value;

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::draw::{CairoLike, ConstraintStatus, DrawStyle};

/// Type of a leaf-module registration function.
type LeafRegister = fn(&Bound<'_, PyModule>) -> PyResult<()>;

pub(crate) const MODULE_DOC: &str = "\
Sketch constraint classes.

One leaf module per constraint type, mirroring the core
sketch::constraints hierarchy: raygeo.sketch.constraints.horizontal,
.distance, .perpendicular, and so on.  Constraint objects carry the
resolved point ids and dimension values of one constraint of a
sketch problem and are passed to raygeo.sketch.Solver.
";

/// Converts a serde JSON value into a Python object (mapping objects
/// become dictionaries).
pub(crate) fn dict_from_value<'py>(
    py: Python<'py>,
    value: &Value,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    if let Value::Object(map) = value {
        for (key, item) in map {
            dict.set_item(key, value_to_py(py, item)?)?;
        }
    }
    Ok(dict)
}

fn value_to_py<'py>(
    py: Python<'py>,
    value: &Value,
) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::types::PyList;
    match value {
        Value::Null => Ok(py.None().into_bound(py)),
        Value::Bool(b) => {
            let owned = (*b).into_pyobject(py)?.to_owned();
            Ok(owned.into_any())
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any())
            } else {
                let f = n.as_f64().unwrap_or(0.0);
                Ok(f.into_pyobject(py)?.into_any())
            }
        }
        Value::String(s) => Ok(s.into_pyobject(py)?.into_any()),
        Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.into_any())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, item)?)?;
            }
            Ok(dict.into_any())
        }
    }
}

/// Maps the [`CairoLike`] interface onto a Python cairo context,
/// remembering the first call error so it can surface after the
/// draw pass.
struct PyCairoAdapter<'a> {
    ctx: Bound<'a, PyAny>,
    error: std::cell::RefCell<Option<PyErr>>,
}

impl<'a> PyCairoAdapter<'a> {
    fn call<A: for<'py> PyCallArgs<'py>>(&self, name: &str, args: A) {
        if self.error.borrow().is_some() {
            return;
        }
        if let Err(err) = self.ctx.call_method1(name, args) {
            *self.error.borrow_mut() = Some(err);
        }
    }
}

impl CairoLike for PyCairoAdapter<'_> {
    fn save(&self) {
        self.call("save", ());
    }

    fn restore(&self) {
        self.call("restore", ());
    }

    fn set_line_width(&self, width: f64) {
        self.call("set_line_width", (width,));
    }

    fn set_dash(&self, on: f64, off: f64) {
        self.call("set_dash", ((on, off),));
    }

    fn set_line_cap_round(&self) {
        self.call("set_line_cap", ("round",));
    }

    fn set_line_join_round(&self) {
        self.call("set_line_join", ("round",));
    }

    fn set_source_rgb(&self, r: f64, g: f64, b: f64) {
        self.call("set_source_rgb", (r, g, b));
    }

    fn set_source_rgba(&self, r: f64, g: f64, b: f64, a: f64) {
        self.call("set_source_rgba", (r, g, b, a));
    }

    fn move_to(&self, x: f64, y: f64) {
        self.call("move_to", (x, y));
    }

    fn line_to(&self, x: f64, y: f64) {
        self.call("line_to", (x, y));
    }

    fn arc(
        &self,
        x: f64,
        y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    ) {
        self.call("arc", (x, y, radius, start_angle, end_angle));
    }

    fn arc_negative(
        &self,
        x: f64,
        y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    ) {
        self.call("arc_negative", (x, y, radius, start_angle, end_angle));
    }

    fn rectangle(&self, x: f64, y: f64, width: f64, height: f64) {
        self.call("rectangle", (x, y, width, height));
    }

    fn fill(&self) {
        self.call("fill", ());
    }

    fn stroke(&self) {
        self.call("stroke", ());
    }

    fn stroke_preserve(&self) {
        self.call("stroke_preserve", ());
    }

    fn new_sub_path(&self) {
        self.call("new_sub_path", ());
    }

    fn new_path(&self) {
        self.call("new_path", ());
    }

    fn get_line_width(&self) -> f64 {
        self.ctx
            .getattr("get_line_width")
            .and_then(|method| method.call0())
            .and_then(|value| value.extract::<f64>())
            .unwrap_or(0.0)
    }

    fn translate(&self, x: f64, y: f64) {
        self.call("translate", (x, y));
    }

    fn rotate(&self, angle: f64) {
        self.call("rotate", (angle,));
    }
}

/// Runs the core [`SketchConstraint::draw`] against the registry
/// behind the Python binding, replaying the symbol onto the caller's
/// cairo context.
#[allow(clippy::too_many_arguments)]
pub(crate) fn constraint_draw(
    constraint: &dyn SketchConstraint,
    ctx: &Bound<'_, PyAny>,
    registry: &PyEntityRegistry,
    to_screen: &Bound<'_, PyAny>,
    is_selected: bool,
    is_hovered: bool,
    point_radius: f64,
    status: u8,
) -> PyResult<()> {
    let style = DrawStyle {
        status: ConstraintStatus::from_ordinal(status),
        is_selected,
        is_hovered,
        point_radius,
    };
    let to_screen = |point: (f64, f64)| {
        to_screen
            .call1((point,))
            .and_then(|result| result.extract::<(f64, f64)>())
            .unwrap_or(point)
    };
    let adapter = PyCairoAdapter {
        ctx: ctx.clone(),
        error: std::cell::RefCell::new(None),
    };
    constraint.draw(&registry.inner, &to_screen, &adapter, &style);
    match adapter.error.into_inner() {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// Runs the core [`SketchConstraint::is_hit`] against the registry
/// behind the Python binding, mapping model coordinates through the
/// identity transform when no Python `to_screen` callable is given.
pub(crate) fn constraint_is_hit(
    constraint: &dyn SketchConstraint,
    mx: f64,
    my: f64,
    threshold: f64,
    registry: &PyEntityRegistry,
    to_screen: Option<&Bound<'_, PyAny>>,
) -> bool {
    match to_screen {
        None => {
            let identity = |point: (f64, f64)| point;
            constraint.is_hit(mx, my, threshold, &registry.inner, &identity)
        }
        Some(function) => {
            let to_screen = |point: (f64, f64)| {
                function
                    .call1((point,))
                    .and_then(|result| result.extract::<(f64, f64)>())
                    .unwrap_or(point)
            };
            constraint.is_hit(mx, my, threshold, &registry.inner, &to_screen)
        }
    }
}

/// Whether the entity references all the given points.
pub(crate) fn entity_contains_points(
    registry: &PyEntityRegistry,
    entity_id: u64,
    points: &[u64],
) -> bool {
    match registry.inner.entity_point_ids(entity_id) {
        Some(ids) => points.iter().all(|point| ids.contains(point)),
        None => false,
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let constraints_mod = PyModule::new(py, "constraints")?;

    constraints_mod.setattr("__doc__", MODULE_DOC)?;

    let leaves: [(&str, LeafRegister); 19] = [
        ("angle", angle::register),
        ("aspect_ratio", aspect_ratio::register),
        ("coincident", coincident::register),
        ("collinear", collinear::register),
        ("diameter", diameter::register),
        ("distance", distance::register),
        ("drag", drag::register),
        ("equal_distance", equal_distance::register),
        ("equal_length", equal_length::register),
        ("horizontal", horizontal::register),
        ("parallelogram", parallelogram::register),
        ("perpendicular", perpendicular::register),
        ("point_on_curve", point_on_curve::register),
        ("point_on_line", point_on_line::register),
        ("radius", radius::register),
        ("rotational", rotational::register),
        ("symmetry", symmetry::register),
        ("tangent", tangent::register),
        ("vertical", vertical::register),
    ];
    for (name, register_leaf) in leaves {
        let leaf_mod = PyModule::new(py, name)?;
        register_leaf(&leaf_mod)?;
        constraints_mod.add_submodule(&leaf_mod)?;
    }

    m.add_submodule(&constraints_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints", &constraints_mod)?;

    Ok(())
}
