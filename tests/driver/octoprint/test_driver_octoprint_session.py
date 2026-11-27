"""OctoPrint session tests, driven against the OctoPrint server
emulator (raygeo.driver.octoprint.emulator)."""

import asyncio
import json

import pytest
from conftest import EventRecorder, wait_until

from raygeo.driver import driver_info
from raygeo.driver.core import DriverEvent
from raygeo.driver.octoprint import OctoPrintDriver
from raygeo.driver.octoprint.emulator import OctoPrintEmulator
from raygeo.driver.octoprint.probe import OctoPrintDeviceInfo, describe

FAST_CONFIG = {
    "request_timeout": 2.0,
    "connect_timeout": 2.0,
    "poll_interval": 0.1,
    "reconnect_delay": 0.2,
}

GCODE = "G1 X1 F1000\nG1 X2 F1000\n"


def make_session(events, emulator, config=None, api_key="test-api-key"):
    return OctoPrintDriver(
        config={
            "host": "127.0.0.1",
            "port": emulator.http_port,
            "api_key": api_key,
            **FAST_CONFIG,
            **(config or {}),
        },
        event_callback=events.callback,
    )


@pytest.fixture
async def emulator():
    emu = OctoPrintEmulator()
    await emu.start()
    try:
        yield emu
    finally:
        await emu.stop()


@pytest.fixture
async def rig(emulator, events):
    """A connected session running against the emulator."""
    session = make_session(events, emulator)
    await session.connect()
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
    )
    try:
        yield session, emulator, events
    finally:
        await session.disconnect()


class TestProbe:
    async def test_probe_device(self, emulator):
        session = make_session(EventRecorder(), emulator)
        info = await session.probe_device()
        await session.disconnect()
        assert isinstance(info, OctoPrintDeviceInfo)
        assert info.server_version == "1.9.3"
        assert info.name == "TestPrinter"
        assert info.max_travel == (200.0, 300.0)

    def test_describe_without_server_data(self):
        info = describe(None, None)
        assert isinstance(info, OctoPrintDeviceInfo)
        assert info.server_version is None
        assert info.name == "OctoPrint"
        assert info.max_travel is None

    def test_driver_info(self):
        info = driver_info("OctoPrintDriver")
        assert info.name == "OctoPrintDriver"
        capabilities = info.capabilities
        assert capabilities["uses_gcode"] is True
        assert capabilities["supports_settings"] is False
        assert capabilities["supports_unit_detection"] is False
        assert capabilities["supports_hold"] is True
        assert hasattr(OctoPrintDriver, "probe_info")


class TestConnection:
    async def test_connect_requires_auth(self, emulator, events):
        session = make_session(events, emulator, api_key="wrong-key")
        await session.connect()
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "ERROR",
        )
        await session.disconnect()

    async def test_connect_without_auth_requirement(self, events):
        emu = OctoPrintEmulator(api_key="", require_auth=False)
        await emu.start()
        try:
            session = make_session(events, emu, api_key="")
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda event: event.status == "CONNECTED",
            )
            await session.disconnect()
        finally:
            await emu.stop()

    async def test_state_updates_from_push(self, rig):
        session, emulator, events = rig
        emulator.start_print(duration_s=0.2)
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda event: event.state.status.name == "RUN",
        )
        # The job was started outside the session, so the terminal
        # event only resets the state; no session-side JobFinished.
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda event: event.state.status.name == "IDLE",
        )

    async def test_websocket_authenticates_immediately(self, rig):
        # ``/sockjs/websocket`` sends no ``o`` open frame; the auth
        # message must go out right after connecting, or OctoPrint
        # withholds status updates and the driver stays UNKNOWN.
        _session, emulator, _events = rig
        await wait_until(
            lambda: any("auth" in msg for msg in emulator.ws_messages)
        )
        auth = json.loads(emulator.ws_messages[0])
        assert auth == {"auth": "rayforge:emulator-session-key"}

    async def test_state_getter_stays_responsive_during_pushes(self, rig):
        # Polling the PyO3 state getter while an event is delivered
        # must not deadlock on the GIL/state lock pair.
        session, emulator, _events = rig
        await wait_until(lambda: emulator.ws_messages)
        emulator.set_state_text(
            "Printing",
            operational=True,
            printing=True,
        )
        await wait_until(
            lambda: session.state.status.name == "RUN",
            timeout=2.0,
        )

    async def test_state_updates_from_raw_json_frame(self, rig):
        session, emulator, _events = rig
        await wait_until(lambda: emulator.ws_messages)
        emulator.send_frame(
            json.dumps(
                {
                    "current": {
                        "state": {
                            "text": "Printing",
                            "flags": {
                                "operational": True,
                                "printing": True,
                            },
                        }
                    }
                }
            )
        )
        await wait_until(lambda: session.state.status.name == "RUN")

    async def test_state_updates_from_sockjs_batch(self, rig):
        session, emulator, _events = rig
        await wait_until(lambda: emulator.ws_messages)
        payload = json.dumps(
            {
                "current": {
                    "state": {
                        "text": "Paused",
                        "flags": {
                            "operational": True,
                            "paused": True,
                        },
                    }
                }
            }
        )
        emulator.send_frame("a" + json.dumps([payload]))
        await wait_until(lambda: session.state.status.name == "HOLD")

    async def test_framed_transport_pushes_batches(self, events):
        emu = OctoPrintEmulator(framed_ws=True)
        await emu.start()
        try:
            session = make_session(events, emu)
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda event: event.status == "CONNECTED",
            )
            emu.set_state_text(
                "Printing",
                operational=True,
                printing=True,
            )
            await wait_until(lambda: session.state.status.name == "RUN")
            await session.disconnect()
        finally:
            await emu.stop()


