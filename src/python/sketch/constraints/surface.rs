//! Reference-contract surface shared by the constraint bindings:
//! gettext helpers, value/coordinate formatting, selection
//! resolution, and the attribute walks of the reference base class.

use std::collections::HashSet;

use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use crate::sketch::entities::Entity;

use crate::python::sketch::entities::extract_entity;

/// Translates a message through Python's `gettext`, returning the
/// input unchanged when translation is unavailable.
pub fn gettext(py: Python<'_>, message: &str) -> String {
    py.import("gettext")
        .and_then(|module| module.getattr("gettext"))
        .and_then(|gettext| gettext.call1((message,)))
        .and_then(|result| result.extract())
        .unwrap_or_else(|_| message.to_string())
}

/// Translates a message and interpolates the given fragments into
/// the `{}` placeholders in order (the reference
/// `_("...").format(...)` pattern).
pub fn gettext_format(
    py: Python<'_>,
    message: &str,
    fragments: &[&str],
) -> String {
    let mut translated = gettext(py, message);
    for fragment in fragments {
        translated = translated.replacen("{}", fragment, 1);
    }
    translated
}

/// Formats a dimensional value like the reference `_format_value`
/// (`f"{value:.1f}"`).
pub fn format_value(value: f64) -> String {
    format!("{:.1}", value)
}

/// Formats a coordinate pair through Python's `locale.format_string`
/// with grouping, mirroring the reference `_format_coord`.
pub fn format_coord(py: Python<'_>, x: f64, y: f64) -> String {
    py.import("locale")
        .and_then(|locale| {
            locale.call_method1("format_string", ("%.1f/%.1f", (x, y), true))
        })
        .and_then(|result| result.extract())
        .unwrap_or_else(|_| {
            let x_str = format!("{:.1}", x);
            let y_str = format!("{:.1}", y);
            format!("{}/{}", x_str, y_str)
        })
}

/// The reference `type(entity).__name__` label: the entity's type
/// name with a capitalized first letter (`"line"` → `"Line"`).
pub fn entity_label(entity: &dyn Entity) -> String {
    let name = entity.type_name();
    let mut label = name.to_string();
    if let Some(first) = label.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    label
}

/// The `(point_ids, entity_ids)` lists of a duck-typed selection.
pub fn selection_ids(
    selection: &Bound<'_, PyAny>,
) -> PyResult<(Vec<i64>, Vec<i64>)> {
    let point_ids: Vec<i64> = selection.getattr("point_ids")?.extract()?;
    let entity_ids: Vec<i64> = selection.getattr("entity_ids")?.extract()?;
    Ok((point_ids, entity_ids))
}

/// The resolved-entity shape shared by the `applies_to_entities`
/// predicates: entities the bindings cannot represent come back as
/// `None`.
pub type ResolvedEntities = Vec<Option<Box<dyn Entity>>>;

