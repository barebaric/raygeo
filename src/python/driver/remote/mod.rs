//! The `raygeo.driver.remote` submodule: daemon, client and codec.

pub(crate) mod bridge;
pub(crate) mod client;
pub(crate) mod daemon;
pub(crate) mod events;
pub(crate) mod inspect;
pub(crate) mod mdns;
pub(crate) mod protocol;
pub(crate) mod session;

use pyo3::prelude::*;
use pyo3::types::PySet;

pub(crate) const MODULE_DOC: &str = "\
Remote sessions: a device-host daemon, a client proxy and the \
versioned wire codec that connects them.
";

pyo3_stub_gen::module_doc!("raygeo.driver.remote", "{}", MODULE_DOC);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let remote = PyModule::new(py, "remote")?;
    remote.setattr("__doc__", MODULE_DOC)?;
    remote.add(
        "__all__",
        vec!["client", "daemon", "inspect", "mdns", "protocol"],
    )?;

    let protocol = PyModule::new(py, "protocol")?;
    protocol.setattr(
        "__doc__",
        "Wire framing, envelope, catalog and codecs (v1).",
    )?;
    add_protocol_constants(&protocol)?;
    protocol.add_class::<protocol::LengthPrefix>()?;
    protocol.add("ProtocolError", py.get_type::<protocol::ProtocolError>())?;
    protocol.add("LENGTH_PREFIX", Py::new(py, protocol::LengthPrefix)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::encode_frame, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::decode_frame, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::read_frame, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::write_frame, &protocol)?)?;
    protocol.add_function(wrap_pyfunction!(protocol::request, &protocol)?)?;
    protocol.add_function(wrap_pyfunction!(protocol::response, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::error_response, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::event_frame, &protocol)?)?;
    protocol.add_function(wrap_pyfunction!(
        protocol::welcome_payload,
        &protocol
    )?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::encode_state, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::decode_state, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::encode_event, &protocol)?)?;
    protocol
        .add_function(wrap_pyfunction!(protocol::decode_event, &protocol)?)?;
    remote.add_submodule(&protocol)?;

    let daemon = PyModule::new(py, "daemon")?;
    daemon.setattr("__doc__", "Driver host and its configuration.")?;
    daemon.add(
        "DEFAULT_PORT",
        crate::driver::core::remote::daemon::DEFAULT_PORT,
    )?;
    daemon.add(
        "DEFAULT_RING_SIZE",
        crate::driver::core::remote::daemon::DEFAULT_RING_SIZE,
    )?;
    daemon.add(
        "DEFAULT_LEASE_GRACE",
        crate::driver::core::remote::daemon::DEFAULT_LEASE_GRACE,
    )?;
    daemon.add(
        "SEND_QUEUE_LIMIT",
        crate::driver::core::remote::daemon::SEND_QUEUE_LIMIT,
    )?;
    daemon.add_class::<daemon::DaemonConfig>()?;
    daemon.add_class::<daemon::Daemon>()?;
    daemon.add_class::<daemon::ClientQueue>()?;
    daemon.add_function(wrap_pyfunction!(
        daemon::validate_bind_policy,
        &daemon
    )?)?;
    remote.add_submodule(&daemon)?;

    let client = PyModule::new(py, "client")?;
    client.setattr(
        "__doc__",
        "Client-side remote session proxy and session factory.",
    )?;
    client.add(
        "DEFAULT_PORT",
        crate::driver::core::remote::daemon::DEFAULT_PORT,
    )?;
    client.add(
        "REMOTE_KEYS",
        (
            "remote_host",
            "remote_port",
            "token",
            "tls_ca",
            "tls_hostname",
            "url",
        ),
    )?;
    client.add_class::<client::RemoteSession>()?;
    client.add_function(wrap_pyfunction!(client::create_session, &client)?)?;
    client.add_function(wrap_pyfunction!(client::open_session, &client)?)?;
    client.add_function(wrap_pyfunction!(client::list_devices, &client)?)?;
    client.add_function(wrap_pyfunction!(client::list_drivers, &client)?)?;
    remote.add_submodule(&client)?;

    let mdns = PyModule::new(py, "mdns")?;
    mdns.setattr("__doc__", "mDNS announce and browse.")?;
    mdns.add(
        "SERVICE_TYPE",
        crate::driver::core::remote::mdns::SERVICE_TYPE,
    )?;
    mdns.add_class::<mdns::Announcer>()?;
    mdns.add_class::<mdns::ServiceInfo>()?;
    mdns.add_function(wrap_pyfunction!(mdns::announce, &mdns)?)?;
    mdns.add_function(wrap_pyfunction!(mdns::browse_daemons, &mdns)?)?;
    mdns.add_function(wrap_pyfunction!(mdns::_announce_address, &mdns)?)?;
    mdns.add_function(wrap_pyfunction!(mdns::_service_name, &mdns)?)?;
    mdns.add_function(wrap_pyfunction!(mdns::_service_name_for, &mdns)?)?;
    remote.add_submodule(&mdns)?;

    let inspect = PyModule::new(py, "inspect")?;
    inspect.setattr("__doc__", "Read-only daemon inspection.")?;
    inspect.add_function(wrap_pyfunction!(inspect::daemon_rpc, &inspect)?)?;
    inspect
        .add_function(wrap_pyfunction!(inspect::welcome_probe, &inspect)?)?;
    inspect.add_function(wrap_pyfunction!(inspect::snapshot, &inspect)?)?;
    remote.add_submodule(&inspect)?;

    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    for (name, module) in [
        ("raygeo.driver.remote.protocol", &protocol),
        ("raygeo.driver.remote.daemon", &daemon),
        ("raygeo.driver.remote.client", &client),
        ("raygeo.driver.remote.mdns", &mdns),
        ("raygeo.driver.remote.inspect", &inspect),
    ] {
        modules.set_item(name, module)?;
    }

    m.add_submodule(&remote)?;
    Ok(())
}

