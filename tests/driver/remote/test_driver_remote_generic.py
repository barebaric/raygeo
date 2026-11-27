"""Protocol-agnostic remote sessions: the daemon resolves the
protocol and exposes driver metadata as info; callers never name a
protocol."""

import asyncio

import pytest
import pytest_asyncio
from conftest import EventRecorder

from raygeo.driver import driver_info, drivers
from raygeo.driver.core import DriverEvent
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.grbl.emulator import GrblEmulator
from raygeo.driver.remote.client import (
    list_devices,
    list_drivers,
    open_session,
)
from raygeo.driver.remote.daemon import Daemon, DaemonConfig

FAST_TIMINGS = {
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 0.05,
    "command_timeout": 2.0,
    "poll_response_interval": 0.01,
    "poll_response_attempts": 10,
}


async def connect_and_wait(session, events, timeout=10.0):
    """Connect and wait for the device handshake to finish.

    ``connect()`` only starts the connection loop; the transport is
    opened and the handshake performed asynchronously afterwards.
    Issuing a command before ``CONNECTED`` races that loop and fails
    with "Serial transport not connected" on slow runners.
    """
    await asyncio.wait_for(session.connect(), timeout=timeout)
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
        timeout=timeout,
    )


def test_driver_capabilities():
    grbl = driver_info("GrblSerialDriver").capabilities
    assert grbl["supports_probing"] is True
    assert grbl["supports_settings"] is True
    assert grbl["reports_buffer_stats"] is True
    assert grbl["supports_clear_alarm"] is True
    smoothie = driver_info("SmoothieDriver").capabilities
    assert smoothie["supports_probing"] is True
    assert smoothie["supports_clear_alarm"] is True
    assert smoothie["supports_hold"] is True
    marlin = driver_info("MarlinSerialDriver").capabilities
    assert marlin["supports_hold"] is False
    assert marlin["supports_unit_detection"] is True
    assert marlin["supports_clear_alarm"] is True
    assert (
        driver_info("GrblSerialSimpleDriver").capabilities[
            "reports_buffer_stats"
        ]
        is False
    )
    for cls in drivers:
        info = driver_info(cls.__name__)
        assert info.name == cls.__name__
        assert isinstance(info.capabilities["uses_gcode"], bool)
    assert driver_info("EpilogDriver").capabilities["uses_gcode"] is False
    assert driver_info("GrblSerialDriver").capabilities["uses_gcode"] is True


@pytest_asyncio.fixture
async def rig():
    mock = MockTransport()
    emulator = GrblEmulator(mock, speed_factor=10)
    device_task = asyncio.ensure_future(emulator.run())

    def factory(name, config, dialect, cb):
        return GrblSerialDriver.with_transport(
            mock,
            config={**FAST_TIMINGS, **(config or {})},
            dialect=dialect or {},
            event_callback=cb,
        )

    daemon = Daemon(
        DaemonConfig(
            port=0,
            sessions=[
                {
                    "protocol": "GrblSerialDriver",
                    "config": {"port": "/dev/ttyUSB0"},
                }
            ],
        ),
        session_factory=factory,
    )
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    try:
        yield daemon, port
    finally:
        await daemon.stop()
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


async def test_driver_list(rig):
    _, port = rig
    infos = await list_drivers(
        {"remote_host": "127.0.0.1", "remote_port": port}
    )
    assert {p["name"] for p in infos} == {cls.__name__ for cls in drivers}
    grbl = next(p for p in infos if p["name"] == "GrblSerialDriver")
    assert grbl["connection"] == "serial"
    capabilities = grbl["capabilities"]
    assert isinstance(capabilities, dict)
    assert capabilities["supports_probing"] is True


async def test_device_list_includes_configured(rig):
    _, port = rig
    devices = await list_devices(
        {"remote_host": "127.0.0.1", "remote_port": port}
    )
    assert len(devices) == 1
    assert devices[0]["device"] == "/dev/ttyUSB0"
    assert devices[0]["protocol"] == "GrblSerialDriver"
    assert devices[0]["label"] == "GrblSerialDriver"