/// Resolves the selected entities the way the reference
/// `can_apply_to` implementations do: through the selection's
/// `resolve_entities(sketch.registry if sketch else None)`.
/// Entities the bindings cannot represent come back as `None` so
/// the applicability predicates can reject them.
pub fn resolve_entities(
    selection: &Bound<'_, PyAny>,
    sketch: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<ResolvedEntities>> {
    let registry = match sketch {
        Some(sketch) => sketch.getattr("registry").ok(),
        None => None,
    };
    let resolved = selection.call_method1("resolve_entities", (registry,))?;
    if resolved.is_none() {
        return Ok(None);
    }
    let list: Vec<Bound<'_, PyAny>> = resolved.extract()?;
    let mut entities = Vec::with_capacity(list.len());
    for object in &list {
        let entity = extract_entity(object).ok();
        entities.push(entity);
    }
    Ok(Some(entities))
}

/// Extracts an iterable of entity objects into the resolved-entity
/// shape used by the `applies_to_entities` predicates.
pub fn entities_from_py(
    entities: &Bound<'_, PyAny>,
) -> PyResult<ResolvedEntities> {
    let mut resolved = Vec::new();
    for object in entities.try_iter()? {
        resolved.push(extract_entity(&object?).ok());
    }
    Ok(resolved)
}

/// The human-readable type name, dispatched through the object's
/// own (possibly overridden) `get_type_name`.
pub fn type_name(slf: &Bound<'_, PyAny>) -> PyResult<String> {
    slf.call_method0("get_type_name")?.extract()
}

/// The reference `get_title` of the dimensional constraints:
/// `"<type name> <value:.1f><suffix>"`.
pub fn title_value(slf: &Bound<'_, PyAny>, suffix: &str) -> PyResult<String> {
    let value: f64 = slf.getattr("value")?.extract()?;
    Ok(format!(
        "{} {}{}",
        type_name(slf)?,
        format_value(value),
        suffix
    ))
}

/// The plain `get_title` of the non-dimensional constraints: just
/// the type name.
pub fn title_plain(slf: &Bound<'_, PyAny>) -> PyResult<String> {
    type_name(slf)
}

/// The `get_title` of the aspect-ratio constraint:
/// `"<type name> <ratio:.2f>"`.
pub fn title_ratio(slf: &Bound<'_, PyAny>) -> PyResult<String> {
    let ratio: f64 = slf.getattr("ratio")?.extract()?;
    Ok(format!("{} {:.2}", type_name(slf)?, ratio))
}

/// The `get_title` of the rotational constraint: the value
/// interpreted as radians, shown in degrees.
pub fn title_degrees(slf: &Bound<'_, PyAny>) -> PyResult<String> {
    let value: f64 = slf.getattr("value")?.extract()?;
    let degrees = value * 180.0 / std::f64::consts::PI;
    Ok(format!("{} {:.1}\u{00b0}", type_name(slf)?, degrees))
}

/// The subtitle body of the point-pair constraints:
/// `_("From {} to {}")` over the two points' coordinates.
pub fn subtitle_from_to(
    py: Python<'_>,
    registry: &dyn Fn(i64) -> Option<(f64, f64)>,
    first: i64,
    second: i64,
) -> String {
    let (Some(p1), Some(p2)) = (registry(first), registry(second)) else {
        return String::new();
    };
    gettext_format(
        py,
        "From {} to {}",
        &[&format_coord(py, p1.0, p1.1), &format_coord(py, p2.0, p2.1)],
    )
}

/// The subtitle body of the coincident constraint: `_("At {}")` over
/// the first point's coordinate.
pub fn subtitle_at(
    py: Python<'_>,
    registry: &dyn Fn(i64) -> Option<(f64, f64)>,
    point_id: i64,
) -> String {
    let Some(point) = registry(point_id) else {
        return String::new();
    };
    gettext_format(py, "At {}", &[&format_coord(py, point.0, point.1)])
}

/// The subtitle body of the point-on-shape constraints:
/// `_("Point at {}")` over the constrained point's coordinate.
pub fn subtitle_point_at(
    py: Python<'_>,
    registry: &dyn Fn(i64) -> Option<(f64, f64)>,
    point_id: i64,
) -> String {
    let Some(point) = registry(point_id) else {
        return String::new();
    };
    gettext_format(py, "Point at {}", &[&format_coord(py, point.0, point.1)])
}

/// The reference base's `get_referenced_point_ids` attribute walk.
pub fn referenced_point_ids(slf: &Bound<'_, PyAny>) -> HashSet<i64> {
    let mut ids = HashSet::new();
    for name in ["p1", "p2", "p3", "p4", "center", "point_id"] {
        let Ok(value) = slf.getattr(name) else {
            continue;
        };
        if value.is_none() {
            continue;
        }
        if let Ok(pid) = value.extract::<i64>() {
            ids.insert(pid);
        }
    }
    ids
}

/// The reference base's `get_referenced_entity_ids` attribute walk.
pub fn referenced_entity_ids(slf: &Bound<'_, PyAny>) -> HashSet<i64> {
    let mut ids = HashSet::new();
    for name in [
        "e1_id",
        "e2_id",
        "line_id",
        "shape_id",
        "entity_id",
        "circle_id",
        "axis",
    ] {
        let Ok(value) = slf.getattr(name) else {
            continue;
        };
        if value.is_none() {
            continue;
        }
        if let Ok(eid) = value.extract::<i64>() {
            ids.insert(eid);
        }
    }
    if let Ok(values) = slf.getattr("entity_ids") {
        if let Ok(iter) = values.try_iter() {
            for value in iter.flatten() {
                if let Ok(eid) = value.extract::<i64>() {
                    ids.insert(eid);
                }
            }
        }
    }
    ids
}

/// The attribute names of the reference constraint surface, in the
/// order the reference `vars()` walks see them.
pub const REFERENCE_ATTRS: &[&str] = &[
    "p1",
    "p2",
    "p3",
    "p4",
    "center",
    "point_id",
    "e1_id",
    "e2_id",
    "line_id",
    "shape_id",
    "entity_id",
    "circle_id",
    "axis",
    "e1_far_idx",
    "e2_far_idx",
    "p_origin",
    "p_width",
    "p_height",
    "entity_ids",
    "value",
    "ratio",
    "weight",
    "target_x",
    "target_y",
    "expression",
    "status",
    "user_visible",
];

/// The reference base's `depends_on_points`: intersection of the
/// referenced point ids (dispatched through the override) with the
/// given id collection (a set or sequence in the reference).
pub fn depends_on_points(
    slf: &Bound<'_, PyAny>,
    point_ids: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let referenced: HashSet<i64> =
        slf.call_method0("get_referenced_point_ids")?.extract()?;
    for item in point_ids.try_iter()? {
        let pid: i64 = item?.extract()?;
        if referenced.contains(&pid) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The reference base's `depends_on_entities`, dispatched through
/// the `get_referenced_entity_ids` override.
pub fn depends_on_entities(
    slf: &Bound<'_, PyAny>,
    entity_ids: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let referenced: HashSet<i64> =
        slf.call_method0("get_referenced_entity_ids")?.extract()?;
    for item in entity_ids.try_iter()? {
        let eid: i64 = item?.extract()?;
        if referenced.contains(&eid) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Deep-copies the constraint through its reference dictionary
/// shape, rebuilding via the concrete class's `from_dict`.
pub fn deepcopy<'py>(
    slf: &Bound<'py, PyAny>,
    memo: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let _ = memo;
    let dict = slf.call_method0("to_dict")?;
    let from_dict = slf.get_type().getattr("from_dict")?;
    from_dict.call1((dict,))
}

/// The reference base's `update_from_context`: re-evaluates the
/// expression against the context and updates `value`/`status`.
/// Evaluation failures keep the previous value and set the error
/// status; the reference catches `ValueError`, `SyntaxError`,
/// `NameError`, and `TypeError`.
pub fn update_from_context(
    slf: &Bound<'_, PyAny>,
    context: &Bound<'_, PyDict>,
) {
    let py = context.py();
    let expression: Option<String> = match slf.getattr("expression") {
        Ok(value) => match value.extract() {
            Ok(expression) => expression,
            Err(_) => return,
        },
        Err(_) => return,
    };
    let Some(expression) = expression else {
        let _ = slf.setattr("status", 1);
        return;
    };
    let namespace = PyDict::new(py);
    if let Ok(math) = py.import("math") {
        if let Ok(math_dict) = math.getattr("__dict__") {
            if let Ok(dict) = math_dict.cast::<PyDict>() {
                for (key, value) in dict.iter() {
                    let Ok(name) = key.extract::<String>() else {
                        continue;
                    };
                    if name.starts_with('_') {
                        continue;
                    }
                    let _ = namespace.set_item(key, value);
                }
            }
        }
    }
    for (key, value) in context.iter() {
        let _ = namespace.set_item(key, value);
    }
    let _ = namespace.set_item("__builtins__", PyDict::new(py));
    let result = py
        .import("builtins")
        .and_then(|builtins| builtins.getattr("eval"))
        .and_then(|eval| eval.call1((expression, &namespace, &namespace)))
        .and_then(|value| value.extract::<f64>());
    match result {
        Ok(value) => {
            let _ = slf.setattr("value", value);
            let _ = slf.setattr("status", 2);
        }
        Err(_) => {
            let _ = slf.setattr("status", 3);
        }
    }
}

/// Raises `NotImplementedError` like the reference base's
/// `get_type_name`.
pub fn type_name_unimplemented() -> PyErr {
    PyNotImplementedError::new_err(
        "Returns to human-readable name of this constraint type.",
    )
}
