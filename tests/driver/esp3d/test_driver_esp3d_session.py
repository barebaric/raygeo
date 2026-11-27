"""ESP3D / FluidNC session tests against the HTTP/WebSocket
emulator."""

import asyncio
import time

import pytest

from raygeo.driver.config import Esp3dConfig
from raygeo.driver.core import DriverEvent
from raygeo.driver.core.types import DeviceStatus
from raygeo.driver.esp3d.emulator import Esp3dEmulator
from raygeo.driver.grbl import GrblNetworkDriver

FAST_CONFIG = {
    "host": "127.0.0.1",
    "request_timeout": 2.0,
    "connect_timeout": 2.0,
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 0.05,
    "reconnect_delay": 0.2,
    "command_timeout": 2.0,
    "poll_response_interval": 0.01,
    "safety_shutdown_delay": 0.02,
}

DIALECT = {
    "home_all": "$H",
    "home_axis": "$H{axis_letter}",
    "move_to": "$J=G90 G21 F{speed} X{x} Y{y}",
    "jog": "$J=G91 G21 F{speed}",
    "clear_alarm": "$X",
    "laser_on": "M4 S{power:.0f}",
    "laser_off": "M5",
    "focus_laser_on": "M3 S{power:.0f}",
    "tool_change": "T{tool_number}",
    "set_wcs_offset": "G10 L2 P{p_num} X{x} Y{y} Z{z}",
    "probe_cycle": "G38.2 {axis_letter}{max_travel} F{feed_rate}",
    "safety_off_commands": ["M5", "M9"],
}


class EventRecorder:
    def __init__(self):
        self.events = []

    def callback(self, event):
        self.events.append(event)

    def names(self):
        return [type(event).__name__ for event in self.events]

    async def wait_for(self, kind, pred=None, timeout=5.0):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for event in self.events:
                if not isinstance(event, kind):
                    continue
                if pred is None or pred(event):
                    return event
            await asyncio.sleep(0.01)
        raise AssertionError(
            f"event {kind!r} not observed within {timeout:.1f}s; "
            f"got {self.names()}"
        )


def make_session(events, config=None, dialect=None):
    merged = {**FAST_CONFIG, **(config or {})}
    return GrblNetworkDriver(
        config=Esp3dConfig(**merged),
        dialect={**DIALECT, **(dialect or {})},
        event_callback=events.callback,
    )


@pytest.fixture
def events():
    return EventRecorder()


@pytest.fixture
async def emulator():
    emu = Esp3dEmulator()
    await emu.start()
    try:
        yield emu
    finally:
        await emu.stop()


@pytest.fixture
async def rig(emulator, events):
    session = make_session(
        events,
        config={
            "http_port": emulator.http_port,
            "ws_port": emulator.ws_port,
        },
    )
    await session.connect()
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
    )
    events.events.clear()
    try:
        yield session, emulator, events
    finally:
        await session.disconnect()


class TestConnection:
    async def test_connect_reports_resource_uri(self, rig):
        session, emulator, _ = rig
        assert session.resource_uri == (
            f"tcp://127.0.0.1:{emulator.http_port}"
        )

    async def test_daemon_factory_builds_esp3d_session(self, emulator, events):
        session = GrblNetworkDriver(
            config={
                **FAST_CONFIG,
                "http_port": emulator.http_port,
                "ws_port": emulator.ws_port,
            },
            dialect=DIALECT,
            event_callback=events.callback,
        )
        await session.connect()
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "CONNECTED",
        )
        assert session.resource_uri == (
            f"tcp://127.0.0.1:{emulator.http_port}"
        )
        await session.disconnect()

    async def test_connect_fails_when_device_is_down(self, events):
        session = make_session(
            events,
            config={"host": "127.0.0.1", "http_port": 1, "ws_port": 1},
        )
        await session.connect()
        event = await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda e: e.status == "ERROR",
        )
        assert event.message is not None
        await session.disconnect()


