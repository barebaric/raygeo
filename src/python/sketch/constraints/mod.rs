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
pub(crate) mod surface;
pub(crate) mod symmetry;
pub(crate) mod tangent;
pub(crate) mod vertical;

use pyo3::call::PyCallArgs;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, ScreenMap};
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::draw::{
    CairoLike, ConstraintStatus, DrawContext as DrawContextContract, DrawPoint,
    DrawStyle, TextExtents,
};

/// Type of a leaf-module registration function.
type LeafRegister = fn(&Bound<'_, PyModule>) -> PyResult<()>;

/// Solver weight of the angle constraints, mirroring the reference
/// `angle.py` constant.
pub(crate) const ANGLE_WEIGHT: f64 = 10.0;

// The reference package exposes every constraint class and the
// solver constants at package level; declare the same surface for
// the generated stubs.
pyo3_stub_gen::module_variable!(
    "raygeo.sketch.constraints",
    "ANGLE_WEIGHT",
    f64,
    10.0
);

pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.angle";
    "AngleConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.aspect_ratio";
    "AspectRatioConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.coincident";
    "CoincidentConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.collinear";
    "CollinearConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.diameter";
    "DiameterConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.distance";
    "DistanceConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.drag";
    "DragConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints"
    from "raygeo.sketch.constraints.equal_distance";
    "EqualDistanceConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.equal_length";
    "EqualLengthConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.horizontal";
    "HorizontalConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints"
    from "raygeo.sketch.constraints.parallelogram";
    "ParallelogramConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints"
    from "raygeo.sketch.constraints.perpendicular";
    "PerpendicularConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints"
    from "raygeo.sketch.constraints.point_on_curve";
    "PointOnCurveConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.point_on_line";
    "PointOnLineConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.radius";
    "RadiusConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.rotational";
    "RotationalConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.symmetry";
    "SymmetryConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.tangent";
    "TangentConstraint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.constraints" from "raygeo.sketch.constraints.vertical";
    "VerticalConstraint"
);

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

pub(crate) fn value_to_py<'py>(
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

/// Maps the [`DrawContext`] protocol onto a Python object: the
/// caller's own document (or any duck-typed stand-in) reads through
/// its methods, and the first call error surfaces after the draw
/// pass.
struct PyDrawContext<'a> {
    registry: Bound<'a, PyAny>,
    error: std::cell::RefCell<Option<PyErr>>,
}

impl<'a> PyDrawContext<'a> {
    fn call(&self, name: &str, id: u64) -> Option<Bound<'a, PyAny>> {
        if self.error.borrow().is_some() {
            return None;
        }
        match self.registry.call_method1(name, (id,)) {
            Ok(value) => Some(value),
            Err(err) => {
                *self.error.borrow_mut() = Some(err);
                None
            }
        }
    }

    /// Probes an optional registry method: missing methods yield
    /// `None` without recording an error.
    fn try_call(&self, name: &str, id: u64) -> Option<Bound<'a, PyAny>> {
        if self.error.borrow().is_some() {
            return None;
        }
        self.registry.call_method1(name, (id,)).ok()
    }
}

