"""The remote RPC surface must match every hosted session class.

The daemon calls the Python driver method with the client's
arguments.  A signature mismatch therefore fails inside the daemon,
not at the client.  These tests pin the uniform argument names and
exercise two representative drivers end to end.
"""

import asyncio
import inspect

import pytest
from conftest import DEFAULT_DIALECT, EventRecorder

from raygeo.driver.core import DriverEvent
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.core.types import DeviceState
from raygeo.driver.epilog import EpilogDriver
from raygeo.driver.grbl import GrblNetworkDriver, GrblSerialDriver
from raygeo.driver.grbl.simple import GrblSerialSimpleDriver
from raygeo.driver.marlin import MarlinSerialDriver
from raygeo.driver.octoprint import OctoPrintDriver
from raygeo.driver.remote.client import RemoteSession, create_session
from raygeo.driver.remote.daemon import Daemon, DaemonConfig
from raygeo.driver.smoothie import SmoothieDriver
from raygeo.driver.smoothie.emulator import SmoothieEmulator

SESSIONS = {
    "GrblSerialDriver": GrblSerialDriver,
    "GrblSerialSimpleDriver": GrblSerialSimpleDriver,
    "GrblNetworkDriver": GrblNetworkDriver,
    "SmoothieDriver": SmoothieDriver,
    "MarlinSerialDriver": MarlinSerialDriver,
    "OctoPrintDriver": OctoPrintDriver,
    "EpilogDriver": EpilogDriver,
}

REMOTE_TIMINGS = {
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 0.05,
    "reconnect_delay": 0.2,
    "command_timeout": 2.0,
}


@pytest.mark.parametrize("protocol", sorted(SESSIONS))
def test_hosted_sessions_accept_remote_arguments(protocol):
    """Every hostable session accepts the arguments the proxy sends."""
    cls = SESSIONS[protocol]
    home = inspect.signature(cls.home)
    assert "active_wcs" in home.parameters
    run = inspect.signature(cls.run)
    assert "op_estimates" in run.parameters
    execute = inspect.signature(cls.execute_command)
    assert "command" in execute.parameters


async def test_remote_home_smoothie():
    """home(active_wcs) succeeds against a daemon-hosted Smoothie."""
    mock = MockTransport()
    emulator = SmoothieEmulator(mock, speed_factor=50)
    device_task = asyncio.ensure_future(emulator.run())

    def factory(name, config, dialect, callback):
        return SmoothieDriver.with_transport(
            mock,
            config={**REMOTE_TIMINGS, **(config or {})},
            dialect={**DEFAULT_DIALECT, **(dialect or {})},
            event_callback=callback,
        )

    daemon = Daemon(
        DaemonConfig(
            port=0,
            sessions=[
                {"protocol": "SmoothieDriver", "config": {"port": "/dev/null"}}
            ],
        ),
        session_factory=factory,
    )
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    events = EventRecorder()
    session = create_session(
        "SmoothieDriver",
        {"remote_host": "127.0.0.1", "remote_port": port},
        DEFAULT_DIALECT,
        events.callback,
    )
    assert isinstance(session, RemoteSession)
    try:
        await asyncio.wait_for(session.connect(), timeout=5)
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "CONNECTED",
            timeout=5,
        )
        await asyncio.wait_for(session.home(None, "G54"), timeout=5)
    finally:
        try:
            await asyncio.wait_for(session.disconnect(), timeout=5)
        except Exception:
            pass
        await daemon.stop()
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


async def test_remote_run_reports_job_errors():
    """A hosted run() failure reaches the caller as an error."""

    class FailingJobSession:
        state = DeviceState()
        job_running = False
        resource_uri = None
        buffer_count = 0
        rx_buffer_size = 0
        pending_commands = []

        async def connect(self):
            pass

        async def disconnect(self):
            pass

        async def run(
            self,
            gcode,
            op_line_map=None,
            op_estimates=None,
            progress_callback=None,
        ):
            raise RuntimeError("job exploded")

    def factory(name, config, dialect, callback):
        return FailingJobSession()

    daemon = Daemon(DaemonConfig(port=0), session_factory=factory)
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    events = EventRecorder()
    session = create_session(
        "GrblSerialDriver",
        {"remote_host": "127.0.0.1", "remote_port": port},
        DEFAULT_DIALECT,
        events.callback,
    )
    assert isinstance(session, RemoteSession)
    try:
        with pytest.raises(ConnectionError) as excinfo:
            await asyncio.wait_for(session.run("G1 X1\n", {0: 0}), timeout=5)
        assert "job exploded" in str(excinfo.value)
        await events.wait_for(
            DriverEvent.JobError,
            lambda event: "job exploded" in event.message,
            timeout=5,
        )
    finally:
        try:
            await asyncio.wait_for(session.aclose(), timeout=5)
        except Exception:
            pass
        await daemon.stop()


async def test_daemon_reports_synchronous_session_errors():
    """A method raising before returning an awaitable is an RPC error."""

    class ExplodingSession:
        state = DeviceState()
        job_running = False
        resource_uri = None
        buffer_count = 0
        rx_buffer_size = 0
        pending_commands = []

        def home(self, axes=None, active_wcs="G54"):
            raise TypeError("boom from session")

        async def connect(self):
            pass

        async def disconnect(self):
            pass

    def factory(name, config, dialect, callback):
        return ExplodingSession()

    daemon = Daemon(DaemonConfig(port=0), session_factory=factory)
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    session = create_session(
        "GrblSerialDriver",
        {"remote_host": "127.0.0.1", "remote_port": port},
        DEFAULT_DIALECT,
        None,
    )
    assert isinstance(session, RemoteSession)
    try:
        with pytest.raises(ConnectionError) as excinfo:
            await asyncio.wait_for(session.home(None, "G54"), timeout=5)
        assert "boom from session" in str(excinfo.value)
    finally:
        try:
            await asyncio.wait_for(session.aclose(), timeout=5)
        except Exception:
            pass
        await daemon.stop()


async def test_daemon_preserves_timeout_error_kind():
    """A Python TimeoutError reaches the caller as a timeout."""

    class SlowSession:
        state = DeviceState()
        job_running = False
        resource_uri = None
        buffer_count = 0
        rx_buffer_size = 0
        pending_commands = []

        def home(self, axes=None, active_wcs="G54"):
            raise TimeoutError("device did not answer")

        async def connect(self):
            pass

        async def disconnect(self):
            pass

    def factory(name, config, dialect, callback):
        return SlowSession()

    daemon = Daemon(DaemonConfig(port=0), session_factory=factory)
    server = await daemon.start()
    port = server.sockets[0].getsockname()[1]
    session = create_session(
        "GrblSerialDriver",
        {"remote_host": "127.0.0.1", "remote_port": port},
        DEFAULT_DIALECT,
        None,
    )
    assert isinstance(session, RemoteSession)
    try:
        with pytest.raises(TimeoutError) as excinfo:
            await asyncio.wait_for(session.home(None, "G54"), timeout=5)
        assert "device did not answer" in str(excinfo.value)
    finally:
        try:
            await asyncio.wait_for(session.aclose(), timeout=5)
        except Exception:
            pass
        await daemon.stop()
