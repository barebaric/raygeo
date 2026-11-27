//! The `raydriver.grbl` Python submodule: GRBL drivers, transports
//! and value types.

pub(crate) mod driver;
pub(crate) mod parser;
pub(crate) mod probe;
pub(crate) mod simple;

use pyo3::prelude::*;

pub(crate) const MODULE_DOC: &str = "\
GRBL device drivers implemented in Rust.

GrblSerialDriver, GrblTelnetDriver, GrblBluetoothDriver and \
GrblNetworkDriver each own a connection lifecycle (handshake, status \
polling, reconnect), interactive commands, and job streaming with \
character-counting flow control, stall detection and deadlock \
recovery; they differ only in their transport. \
GrblSerialSimpleDriver (raydriver.grbl.simple) is the minimal \
alternative: strict ping-pong, for hardware where the full driver \
misbehaves. Dialects remain data owned by the application: resolved \
command templates are passed into the driver.
";

pyo3_stub_gen::module_doc!("raygeo.driver.grbl", "{}", MODULE_DOC);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let grbl_mod = PyModule::new(py, "grbl")?;

    grbl_mod.setattr("__doc__", MODULE_DOC)?;
    grbl_mod.add(
        "__all__",
        vec![
            "GrblSerialDriver",
            "GrblTelnetDriver",
            "GrblBluetoothDriver",
            "GrblNetworkDriver",
            "parser",
            "probe",
        ],
    )?;

    grbl_mod.add_class::<driver::PyGrblSerialDriver>()?;
    grbl_mod.add_class::<driver::PyGrblTelnetDriver>()?;
    grbl_mod.add_class::<driver::PyGrblBluetoothDriver>()?;
    grbl_mod.add_class::<driver::PyGrblNetworkDriver>()?;

    let simple_mod = PyModule::new(py, "simple")?;
    simple::register(&simple_mod)?;
    grbl_mod.add_submodule(&simple_mod)?;

    let parser_mod = PyModule::new(py, "parser")?;
    parser::register(&parser_mod)?;
    grbl_mod.add_submodule(&parser_mod)?;

    probe::register(&grbl_mod)?;

    m.add_submodule(&grbl_mod)?;
    // Note: raydriver.grbl is intentionally NOT registered in
    // sys.modules — the Python __init__.py at
    // python/raydriver/grbl/__init__.py serves as the package and
    // delegates to this Rust module.

    Ok(())
}