impl DrawContextContract for PyDrawContext<'_> {
    fn get_point(&self, id: u64) -> Option<DrawPoint> {
        let value = self.call("get_point", id)?;
        let xy: (f64, f64) = match value.extract::<(f64, f64)>() {
            Ok(pair) => pair,
            Err(_) => {
                let x = value.getattr("x").ok()?.extract().ok()?;
                let y = value.getattr("y").ok()?.extract().ok()?;
                (x, y)
            }
        };
        let fixed = match self.try_call("point_is_fixed", id) {
            Some(flag) => flag.extract::<bool>().unwrap_or(false),
            None => value
                .getattr("fixed")
                .ok()
                .and_then(|flag| flag.extract().ok())
                .unwrap_or(false),
        };
        let constrained = match self.try_call("point_constrained", id) {
            Some(flag) => flag.extract::<bool>().unwrap_or(false),
            None => value
                .getattr("constrained")
                .ok()
                .and_then(|flag| flag.extract().ok())
                .unwrap_or(false),
        };
        Some(DrawPoint {
            x: xy.0,
            y: xy.1,
            fixed,
            constrained,
        })
    }

    fn entity_ids(&self) -> Vec<u64> {
        if let Ok(ids) = self
            .registry
            .call_method0("entity_ids")
            .and_then(|ids| ids.extract::<Vec<u64>>())
        {
            return ids;
        }
        // The reference registry exposes its entities as a list
        // attribute instead of a method.
        let entities = match self.registry.getattr("entities") {
            Ok(entities) => entities,
            Err(_) => return vec![],
        };
        let iter = match entities.try_iter() {
            Ok(iter) => iter,
            Err(_) => return vec![],
        };
        let mut ids = Vec::new();
        for entity in iter.flatten() {
            if let Ok(id) = entity.getattr("id").and_then(|v| v.extract()) {
                ids.push(id);
            }
        }
        ids
    }

    fn entity_type(&self, id: u64) -> Option<std::borrow::Cow<'static, str>> {
        if let Some(value) = self.try_call("entity_type", id) {
            if let Ok(name) = value.extract::<String>() {
                return Some(std::borrow::Cow::Owned(name));
            }
        }
        let entity = self.call("get_entity", id)?;
        let name: String =
            entity.getattr("type").ok().and_then(|v| v.extract().ok())?;
        Some(std::borrow::Cow::Owned(name))
    }

    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>> {
        if let Some(value) = self.try_call("entity_point_ids", id) {
            if let Ok(ids) = value.extract::<Vec<u64>>() {
                return Some(ids);
            }
        }
        let entity = self.call("get_entity", id)?;
        entity.call_method0("get_point_ids").ok()?.extract().ok()
    }

    fn point_in_text_box_frame(&self, point_id: u64) -> bool {
        match self.try_call("point_is_in_text_box_frame", point_id) {
            Some(flag) => flag.extract::<bool>().unwrap_or(false),
            None => false,
        }
    }

    fn entity_arc_midpoint(&self, id: u64) -> Option<(f64, f64)> {
        let registry = self
            .registry
            .extract::<PyRef<'_, super::registry::PyEntityRegistry>>()
            .ok()?;
        let arc = registry
            .inner
            .get_entity(id)?
            .as_any()
            .downcast_ref::<crate::sketch::entities::arc::Arc>()?;
        arc.get_midpoint(&registry.inner)
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
        self.call("set_line_cap", (1,));
    }

    fn set_line_join_round(&self) {
        self.call("set_line_join", (1,));
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

    fn text_extents(&self, text: &str, font_px: f64) -> Option<TextExtents> {
        if self.error.borrow().is_some() {
            return None;
        }
        let py = self.ctx.py();
        let result = (|| -> PyResult<TextExtents> {
            let (pango, pango_cairo) = host_text_stack(py)?;
            let layout = create_text_layout(
                &pango,
                &pango_cairo,
                &self.ctx,
                text,
                font_px,
            )?;
            let extents = layout.call_method0("get_pixel_extents")?;
            let ink = extents.get_item(0)?;
            let x: f64 = ink.getattr("x")?.extract()?;
            let y: f64 = ink.getattr("y")?.extract()?;
            let width: f64 = ink.getattr("width")?.extract()?;
            let height: f64 = ink.getattr("height")?.extract()?;
            Ok((x, y, width, height))
        })();
        result.ok()
    }

    fn show_text(&self, text: &str, font_px: f64) -> bool {
        if self.error.borrow().is_some() {
            return false;
        }
        let py = self.ctx.py();
        let result = (|| -> PyResult<()> {
            let (pango, pango_cairo) = host_text_stack(py)?;
            let layout = create_text_layout(
                &pango,
                &pango_cairo,
                &self.ctx,
                text,
                font_px,
            )?;
            pango_cairo.call_method1("show_layout", (&self.ctx, &layout))?;
            Ok(())
        })();
        match result {
            Ok(()) => true,
            Err(err) => {
                *self.error.borrow_mut() = Some(err);
                false
            }
        }
    }
}

/// The host `gi.repository` text stack: `(Pango, PangoCairo)`.
fn host_text_stack(
    py: Python<'_>,
) -> PyResult<(Bound<'_, PyAny>, Bound<'_, PyAny>)> {
    let gi = py.import("gi.repository")?;
    let pango = gi.getattr("Pango")?;
    let pango_cairo = gi.getattr("PangoCairo")?;
    Ok((pango, pango_cairo))
}