class TestPathPrefix:
    async def test_path_prefix_routes_api_and_websocket(self, events):
        emu = OctoPrintEmulator(path_prefix="/octoprint")
        await emu.start()
        try:
            session = make_session(events, emu, config={"path": "/octoprint"})
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda event: event.status == "CONNECTED",
            )
            info = await session.probe_device()
            assert info.server_version == "1.9.3"
            await session.run_raw("M999")
            assert {"command": "M999"} in emu.commands
            await session.disconnect()
        finally:
            await emu.stop()


class TestJobs:
    async def test_run_uploads_and_completes(self, rig):
        session, emulator, events = rig
        op_map = {0: 0, 1: 1}
        await session.run(GCODE, op_map, [])
        assert len(emulator.uploaded) == 1
        assert emulator.uploaded[0].decode().splitlines() == [
            "G1 X1 F1000",
            "G1 X2 F1000",
        ]
        await events.wait_for(DriverEvent.JobFinished)

    async def test_run_reports_ops_upfront(self, rig):
        session, emulator, events = rig
        done_ops = []

        def on_progress(op_index):
            done_ops.append(op_index)

        op_map = {0: 0, 1: 1, 2: 1}
        await session.run(GCODE, op_map, [], on_progress)
        # No granular progress exists on this protocol: every op is
        # reported once, when the job starts.
        assert sorted(done_ops) == [0, 1, 1]

    async def test_run_raw_sends_commands(self, rig):
        session, emulator, events = rig
        await session.run_raw("M3 S500\nG1 X1 F1000\n")
        await events.wait_for(DriverEvent.JobFinished)
        assert {"command": "M3 S500"} in emulator.commands
        assert {"command": "G1 X1 F1000"} in emulator.commands

    async def test_cancel_finishes_the_job(self, rig):
        session, emulator, events = rig
        run_task = asyncio.ensure_future(session.run(GCODE, {0: 0}, []))
        await asyncio.sleep(0.05)
        await session.cancel()
        await events.wait_for(DriverEvent.JobFinished)
        await run_task
        assert {"command": "cancel"} in emulator.commands

    async def test_cancel_without_a_job_is_a_noop(self, rig):
        # OctoPrint answers 409 when nothing is printing; that must
        # not surface as a device error.
        session, emulator, _events = rig
        await session.cancel()
        assert {"command": "cancel"} in emulator.commands

    async def test_busy_server_rejects_upload(self, emulator, events):
        session = make_session(events, emulator)
        await session.connect()
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "CONNECTED",
        )
        emulator.fail_next_upload = True
        with pytest.raises(Exception, match="busy or not operational"):
            await session.run(GCODE, {}, [])
        await session.disconnect()


