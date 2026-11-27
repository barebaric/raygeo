"""Tests for raygeo.driver.remote.client.

Drives the proxy against a real daemon + firmware emulator, and
compares one scenario run twice — local session and proxy — for
event and result parity.
"""

import asyncio
from pathlib import Path

import pytest
from conftest import (
    DEFAULT_DIALECT,
    FAST_TIMINGS,
    EventRecorder,
    server_port,
    wait_until,
)

from raygeo.driver.core import DriverEvent
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.grbl.emulator import GrblEmulator
from raygeo.driver.remote import protocol
from raygeo.driver.remote.client import RemoteSession, create_session
from raygeo.driver.remote.daemon import Daemon, DaemonConfig

CERTS = Path(__file__).parent / "certs"
CA = CERTS / "daemon-test-ca.pem"
CERT = CERTS / "daemon-test-cert.pem"
KEY = CERTS / "daemon-test-key.pem"


def job_gcode(lines):
    return "".join(f"G0 X{i}\n" for i in range(lines))


@pytest.fixture
async def remote_rig():
    """A daemon hosting one emulator-backed grbl session."""
    mock = MockTransport()
    emulator = GrblEmulator(mock, speed_factor=10)
    device_task = asyncio.ensure_future(emulator.run())

    def factory(protocol_name, config, dialect, event_callback):
        assert protocol_name == "GrblSerialDriver"
        return GrblSerialDriver.with_transport(
            mock,
            config={**FAST_TIMINGS, **(config or {})},
            dialect={**DEFAULT_DIALECT, **(dialect or {})},
            event_callback=event_callback,
        )

    daemon = Daemon(
        DaemonConfig(port=0, keepalive_interval=0.5, lease_grace=0.5),
        session_factory=factory,
    )
    server = await daemon.start()
    try:
        yield server_port(server), mock
    finally:
        await daemon.stop()
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


def make_proxy(port, events, **extra):
    config = {
        "remote_host": "127.0.0.1",
        "remote_port": port,
        **extra,
    }
    session = create_session(
        "GrblSerialDriver", config, DEFAULT_DIALECT, events.callback
    )
    assert isinstance(session, RemoteSession)
    return session


async def connect_and_wait(session, events, timeout=10.0):
    """Connect and wait for the device handshake to finish.

    ``connect()`` only starts the connection loop; the transport is
    opened and the handshake performed asynchronously afterwards.
    Issuing a command before ``CONNECTED`` races that loop and fails
    with "Serial transport not connected" on slow runners.
    """
    await session.connect()
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
        timeout=timeout,
    )


def test_factory_routes_local():
    session = create_session(
        "GrblSerialDriver", {"port": "/dev/ttyUSB0"}, DEFAULT_DIALECT, None
    )
    assert isinstance(session, GrblSerialDriver)


async def test_factory_routes_remote(remote_rig):
    port, mock = remote_rig
    session = create_session(
        "GrblSerialDriver",
        {"remote_host": "127.0.0.1", "remote_port": port},
        None,
        None,
    )
    assert isinstance(session, RemoteSession)


def test_factory_parses_url():
    session = create_session(
        "GrblSerialDriver",
        {"url": "raydriver://example.org:9999#secret"},
        None,
        None,
    )
    assert isinstance(session, RemoteSession)
    assert session._endpoint["remote_host"] == "example.org"
    assert session._endpoint["remote_port"] == 9999
    assert session._endpoint["token"] == "secret"


def test_factory_rejects_bad_url():
    with pytest.raises(ValueError):
        create_session(
            "GrblSerialDriver", {"url": "http://example.org"}, None, None
        )


async def test_connect_and_interactive_command(remote_rig):
    port, mock = remote_rig
    events = EventRecorder()
    proxy = make_proxy(port, events)
    try:
        await connect_and_wait(proxy, events)
        lines = await proxy.execute_interactive_command("$I")
        assert any("VER:" in line for line in lines)
        assert proxy.resource_uri is None or isinstance(
            proxy.resource_uri, str
        )
    finally:
        await proxy.aclose()