/// Builds the reference `create_text_layout` equivalent: a Pango
/// layout with the Sans font at the given absolute pixel size.
fn create_text_layout<'py>(
    pango: &Bound<'py, PyAny>,
    pango_cairo: &Bound<'py, PyAny>,
    ctx: &Bound<'py, PyAny>,
    text: &str,
    font_px: f64,
) -> PyResult<Bound<'py, PyAny>> {
    let layout = pango_cairo.call_method1("create_layout", (ctx,))?;
    let font = pango.getattr("FontDescription")?.call0()?;
    font.call_method1("set_family", ("Sans",))?;
    let weight = pango.getattr("Weight")?.getattr("NORMAL")?;
    font.call_method1("set_weight", (weight,))?;
    let style = pango.getattr("Style")?.getattr("NORMAL")?;
    font.call_method1("set_style", (style,))?;
    let scale: f64 = pango.getattr("SCALE")?.extract()?;
    font.call_method1("set_absolute_size", (font_px * scale,))?;
    layout.call_method1("set_font_description", (&font,))?;
    layout.call_method1("set_text", (text, -1))?;
    Ok(layout)
}

/// Runs a draw helper against the caller's cairo context,
/// surfacing the first recorded cairo error after the pass.
pub(crate) fn draw_with_cairo(
    ctx: &Canvas<'_>,
    draw: impl FnOnce(&dyn CairoLike),
) -> PyResult<()> {
    let adapter = PyCairoAdapter {
        ctx: ctx.0.clone(),
        error: std::cell::RefCell::new(None),
    };
    draw(&adapter);
    match adapter.error.into_inner() {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// Runs the core [`SketchConstraint::draw`] against the registry
/// behind the Python binding, replaying the symbol onto the caller's
/// cairo context.
#[allow(clippy::too_many_arguments)]
pub(crate) fn constraint_draw(
    constraint: &dyn SketchConstraint,
    ctx: &Canvas<'_>,
    registry: &Bound<'_, PyAny>,
    to_screen: &ScreenMap<'_>,
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
            .0
            .call1((point,))
            .and_then(|result| result.extract::<(f64, f64)>())
            .unwrap_or(point)
    };
    let canvas = PyCairoAdapter {
        ctx: ctx.0.clone(),
        error: std::cell::RefCell::new(None),
    };
    let context = PyDrawContext {
        registry: registry.clone(),
        error: std::cell::RefCell::new(None),
    };
    constraint.draw(&context, &to_screen, &canvas, &style);
    let error = canvas
        .error
        .into_inner()
        .or_else(|| context.error.into_inner());
    match error {
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
    registry: &PointCtx<'_>,
    to_screen: Option<&ScreenMap<'_>>,
) -> bool {
    match to_screen {
        None => {
            let identity = |point: (f64, f64)| point;
            constraint.is_hit(mx, my, threshold, registry, &identity)
        }
        Some(function) => {
            let to_screen = |point: (f64, f64)| {
                function
                    .0
                    .call1((point,))
                    .and_then(|result| result.extract::<(f64, f64)>())
                    .unwrap_or(point)
            };
            constraint.is_hit(mx, my, threshold, registry, &to_screen)
        }
    }
}

/// Runs the core [`SketchConstraint::gradient`] against the
/// registry, returning the reference mapping shape
/// `{point_id: [(d_error_dx, d_error_dy), ...]}` with one entry per
/// scalar residual row.
pub(crate) fn constraint_gradient(
    constraint: &dyn SketchConstraint,
    registry: &mut PointCtx<'_>,
) -> HashMap<u64, Vec<(f64, f64)>> {
    let total_rows = constraint.residual_count(registry);
    let entries = constraint.gradient(registry);
    let mut map: HashMap<u64, Vec<(f64, f64)>> = HashMap::new();
    for (point_id, row, dx, dy) in entries {
        let rows = map.entry(point_id).or_default();
        if rows.len() <= row {
            rows.resize(row + 1, (0.0, 0.0));
        }
        let (ox, oy) = rows[row];
        rows[row] = (ox + dx, oy + dy);
    }
    // The reference mapping gives every referenced point a list as
    // long as the constraint's residual count.
    for rows in map.values_mut() {
        if rows.len() < total_rows {
            rows.resize(total_rows, (0.0, 0.0));
        }
    }
    map
}

/// Rewrites one id in place through *map*.
pub(crate) fn remap_id(
    value: &mut i64,
    map: &std::collections::HashMap<i64, i64>,
) {
    if let Some(new) = map.get(value) {
        *value = *new;
    }
}

/// Rewrites an optional id in place through *map*.
pub(crate) fn remap_opt(
    value: &mut Option<i64>,
    map: &std::collections::HashMap<i64, i64>,
) {
    if let Some(current) = value {
        if let Some(new) = map.get(current) {
            *value = Some(*new);
        }
    }
}

/// Rewrites a list of ids in place through *map*.
pub(crate) fn remap_ids(
    values: &mut [i64],
    map: &std::collections::HashMap<i64, i64>,
) {
    for value in values {
        if let Some(new) = map.get(value) {
            *value = *new;
        }
    }
}

/// The common base class of every constraint binding, carrying the
/// reference `Constraint` base surface.  The per-class members live
/// on the subclasses; polymorphism is carried by the Rust-side
/// `SketchConstraint` trait.
#[gen_stub_pyclass]
#[pyclass(
    get_all,
    set_all,
    skip_from_py_object,
    subclass,
    module = "raygeo.sketch.constraints",
    name = "Constraint"
)]
#[derive(Default, Clone)]
pub struct PyConstraintBase {
    /// The resolved dimension value (dimensional constraints).
    pub value: f64,
    /// The stored expression string (dimensional constraints).
    pub expression: Option<String>,
    /// The reference validation status ordinal.
    pub status: u8,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyConstraintBase {
    /// Rewrites every id reference through *id_map*; concrete
    /// constraints override this.
    fn remap_references(&mut self, _id_map: HashMap<i64, i64>) {}

    /// Whether the constraint can be applied to the selection.
    #[classmethod]
    #[pyo3(signature = (_selection, _sketch = None))]
    fn can_apply_to(
        _cls: &Bound<'_, PyType>,
        _selection: &Bound<'_, PyAny>,
        _sketch: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }

    /// Whether the constraint type can apply to the given entities.
    #[classmethod]
    fn applies_to_entities(
        _cls: &Bound<'_, PyType>,
        entities: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let count = entities.try_iter()?.count();
        Ok(count == 0)
    }

    /// The string key used to identify this constraint type; None
    /// for constraints that cannot be created by users.
    #[classmethod]
    fn get_type_key(_cls: &Bound<'_, PyType>) -> Option<&'static str> {
        None
    }

    /// The human-readable name of this constraint type.
    #[staticmethod]
    fn get_type_name() -> PyResult<String> {
        Err(surface::type_name_unimplemented())
    }

    /// A human-readable title for this constraint.
    fn get_title(slf: &Bound<'_, Self>) -> PyResult<String> {
        surface::type_name(slf.as_any())
    }

    /// A human-readable subtitle describing the constrained
    /// entities.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let _ = (py, registry);
        String::new()
    }

    /// The user-facing hint for the constraint edit dialog.
    fn get_edit_subtitle(&self, py: Python<'_>) -> String {
        surface::gettext(py, "Enter value or expression.")
    }

    /// Whether this constraint restricts the length/distance of the
    /// segment (p1, p2) or of the given entity.
    #[pyo3(signature = (p1, p2, entity_id = None))]
    fn targets_segment(
        &self,
        p1: i64,
        p2: i64,
        entity_id: Option<i64>,
    ) -> bool {
        let _ = (p1, p2, entity_id);
        false
    }

    /// The error of the constraint at the registry's positions.
    #[pyo3(signature = (reg, params = None))]
    fn error(&self, reg: PointCtx<'_>, params: Option<Py<PyAny>>) -> f64 {
        let _ = (reg, params);
        0.0
    }

    /// The partial derivatives of the error by point id.
    #[pyo3(signature = (reg, params = None))]
    fn gradient(
        &self,
        reg: PointCtx<'_>,
        params: Option<Py<PyAny>>,
    ) -> HashMap<u64, Vec<(f64, f64)>> {
        let _ = (reg, params);
        HashMap::new()
    }

    /// Whether the constraint explicitly defines the radius/length
    /// of the given entity.
    fn constrains_radius(
        &self,
        registry: PointCtx<'_>,
        entity_id: i64,
    ) -> bool {
        let _ = (registry, entity_id);
        false
    }

    /// Serializes the constraint to a dictionary.
    fn to_dict<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        PyDict::new(py)
    }