fn add_protocol_constants(protocol: &Bound<'_, PyModule>) -> PyResult<()> {
    use crate::driver::core::remote::protocol as core;
    protocol.add("PROTOCOL_VERSION", core::PROTOCOL_VERSION)?;
    protocol.add("MAX_FRAME_BYTES", core::MAX_FRAME_BYTES)?;
    protocol.add(
        "DEFAULT_KEEPALIVE_INTERVAL",
        core::DEFAULT_KEEPALIVE_INTERVAL,
    )?;
    protocol.add("DEFAULT_KEEPALIVE_MISSES", core::DEFAULT_KEEPALIVE_MISSES)?;
    for (name, value) in [
        ("AUTH", core::AUTH),
        ("PING", core::PING),
        ("PONG", core::PONG),
        ("DEVICE_LIST", core::DEVICE_LIST),
        ("DRIVER_LIST", core::DRIVER_LIST),
        ("SESSION_LIST", core::SESSION_LIST),
        ("SESSION_OPEN", core::SESSION_OPEN),
        ("SESSION_ATTACH", core::SESSION_ATTACH),
        ("SESSION_DETACH", core::SESSION_DETACH),
        ("SESSION_CLOSE", core::SESSION_CLOSE),
        ("SESSION_COMMAND", core::SESSION_COMMAND),
        ("SESSION_PROPERTY", core::SESSION_PROPERTY),
        ("CLIENT_LIST", core::CLIENT_LIST),
        ("WELCOME", core::WELCOME),
        ("EVENT", core::EVENT),
        ("BAD_REQUEST", core::BAD_REQUEST),
        ("BAD_VERSION", core::BAD_VERSION),
        ("AUTH_FAILED", core::AUTH_FAILED),
        ("UNKNOWN_TYPE", core::UNKNOWN_TYPE),
        ("UNKNOWN_SESSION", core::UNKNOWN_SESSION),
        ("UNKNOWN_DEVICE", core::UNKNOWN_DEVICE),
        ("UNKNOWN_METHOD", core::UNKNOWN_METHOD),
        ("UNKNOWN_PROPERTY", core::UNKNOWN_PROPERTY),
        ("LEASE_TAKEN", core::LEASE_TAKEN),
        ("LEASE_HELD", core::LEASE_HELD),
        ("JOB_RUNNING", core::JOB_RUNNING),
        ("FRAME_TOO_LARGE", core::FRAME_TOO_LARGE),
        ("INTERNAL", core::INTERNAL),
        ("KIND_PROGRESS", core::KIND_PROGRESS),
        ("KIND_JOB_ERROR", core::KIND_JOB_ERROR),
        ("KIND_LEASE", core::KIND_LEASE),
    ] {
        protocol.add(name, value)?;
    }
    let request_types = PySet::empty(protocol.py())?;
    for name in [
        core::AUTH,
        core::PING,
        core::PONG,
        core::DEVICE_LIST,
        core::DRIVER_LIST,
        core::SESSION_LIST,
        core::SESSION_OPEN,
        core::SESSION_ATTACH,
        core::SESSION_DETACH,
        core::SESSION_CLOSE,
        core::SESSION_COMMAND,
        core::SESSION_PROPERTY,
        core::CLIENT_LIST,
    ] {
        request_types.add(name)?;
    }
    protocol.add("REQUEST_TYPES", request_types)?;
    let drivers: Vec<&str> = crate::driver::registry::DriverKind::ALL
        .iter()
        .map(|kind| kind.name())
        .collect();
    protocol.add("DRIVERS", drivers)?;
    Ok(())
}
