use pyo3::exceptions::PyRuntimeError;

// Use pyo3's native create_exception! for runtime (no stub info).
pyo3::create_exception!(
    "raygeo.ops.assembly.adaptive",
    RoutingError,
    PyRuntimeError
);

pyo3::create_exception!(
    "raygeo.ops.assembly.adaptive",
    ResumePointNotFoundError,
    PyRuntimeError
);

pyo3::create_exception!(
    "raygeo.pipeline.execute",
    PipelineCancelled,
    PyRuntimeError
);

pyo3::create_exception!("raygeo.formats.coreldraw", CdrError, PyRuntimeError);

pyo3::create_exception!(
    "raygeo.formats.coreldraw",
    CdrUnsupportedFormat,
    CdrError
);

pyo3::create_exception!("raygeo.formats.coreldraw", CdrParseError, CdrError);

// pyo3_stub_gen::create_exception! has a bug: it uses stringify!($module)
// which embeds the surrounding quotes into the module string, causing the
// stub generator to reject it.  Instead we manually implement PyStubType
// + PyRuntimeType + inventory::submit! with the correct module name.
//
// Safety: these impls are valid for any create_exception! type in pyo3.

use pyo3_stub_gen::type_info::PyClassInfo;
use pyo3_stub_gen::{impl_py_runtime_type, inventory, PyStubType, TypeInfo};

impl PyStubType for RoutingError {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("RoutingError")
    }
}
impl_py_runtime_type!(RoutingError);
inventory::submit! {
    PyClassInfo {
        pyclass_name: "RoutingError",
        struct_id: std::any::TypeId::of::<RoutingError>,
        getters: &[],
        setters: &[],
        module: Some("raygeo.ops.assembly.adaptive"),
        doc: "Raised when all route strategies fail to find a path.",
        bases: &[|| <PyRuntimeError as PyStubType>::type_output()],
        has_eq: false,
        has_ord: false,
        has_hash: false,
        has_str: false,
        subclass: true,
    }
}

impl PyStubType for ResumePointNotFoundError {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("ResumePointNotFoundError")
    }
}
impl_py_runtime_type!(ResumePointNotFoundError);
inventory::submit! {
    PyClassInfo {
        pyclass_name: "ResumePointNotFoundError",
        struct_id: std::any::TypeId::of::<ResumePointNotFoundError>,
        getters: &[],
        setters: &[],
        module: Some("raygeo.ops.assembly.adaptive"),
        doc: "Raised when all resume strategies fail to find an engagement point.",
        bases: &[|| <PyRuntimeError as PyStubType>::type_output()],
        has_eq: false,
        has_ord: false,
        has_hash: false,
        has_str: false,
        subclass: true,
    }
}

impl PyStubType for PipelineCancelled {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("PipelineCancelled")
    }
}
impl_py_runtime_type!(PipelineCancelled);
inventory::submit! {
    PyClassInfo {
        pyclass_name: "PipelineCancelled",
        struct_id: std::any::TypeId::of::<PipelineCancelled>,
        getters: &[],
        setters: &[],
        module: Some("raygeo.pipeline.execute"),
        doc: "Raised when pipeline execution was cancelled (normal \
              during rapid rebuilds).",
        bases: &[|| <PyRuntimeError as PyStubType>::type_output()],
        has_eq: false,
        has_ord: false,
        has_hash: false,
        has_str: false,
        subclass: true,
    }
}

impl PyStubType for CdrError {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("CdrError")
    }
}
impl_py_runtime_type!(CdrError);
inventory::submit! {
    PyClassInfo {
        pyclass_name: "CdrError",
        struct_id: std::any::TypeId::of::<CdrError>,
        getters: &[],
        setters: &[],
        module: Some("raygeo.formats.coreldraw"),
        doc: "Base class of the CorelDRAW reader errors.",
        bases: &[|| <PyRuntimeError as PyStubType>::type_output()],
        has_eq: false,
        has_ord: false,
        has_hash: false,
        has_str: false,
        subclass: true,
    }
}

impl PyStubType for CdrUnsupportedFormat {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("CdrUnsupportedFormat")
    }
}
impl_py_runtime_type!(CdrUnsupportedFormat);
inventory::submit! {
    PyClassInfo {
        pyclass_name: "CdrUnsupportedFormat",
        struct_id: std::any::TypeId::of::<CdrUnsupportedFormat>,
        getters: &[],
        setters: &[],
        module: Some("raygeo.formats.coreldraw"),
        doc: "Raised when the data is not a CorelDRAW or CMX document.",
        bases: &[|| <CdrError as PyStubType>::type_output()],
        has_eq: false,
        has_ord: false,
        has_hash: false,
        has_str: false,
        subclass: true,
    }
}

impl PyStubType for CdrParseError {
    fn type_output() -> TypeInfo {
        TypeInfo::builtin("CdrParseError")
    }
}
impl_py_runtime_type!(CdrParseError);
inventory::submit! {
    PyClassInfo {
        pyclass_name: "CdrParseError",
        struct_id: std::any::TypeId::of::<CdrParseError>,
        getters: &[],
        setters: &[],
        module: Some("raygeo.formats.coreldraw"),
        doc: "Raised when a CorelDRAW container, record or argument table \
              cannot be read.",
        bases: &[|| <CdrError as PyStubType>::type_output()],
        has_eq: false,
        has_ord: false,
        has_hash: false,
        has_str: false,
        subclass: true,
    }
}
