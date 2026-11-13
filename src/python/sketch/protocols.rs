//! Duck-typed parameter surfaces for the symbol rendering APIs.
//!
//! The generated stubs type these parameters against the runtime
//! protocols in `raygeo.sketch.protocols`; any object satisfying the
//! protocol is accepted at runtime.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3_stub_gen::{
    ImportKind, ImportRef, ModuleRef, PyStubType, TypeIdentifierRef, TypeInfo,
};

fn protocol_type(name: &str) -> TypeInfo {
    let module = ModuleRef::Named("raygeo.sketch.protocols".to_string());
    let mut import = std::collections::HashSet::new();
    import.insert(ImportRef::Module(module.clone()));
    let mut type_refs = HashMap::new();
    type_refs.insert(
        name.to_string(),
        TypeIdentifierRef {
            module: module.clone(),
            import_kind: ImportKind::Module,
        },
    );
    TypeInfo {
        name: name.to_string(),
        source_module: Some(module),
        import,
        type_refs,
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