class TestCommands:
    async def test_home_all_axes(self, rig):
        session, emulator, events = rig
        await session.home()
        assert emulator.commands[-1] == {
            "command": "home",
            "axes": ["x", "y", "z"],
        }

    async def test_home_named_axes(self, rig):
        session, emulator, events = rig
        await session.home(["X"])
        assert emulator.commands[-1] == {"command": "home", "axes": ["x"]}

    async def test_move_to_is_absolute(self, rig):
        session, emulator, events = rig
        await session.move_to(1000.0, 10.0, 5.0, 1.5)
        assert emulator.commands[-1] == {
            "command": "jog",
            "x": 10.0,
            "y": 5.0,
            "z": 1.5,
            "speed": 1000.0,
            "absolute": True,
        }

    async def test_jog_is_relative(self, rig):
        session, emulator, events = rig
        await session.jog(300.0, [("x", 10.0), ("y", -5.0)])
        assert emulator.commands[-1] == {
            "command": "jog",
            "speed": 300.0,
            "x": 10.0,
            "y": -5.0,
        }

    async def test_move_to_absolute_omits_unspecified_speed(self, rig):
        session, emulator, events = rig
        await session.move_to_absolute(10.0, 5.0)
        assert emulator.commands[-1] == {
            "command": "jog",
            "x": 10.0,
            "y": 5.0,
            "absolute": True,
        }

    async def test_move_to_absolute_with_speed(self, rig):
        session, emulator, events = rig
        await session.move_to_absolute(10.0, 5.0, None, 6000.0)
        assert emulator.commands[-1] == {
            "command": "jog",
            "x": 10.0,
            "y": 5.0,
            "absolute": True,
            "speed": 6000.0,
        }

    async def test_select_tool_sends_t_command(self, rig):
        session, emulator, events = rig
        await session.select_tool(2)
        assert {"command": "T2"} in emulator.commands

    async def test_set_power_maps_to_m_codes(self, rig):
        session, emulator, events = rig
        await session.set_power(None)
        await session.set_power(500.0)
        assert {"command": "M5"} in emulator.commands
        assert {"command": "M3 S500"} in emulator.commands

    async def test_clear_alarm_sends_m999(self, rig):
        session, emulator, events = rig
        await session.clear_alarm()
        assert {"command": "M999"} in emulator.commands

    async def test_set_wcs_offset(self, rig):
        session, emulator, events = rig
        await session.set_wcs_offset("G55", 10.0, 20.0, 1.5)
        assert {
            "command": "G10 L2 P2 X10.000 Y20.000 Z1.500"
        } in emulator.commands

    async def test_set_wcs_offset_rejects_unknown_slot(self, rig):
        session, emulator, events = rig
        with pytest.raises(Exception, match="invalid WCS slot"):
            await session.set_wcs_offset("G61", 1.0, 2.0, None)

    async def test_set_hold_sends_pause_and_resume(self, rig):
        session, emulator, events = rig
        await session.set_hold(True)
        await session.set_hold(False)
        commands = [c.get("action") for c in emulator.commands]
        assert "pause" in commands
        assert "resume" in commands


class TestUniformSurface:
    async def test_methods_without_server_equivalents(self, rig):
        session, emulator, events = rig
        assert await session.read_settings() == []
        assert await session.read_wcs_offsets() == {}
        assert await session.read_parser_state() is None
        assert await session.detect_unit_system() is None
        pos = await session.run_probe_cycle("X", -10.0, 100.0)
        assert pos is None

    async def test_write_setting_raises_unsupported(self, rig):
        session, emulator, events = rig
        with pytest.raises(NotImplementedError, match="settings"):
            await session.write_setting("30", "1000")

    async def test_execute_interactive_returns_empty(self, rig):
        session, emulator, events = rig
        assert await session.execute_interactive_command("M115") == []
        assert {"command": "M115"} in emulator.commands

    def test_resource_uri(self, emulator):
        session = make_session(EventRecorder(), emulator)
        assert session.resource_uri == (
            f"tcp://127.0.0.1:{emulator.http_port}"
        )
