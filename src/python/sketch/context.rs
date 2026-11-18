//! Duck-typed registry parameter for geometry and constraint APIs.
//!
//! The entity and constraint methods take the caller's own document
//! object (the application's registry) and read point positions,
//! point flags, and referenced entities through it, mirroring how
//! the reference implementation passes its registry around.  A
//! compiled [`PyEntityRegistry`] is used directly; any other object
//! with the reference registry surface is adapted call-by-call.

use std::borrow::Cow;

use pyo3::prelude::*;
use pyo3_stub_gen::{ImportRef, ModuleRef, PyStubType, TypeInfo};

use crate::sketch::constraint::PointContext;
use crate::sketch::draw::DrawPoint;
use crate::sketch::entities::Entity;

use super::entities::extract_entity;

/// The registry argument of entity and constraint methods: either
/// the compiled document model or the caller's duck-typed registry.
pub struct PointCtx<'py>(pub Bound<'py, PyAny>);

impl<'a, 'py> FromPyObject<'a, 'py> for PointCtx<'py> {
    type Error = PyErr;

    fn extract(ob: pyo3::Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        Ok(PointCtx(ob.to_owned()))
    }
}

impl PyStubType for PointCtx<'_> {
    fn type_output() -> TypeInfo {
        let module = ModuleRef::Named("raygeo.sketch.protocols".to_string());
        TypeInfo {
            name: "Registry".to_string(),
            source_module: Some(module.clone()),
            import: std::collections::HashSet::from([ImportRef::Module(
                module,
            )]),
            type_refs: std::collections::HashMap::new(),
        }
    }
}

/// Read view over a duck-typed Python registry.
struct PythonPointContext<'py> {
    registry: Bound<'py, PyAny>,
}

impl PythonPointContext<'_> {
    /// The point object for an id, or `None`.
    fn point(&self, id: u64) -> Option<Bound<'_, PyAny>> {
        let point = self.registry.call_method1("get_point", (id,)).ok()?;
        if point.is_none() {
            return None;
        }
        Some(point)
    }

    /// The `(x, y)` pair of a point object in reference shape.
    fn xy_of(&self, point: &Bound<'_, PyAny>) -> Option<(f64, f64)> {
        if let Ok((x, y)) = point.extract::<(f64, f64)>() {
            return Some((x, y));
        }
        let x: f64 = point.getattr("x").ok()?.extract().ok()?;
        let y: f64 = point.getattr("y").ok()?.extract().ok()?;
        Some((x, y))
    }

    /// A boolean attribute of a point object, defaulting to false.
    fn flag_of(&self, point: &Bound<'_, PyAny>, name: &str) -> bool {
        point
            .getattr(name)
            .ok()
            .and_then(|flag| flag.extract().ok())
            .unwrap_or(false)
    }
}

impl PointContext for PythonPointContext<'_> {
    fn get_entity(&self, id: u64) -> Option<Box<dyn Entity>> {
        let entity = self.registry.call_method1("get_entity", (id,)).ok()?;
        if entity.is_none() {
            return None;
        }
        extract_entity(&entity).ok()
    }

    fn point_position(&self, id: u64) -> Option<(f64, f64)> {
        let point = self.point(id)?;
        self.xy_of(&point)
    }

    fn get_point(&self, id: u64) -> Option<DrawPoint> {
        let point = self.point(id)?;
        let (x, y) = self.xy_of(&point)?;
        Some(DrawPoint {
            x,
            y,
            fixed: self.flag_of(&point, "fixed"),
            constrained: self.flag_of(&point, "constrained"),
        })
    }

    fn set_xy(&mut self, id: u64, x: f64, y: f64) {
        let Some(point) = self.point(id) else {
            return;
        };
        let _ = point.setattr("x", x);
        let _ = point.setattr("y", y);
    }

    fn entity_ids(&self) -> Vec<u64> {
        if let Ok(ids) = self.registry.call_method0("entity_ids") {
            if let Ok(ids) = ids.extract::<Vec<u64>>() {
                return ids;
            }
        }
        // The reference registry exposes its entities as a list
        // attribute instead of a method.
        let entities = match self.registry.getattr("entities") {
            Ok(entities) => entities,
            Err(_) => return vec![],
        };
        let Ok(iter) = entities.try_iter() else {
            return vec![];
        };
        iter.flatten()
            .filter_map(|entity| extract_entity(&entity).ok())
            .map(|entity| entity.base().id)
            .collect()
    }

    fn entity_type(&self, id: u64) -> Option<Cow<'static, str>> {
        let entity = self.get_entity(id)?;
        Some(Cow::Owned(entity.type_name().to_string()))
    }

    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>> {
        Some(self.get_entity(id)?.get_point_ids())
    }
}

impl<'py> PointCtx<'py> {
    /// The compiled registry behind the binding, if the caller
    /// passed one.
    fn compiled(
        &self,
    ) -> Option<PyRef<'py, super::registry::PyEntityRegistry>> {
        self.0.extract().ok()
    }
}

impl PointContext for PointCtx<'_> {
    fn get_entity(&self, id: u64) -> Option<Box<dyn Entity>> {
        if let Some(registry) = self.compiled() {
            return registry.inner.get_entity(id).map(|e| e.clone_box());
        }
        PythonPointContext {
            registry: self.0.clone(),
        }
        .get_entity(id)
    }

    fn point_position(&self, id: u64) -> Option<(f64, f64)> {
        if let Some(registry) = self.compiled() {
            return registry.inner.point_position(id);
        }
        PythonPointContext {
            registry: self.0.clone(),
        }
        .point_position(id)
    }

    fn get_point(&self, id: u64) -> Option<DrawPoint> {
        if let Some(registry) = self.compiled() {
            return registry.inner.get_point(id).map(|point| DrawPoint {
                x: point.x,
                y: point.y,
                fixed: point.fixed,
                constrained: point.constrained,
            });
        }
        PythonPointContext {
            registry: self.0.clone(),
        }
        .get_point(id)
    }

    fn set_xy(&mut self, id: u64, x: f64, y: f64) {
        if let Ok(mut registry) = self
            .0
            .extract::<PyRefMut<'_, super::registry::PyEntityRegistry>>()
        {
            registry.inner.set_xy(id, x, y);
            return;
        }
        PythonPointContext {
            registry: self.0.clone(),
        }
        .set_xy(id, x, y);
    }

    fn entity_ids(&self) -> Vec<u64> {
        if let Some(registry) = self.compiled() {
            return registry.inner.entity_ids();
        }
        PythonPointContext {
            registry: self.0.clone(),
        }
        .entity_ids()
    }

    fn entity_type(&self, id: u64) -> Option<Cow<'static, str>> {
        if let Some(registry) = self.compiled() {
            return registry
                .inner
                .entity_type(id)
                .map(|name| Cow::Owned(name.to_string()));
        }
        PythonPointContext {
            registry: self.0.clone(),
        }
        .entity_type(id)
    }

    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>> {
        if let Some(registry) = self.compiled() {
            return registry.inner.entity_point_ids(id);
        }
        PythonPointContext {
            registry: self.0.clone(),
        }
        .entity_point_ids(id)
    }
}
