//! Duck-typed parameter surfaces for the symbol rendering APIs.
//!
//! The generated stubs type these parameters against the runtime
//! protocols in `raygeo.sketch.protocols`; any object satisfying the
//! protocol is accepted at runtime.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3_stub_gen::{ImportRef, ModuleRef, PyStubType, TypeInfo};

fn protocol_type(name: &str) -> TypeInfo {
    let module = ModuleRef::Named("raygeo.sketch".to_string());
    let mut import = std::collections::HashSet::new();
    import.insert(ImportRef::Module(module.clone()));
    TypeInfo {
        name: format!("protocols.{}", name),
        source_module: Some(module),
        import,
        type_refs: HashMap::new(),
    }
}

fn screen_map_type() -> TypeInfo {
    let mut import = std::collections::HashSet::new();
    import.insert(ImportRef::Module(ModuleRef::Named(
        "collections.abc".to_string(),
    )));
    TypeInfo {
        name: "collections.abc.Callable[[tuple[float, float]], \
               tuple[float, float]]"
            .to_string(),
        source_module: None,
        import,
        type_refs: HashMap::new(),
    }
}

macro_rules! duck_typed {
    ($name:ident, $stub:expr) => {
        #[derive(Clone)]
        pub struct $name<'py>(pub Bound<'py, PyAny>);

        impl<'a, 'py> FromPyObject<'a, 'py> for $name<'py> {
            type Error = PyErr;

            fn extract(ob: pyo3::Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
                Ok($name(ob.to_owned()))
            }
        }

        impl PyStubType for $name<'_> {
            fn type_output() -> TypeInfo {
                $stub
            }
        }
    };
}

// The canvas the symbols replay onto (cairo context or stand-in).
duck_typed!(Canvas, protocol_type("Canvas"));

// The document the symbol layout reads through.
duck_typed!(DrawContext, protocol_type("DrawContext"));

// Maps model coordinates to screen coordinates.
duck_typed!(ScreenMap, screen_map_type());