async def test_open_by_device_resolves_protocol(rig):
    _, port = rig
    events = EventRecorder()
    session = open_session(
        {"remote_host": "127.0.0.1", "remote_port": port},
        device="/dev/ttyUSB0",
        dialect={},
        event_callback=events.callback,
    )
    assert session.protocol is None
    try:
        await connect_and_wait(session, events)
        assert session.protocol == "GrblSerialDriver"
        assert session.metadata is not None
        assert session.metadata["name"] == "GrblSerialDriver"
        reply = await asyncio.wait_for(
            session.execute_interactive_command("$I"), timeout=10
        )
        assert any("VER:" in line for line in reply)
    finally:
        await session.disconnect()


async def test_disconnect_releases_control_lease(rig):
    # A probe-like client connects, then disconnects; a second client
    # (the machine) must then be able to take control of the device.
    _, port = rig
    endpoint = {"remote_host": "127.0.0.1", "remote_port": port}
    first_events = EventRecorder()
    first = open_session(
        endpoint,
        device="/dev/ttyUSB0",
        dialect={},
        event_callback=first_events.callback,
    )
    await connect_and_wait(first, first_events)
    await first.disconnect()

    second_events = EventRecorder()
    second = open_session(
        endpoint,
        device="/dev/ttyUSB0",
        dialect={},
        event_callback=second_events.callback,
    )
    try:
        await connect_and_wait(second, second_events)
        reply = await asyncio.wait_for(
            second.execute_interactive_command("$I"), timeout=10
        )
        assert any("VER:" in line for line in reply)
    finally:
        await second.disconnect()


async def test_remote_jog_accepts_deltas(rig):
    # Jog deltas cross the wire as JSON lists; the daemon must restore
    # the tuples the Rust session expects.
    from raygeo.driver.core import DriverEvent

    _, port = rig
    ready = asyncio.Event()

    def on_event(event):
        if (
            isinstance(event, DriverEvent.ConnectionStatusChanged)
            and event.status == "CONNECTED"
        ):
            ready.set()

    session = open_session(
        {"remote_host": "127.0.0.1", "remote_port": port},
        device="/dev/ttyUSB0",
        dialect={},
        event_callback=on_event,
    )
    try:
        await asyncio.wait_for(session.connect(), timeout=10)
        await asyncio.wait_for(ready.wait(), timeout=10)
        await asyncio.wait_for(
            session.jog(1000, [("X", 1.0), ("Y", 2.0)]), timeout=10
        )
    finally:
        await session.disconnect()


async def test_open_by_url_endpoint(rig):
    _, port = rig
    session = open_session(
        f"raydriver://127.0.0.1:{port}", device="/dev/ttyUSB0", dialect={}
    )
    try:
        await asyncio.wait_for(session.connect(), timeout=10)
        assert session.protocol == "GrblSerialDriver"
    finally:
        await session.disconnect()


async def test_open_unknown_device():
    # A port-aware factory: only the known port opens, so probing an
    # unknown device fails and the daemon reports unknown_device.
    def factory(name, config, dialect, cb):
        if config.get("port") != "/dev/ttyUSB0":
            raise OSError("no such port")
        return GrblSerialDriver.with_transport(
            MockTransport(),
            config={**FAST_TIMINGS, **(config or {})},
            dialect=dialect or {},
            event_callback=cb,
        )

    daemon = Daemon(DaemonConfig(port=0), session_factory=factory)
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    try:
        session = open_session(
            {"remote_host": "127.0.0.1", "remote_port": port},
            device="/dev/nonexistent-raydriver",
            dialect={},
        )
        with pytest.raises(ConnectionError):
            await asyncio.wait_for(session.connect(), timeout=15)
    finally:
        await daemon.stop()


async def test_open_requires_device_or_session(rig):
    _, port = rig
    with pytest.raises(ValueError):
        open_session({"remote_host": "127.0.0.1", "remote_port": port})


def test_driver_capabilities_are_typed():
    # The daemon keeps no method-name table: GRBL simply does not
    # claim alarm clearing, while Smoothie does.  Unsupported calls
    # surface from the driver as NotImplementedError.
    grbl = driver_info("GrblSerialDriver").capabilities
    smoothie = driver_info("SmoothieDriver").capabilities
    assert grbl["supports_clear_alarm"] is True
    assert smoothie["supports_clear_alarm"] is True