async def test_run_progress_parity(remote_rig):
    """One scenario twice: local session and proxy must agree."""
    port, mock = remote_rig
    local_mock = MockTransport()
    local_emulator = GrblEmulator(local_mock, speed_factor=10)
    local_task = asyncio.ensure_future(local_emulator.run())
    local_events = EventRecorder()
    local = GrblSerialDriver.with_transport(
        local_mock,
        config=FAST_TIMINGS,
        dialect=DEFAULT_DIALECT,
        event_callback=local_events.callback,
    )
    remote_events = EventRecorder()
    proxy = make_proxy(port, remote_events)
    local_ops: list[int] = []
    remote_ops: list[int] = []
    gcode = job_gcode(8)
    op_map = {i: i for i in range(8)}
    try:
        await connect_and_wait(local, local_events)
        await connect_and_wait(proxy, remote_events)
        await local.run(
            gcode,
            op_line_map=op_map,
            progress_callback=local_ops.append,
        )
        await proxy.run(
            gcode,
            op_line_map=op_map,
            progress_callback=remote_ops.append,
        )
        assert local_ops == remote_ops == list(range(8))
        key_kinds = (
            "ConnectionStatusChanged",
            "JobFinished",
        )
        # Compare the multiset: local event marshaling may deliver
        # CONNECTED after an await point under load, while the
        # proxy preserves wire order.
        assert sorted(
            name for name in local_events.names() if name in key_kinds
        ) == sorted(
            name for name in remote_events.names() if name in key_kinds
        )
        assert not proxy.job_running
    finally:
        await proxy.aclose()
        await local.disconnect()
        local_task.cancel()
        try:
            await local_task
        except asyncio.CancelledError:
            pass


async def test_read_settings_parity(remote_rig):
    port, mock = remote_rig
    local_mock = MockTransport()
    local_emulator = GrblEmulator(local_mock, speed_factor=10)
    local_task = asyncio.ensure_future(local_emulator.run())
    local_events = EventRecorder()
    local = GrblSerialDriver.with_transport(
        local_mock,
        config=FAST_TIMINGS,
        dialect=DEFAULT_DIALECT,
        event_callback=local_events.callback,
    )
    remote_events = EventRecorder()
    proxy = make_proxy(port, remote_events)
    try:
        await connect_and_wait(local, local_events)
        await connect_and_wait(proxy, remote_events)
        local_settings = [list(pair) for pair in await local.read_settings()]
        remote_settings = [list(pair) for pair in await proxy.read_settings()]
        assert local_settings == remote_settings
    finally:
        await proxy.aclose()
        await local.disconnect()
        local_task.cancel()
        try:
            await local_task
        except asyncio.CancelledError:
            pass


async def test_disconnect_sleeping_and_reattach(remote_rig):
    port, mock = remote_rig
    events = EventRecorder()
    proxy = make_proxy(port, events)
    try:
        await connect_and_wait(proxy, events)
        run_task = asyncio.ensure_future(
            proxy.run(job_gcode(40), op_line_map={i: i for i in range(40)})
        )
        await wait_until(lambda: proxy.job_running)
        proxy.drop_connection()  # simulate the network dropping
        with pytest.raises(ConnectionError):
            await run_task
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "SLEEPING",
        )
        await proxy.attach()
        await wait_until(lambda: not proxy.job_running, timeout=20.0)
        assert b"G0 X39" in b"".join(mock.sent())
    finally:
        await proxy.aclose()


async def test_cancel_stops_job(remote_rig):
    port, mock = remote_rig
    events = EventRecorder()
    proxy = make_proxy(port, events)
    try:
        await connect_and_wait(proxy, events)
        run_task = asyncio.ensure_future(
            proxy.run(job_gcode(60), op_line_map={i: i for i in range(60)})
        )
        await wait_until(lambda: proxy.job_running)
        await proxy.cancel()
        await asyncio.wait_for(run_task, 10.0)
        assert not proxy.job_running
    finally:
        await proxy.aclose()