class TestCommands:
    async def test_interactive_command(self, rig):
        session, emulator, _ = rig
        await session.execute_interactive_command("M5")
        assert emulator.commands[-1] == "M5"

    async def test_hold_and_resume(self, rig):
        session, emulator, _ = rig
        await session.set_hold(True)
        await session.set_hold(False)
        # Status polls ("?") may interleave with realtime commands.
        assert "!" in emulator.commands
        assert "~" in emulator.commands
        assert emulator.commands.index("!") < emulator.commands.index("~")

    async def test_raw_lines_are_forwarded(self, rig):
        session, emulator, events = rig
        emulator.broadcast("[MSG:hello]")
        await events.wait_for(
            DriverEvent.RawRx, lambda e: e.line == "[MSG:hello]"
        )


class TestStateAndSettings:
    async def test_status_report_drives_state(self, rig):
        _, emulator, events = rig
        emulator.set_status("<Run|MPos:1.000,2.000,3.000|FS:100,0>")
        event = await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status == DeviceStatus.RUN,
        )
        assert list(event.state.machine_pos) == [1.0, 2.0, 3.0]

    async def test_read_settings(self, rig):
        session, _, _ = rig
        pairs = await session.read_settings()
        assert ("13", "0") in pairs
        assert ("130", "400.000") in pairs

    async def test_read_wcs_offsets(self, rig):
        session, _, _ = rig
        offsets = await session.read_wcs_offsets()
        assert offsets["G54"] == (0.0, 0.0, 0.0)
        assert set(offsets) == {"G54", "G55", "G56", "G57", "G58", "G59"}

    async def test_read_parser_state(self, rig):
        session, _, _ = rig
        assert await session.read_parser_state() == "G54"

    async def test_detect_unit_system(self, rig):
        session, _, _ = rig
        assert await session.detect_unit_system() == "metric"

    async def test_probe_cycle(self, rig):
        session, _, _ = rig
        assert await session.run_probe_cycle("Z", 10.0, 100.0) == (
            0.0,
            0.0,
            0.0,
        )

    async def test_set_wcs_offset_uses_dialect(self, rig):
        session, emulator, _ = rig
        await session.set_wcs_offset("G55", 1.0, 2.0, None)
        assert emulator.commands[-1] == "G10 L2 P2 X1.0 Y2.0"


class TestJobs:
    async def test_run_uploads_and_starts(self, rig):
        session, emulator, events = rig
        progress = []
        await asyncio.wait_for(
            session.run(
                "M17 S1\nG1 X10 F1000\nM5\n",
                op_line_map={0: 0, 1: 1, 2: 2},
                progress_callback=progress.append,
            ),
            5.0,
        )
        assert emulator.uploaded == b"M17 S1\nG1 X10 F1000\nM5\n"
        assert emulator.started_files == ["rayforge.gcode"]
        assert progress == [0, 1, 2]
        await events.wait_for(DriverEvent.JobFinished)

    async def test_run_raw_uploads_and_starts(self, rig):
        session, emulator, events = rig
        await asyncio.wait_for(session.run_raw("M5\n"), 5.0)
        assert emulator.uploaded == b"M5\n"
        assert emulator.started_files == ["rayforge.gcode"]
        await events.wait_for(DriverEvent.JobFinished)

    async def test_cancel_sends_soft_reset(self, rig):
        session, emulator, events = rig
        emulator.job_duration = 5.0
        task = asyncio.ensure_future(session.run("M5\n"))
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        await session.cancel()
        await events.wait_for(DriverEvent.JobFinished)
        assert "\x18" in emulator.commands
        await asyncio.wait_for(task, 5.0)


class TestDisconnect:
    async def test_disconnect_reports_disconnected(self, rig):
        session, _, events = rig
        await session.disconnect()
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda e: e.status == "DISCONNECTED",
        )
