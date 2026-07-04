use pyo3::create_exception;

create_exception!(
    "raygeo.ops.assembly.adaptive",
    RoutingError,
    pyo3::exceptions::PyRuntimeError
);

create_exception!(
    "raygeo.ops.assembly.adaptive",
    ResumePointNotFoundError,
    pyo3::exceptions::PyRuntimeError
);