async def test_update_dialect_is_synchronous(remote_rig):
    port, mock = remote_rig
    events = EventRecorder()
    proxy = make_proxy(port, events)
    try:
        await connect_and_wait(proxy, events)
        result = proxy.update_dialect({**DEFAULT_DIALECT})
        assert result is None
        lines = await proxy.execute_interactive_command("$I")
        assert any("VER:" in line for line in lines)
    finally:
        await proxy.aclose()


async def test_update_dialect_before_connect(remote_rig):
    port, mock = remote_rig
    proxy = make_proxy(port, EventRecorder())
    try:
        proxy.update_dialect({**DEFAULT_DIALECT})
    finally:
        await proxy.aclose()


async def test_token_requires_tls(remote_rig):
    port, mock = remote_rig
    # Non-loopback endpoint: the refusal fires before any connect.
    proxy = make_proxy(
        port, EventRecorder(), token="secret", remote_host="192.0.2.1"
    )
    with pytest.raises(ConnectionError, match="without TLS"):
        await proxy.connect()


async def test_token_loopback_without_tls():
    # On loopback the token never leaves the machine, so TLS is not
    # required — this is the `daemon --setup` quickstart path.
    mock = MockTransport()
    emulator = GrblEmulator(mock, speed_factor=10)
    device_task = asyncio.ensure_future(emulator.run())

    def factory(protocol_name, config, dialect, event_callback):
        return GrblSerialDriver.with_transport(
            mock,
            config={**FAST_TIMINGS, **(config or {})},
            dialect={**DEFAULT_DIALECT, **(dialect or {})},
            event_callback=event_callback,
        )

    daemon = Daemon(
        DaemonConfig(port=0, token="secret", keepalive_interval=0.5),
        session_factory=factory,
    )
    server = await daemon.start()
    events = EventRecorder()
    proxy = make_proxy(server_port(server), events, token="secret")
    try:
        await connect_and_wait(proxy, events)
        lines = await proxy.execute_interactive_command("$I")
        assert any("VER:" in line for line in lines)
    finally:
        await proxy.aclose()
        await daemon.stop()
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


async def test_tls_token_roundtrip():
    mock = MockTransport()
    emulator = GrblEmulator(mock, speed_factor=10)
    device_task = asyncio.ensure_future(emulator.run())

    def factory(protocol_name, config, dialect, event_callback):
        return GrblSerialDriver.with_transport(
            mock,
            config={**FAST_TIMINGS, **(config or {})},
            dialect={**DEFAULT_DIALECT, **(dialect or {})},
            event_callback=event_callback,
        )

    daemon = Daemon(
        DaemonConfig(
            port=0,
            token="secret",
            tls_cert=str(CERT),
            tls_key=str(KEY),
            keepalive_interval=0.5,
        ),
        session_factory=factory,
    )
    server = await daemon.start()
    events = EventRecorder()
    proxy = make_proxy(
        server_port(server),
        events,
        token="secret",
        tls_ca=str(CA),
        tls_hostname="localhost",
    )
    try:
        await connect_and_wait(proxy, events)
        lines = await proxy.execute_interactive_command("$I")
        assert any("VER:" in line for line in lines)
    finally:
        await proxy.aclose()
        await daemon.stop()
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


async def test_version_mismatch_refused():
    async def handler(reader, writer):
        try:
            await protocol.write_frame(
                writer,
                {
                    "type": protocol.WELCOME,
                    "payload": {
                        "protocol": 999,
                        "raydriver": "x",
                        "auth_required": False,
                    },
                },
            )
            await asyncio.sleep(0.2)
        finally:
            writer.close()
            await writer.wait_closed()

    server = await asyncio.start_server(handler, "127.0.0.1", 0)
    proxy = create_session(
        "GrblSerialDriver",
        {
            "remote_host": "127.0.0.1",
            "remote_port": server_port(server),
        },
        None,
        None,
    )
    try:
        with pytest.raises(ConnectionError, match="protocol version"):
            await proxy.connect()
    finally:
        server.close()
        await server.wait_closed()