    /// Whether the constraint's visual representation is hit.
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
        let _ = (sx, sy, registry, to_screen, element, threshold);
        false
    }

    /// Draws the constraint's visual representation.
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
    ) {
        let _ = (
            ctx,
            registry,
            to_screen,
            is_selected,
            is_hovered,
            point_radius,
        );
    }

    /// Re-evaluates the expression (if present) using the provided
    /// context and updates `value` and `status`.
    fn update_from_context(slf: &Bound<'_, Self>, context: &Bound<'_, PyDict>) {
        surface::update_from_context(slf.as_any(), context)
    }

    /// Whether the constraint references any of the given points.
    fn depends_on_points(
        slf: &Bound<'_, Self>,
        point_ids: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        surface::depends_on_points(slf.as_any(), point_ids)
    }

    /// Whether the constraint references any of the given entities.
    fn depends_on_entities(
        slf: &Bound<'_, Self>,
        entity_ids: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        surface::depends_on_entities(slf.as_any(), entity_ids)
    }

    /// The set of point ids this constraint references.
    fn get_referenced_point_ids(slf: &Bound<'_, Self>) -> HashSet<i64> {
        surface::referenced_point_ids(slf.as_any())
    }

    /// The set of entity ids this constraint references.
    fn get_referenced_entity_ids(slf: &Bound<'_, Self>) -> HashSet<i64> {
        surface::referenced_entity_ids(slf.as_any())
    }

    /// Whether the constraint survives a mirror of all referenced
    /// geometry.
    fn is_mirror_compatible(&self) -> bool {
        true
    }

    /// Updates the internal state for a mirror transform; a no-op
    /// for mirror-invariant constraints.
    fn mirror(&self, _axis: &Bound<'_, PyAny>) {}

    /// Whether the constraint pins geometry to the global axes.
    fn is_world_anchored(&self) -> bool {
        false
    }

    /// A point id that can be dragged to manipulate this constraint.
    fn get_draggable_point(&self) -> Option<i64> {
        None
    }

    /// The reference attribute view of the constraint: the
    /// `vars()` walk of the reference implementation sees exactly
    /// these attributes.
    #[getter]
    fn __dict__(slf: &Bound<'_, Self>, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for name in surface::REFERENCE_ATTRS {
            if let Ok(value) = slf.getattr(name) {
                dict.set_item(name, value)?;
            }
        }
        Ok(dict.unbind())
    }

    /// Deep-copies the constraint through its reference dictionary
    /// shape, rebuilding via the concrete subclass's `from_dict`.
    fn __deepcopy__<'py>(
        slf: &Bound<'py, Self>,
        memo: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        surface::deepcopy(slf.as_any(), memo)
    }

    fn __copy__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        Self::__deepcopy__(slf, &PyDict::new(slf.py()))
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let constraints_mod = PyModule::new(py, "constraints")?;

    constraints_mod.setattr("__doc__", MODULE_DOC)?;

    constraints_mod.add_class::<PyConstraintBase>()?;
    constraints_mod.add("ANGLE_WEIGHT", ANGLE_WEIGHT)?;

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

    // The reference package exposes every constraint class at
    // package level as well (`from .angle import AngleConstraint`).
    constraints_mod.add_class::<angle::PyAngleConstraint>()?;
    constraints_mod.add_class::<aspect_ratio::PyAspectRatioConstraint>()?;
    constraints_mod.add_class::<coincident::PyCoincidentConstraint>()?;
    constraints_mod.add_class::<collinear::PyCollinearConstraint>()?;
    constraints_mod.add_class::<diameter::PyDiameterConstraint>()?;
    constraints_mod.add_class::<distance::PyDistanceConstraint>()?;
    constraints_mod.add_class::<drag::PyDragConstraint>()?;
    constraints_mod.add_class::<equal_distance::PyEqualDistanceConstraint>()?;
    constraints_mod.add_class::<equal_length::PyEqualLengthConstraint>()?;
    constraints_mod.add_class::<horizontal::PyHorizontalConstraint>()?;
    constraints_mod.add_class::<parallelogram::PyParallelogramConstraint>()?;
    constraints_mod.add_class::<perpendicular::PyPerpendicularConstraint>()?;
    constraints_mod.add_class::<point_on_curve::PyPointOnCurveConstraint>()?;
    constraints_mod.add_class::<point_on_line::PyPointOnLineConstraint>()?;
    constraints_mod.add_class::<radius::PyRadiusConstraint>()?;
    constraints_mod.add_class::<rotational::PyRotationalConstraint>()?;
    constraints_mod.add_class::<symmetry::PySymmetryConstraint>()?;
    constraints_mod.add_class::<tangent::PyTangentConstraint>()?;
    constraints_mod.add_class::<vertical::PyVerticalConstraint>()?;

    m.add_submodule(&constraints_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints", &constraints_mod)?;

    Ok(())
}
