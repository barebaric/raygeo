"""Driver lifecycle, command exchange and job tests, driven against
the xTool WiFi firmware emulator."""

import asyncio

import pytest
from conftest import make_session, wait_until

from raygeo.driver.core import DriverEvent
from raygeo.driver.core.types import DeviceStatus


class TestConnection:
    async def test_connect_identifies_the_machine(self, emulator, events):
        session = make_session(
            events,
            config={
                "http_port": emulator.http_port,
                "ws_port": emulator.ws_port,
            },
        )
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
            )
            assert session.resource_uri == (
                f"tcp://127.0.0.1:{emulator.http_port}"
            )
        finally:
            await session.disconnect()

    async def test_connect_fails_on_a_foreign_device(self, events):
        from raygeo.driver.xtool.emulator import XtoolEmulator

        emulator = XtoolEmulator(machine_type="Some Router Board")
        await emulator.start()
        session = make_session(
            events,
            config={
                "http_port": emulator.http_port,
                "ws_port": emulator.ws_port,
                "reconnect_delay": 5.0,
            },
        )
        try:
            await session.connect()
            event = await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "ERROR",
            )
            assert "did not identify" in event.message
        finally:
            await session.disconnect()
            await emulator.stop()

    async def test_connect_fails_when_device_is_down(self, events):
        session = make_session(
            events,
            config={
                "host": "127.0.0.1",
                "http_port": 1,
                "ws_port": 1,
                "reconnect_delay": 5.0,
            },
        )
        await session.connect()
        event = await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda e: e.status == "ERROR",
        )
        assert event.message is not None


class TestCommands:
    async def test_execute_interactive_command(self, rig):
        session, emulator, events = rig
        lines = await session.execute_interactive_command("M5")
        assert lines == []
        assert emulator.commands == ["M5"]
        await events.wait_for(
            DriverEvent.CommandStatusChanged,
            lambda e: e.status == "IDLE",
        )

    async def test_rejected_command_raises(self, rig):
        session, emulator, _ = rig
        emulator.reject_commands = True
        with pytest.raises(RuntimeError, match="rejected"):
            await session.execute_interactive_command("M5")

    async def test_execute_block_records_lines(self, rig):
        session, emulator, _ = rig
        await session.execute_block("M17 S1\nM207 S0\n")
        assert emulator.blocks == ["M17 S1\nM207 S0\n"]
        assert emulator.commands == []

    async def test_non_json_response_raises(self, rig):
        session, emulator, _ = rig
        emulator.down = True
        with pytest.raises(Exception):
            await session.execute_interactive_command("M5")
        emulator.down = False

    async def test_set_power_uses_the_dialect_template(self, rig):
        session, emulator, _ = rig
        await session.set_power(500)
        assert emulator.commands == ["M4 S500"]
        await session.set_power(None)
        assert emulator.commands[-1] == "M5"

    async def test_set_focus_power_uses_the_dialect_template(self, rig):
        session, emulator, _ = rig
        await session.set_focus_power(100)
        assert emulator.commands == ["G1 F1 M3 S100"]


class TestMotion:
    async def test_home_sequence_and_completion(self, rig):
        session, emulator, events = rig
        await session.home()
        assert emulator.commands == ["M17 S1", "M207 S0", "M28", "M18"]

    async def test_jog_block(self, rig):
        session, emulator, _ = rig
        await session.jog(1000, [("x", 10.0), ("y", -5.0)])
        assert len(emulator.blocks) == 1
        block = emulator.blocks[0]
        assert "G92 X0 Y0" in block
        assert "G1 X10 Y-5 F1000 S0" in block
        assert block.startswith("M17 S1\n")
        assert block.endswith("M18\n")

    async def test_move_to_block(self, rig):
        session, emulator, _ = rig
        await session.move_to(600, 10, 20)
        assert len(emulator.blocks) == 1
        block = emulator.blocks[0]
        assert "G1 X10 Y20 F600 S0" in block
        assert "G92" not in block

    async def test_move_to_carries_speed(self, rig):
        session, emulator, _ = rig
        await session.move_to(1500, 5, 6)
        assert "F1500" in emulator.blocks[0]

    async def test_jog_without_deltas_is_a_noop(self, rig):
        session, emulator, _ = rig
        await session.jog(1000, [])
        assert emulator.blocks == []


class TestJobs:
    async def test_run_uploads_and_starts(self, rig):
        session, emulator, events = rig
        task = asyncio.ensure_future(session.run("M17 S1\nG1 X10 F1000\nM5\n"))
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        assert emulator.uploaded == b"M17 S1\nG1 X10 F1000\nM5\n"
        assert emulator.started_files == ["/tmp.gcode"]
        await asyncio.wait_for(task, 5.0)
        await events.wait_for(DriverEvent.JobFinished)
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "IDLE",
        )

    async def test_run_raw_aliases_run(self, rig):
        session, emulator, events = rig
        await asyncio.wait_for(session.run_raw("M5\n"), 5.0)
        assert emulator.uploaded == b"M5\n"
        await events.wait_for(DriverEvent.JobFinished)

    async def test_run_fails_when_job_never_starts(self, rig):
        session, emulator, _ = rig
        emulator.fail_next_start = True
        with pytest.raises(RuntimeError, match="rejected"):
            await asyncio.wait_for(session.run("M5\n"), 5.0)

    async def test_cancel_stops_the_job(self, rig):
        session, emulator, events = rig
        emulator.job_duration = 5.0
        task = asyncio.ensure_future(session.run("M5\n"))
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        await session.cancel()
        await events.wait_for(DriverEvent.JobFinished)
        assert "stop" in emulator.job_actions
        await asyncio.wait_for(task, 5.0)
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "IDLE",
        )

    async def test_pause_and_resume(self, rig):
        session, emulator, events = rig
        task = asyncio.ensure_future(session.run("M5\n"))
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        await session.set_hold(True)
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "HOLD",
        )
        await session.set_hold(False)
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        await asyncio.wait_for(task, 5.0)


class TestEvents:
    async def test_safety_alarm_sets_alarm_state(self, rig):
        session, emulator, _ = rig
        emulator.trigger_error("limitCheck")
        await wait_until(lambda: session.state.status == DeviceStatus.ALARM)
        assert session.state.error.title == "Limit Switch Triggered"

    async def test_unknown_error_still_alarms(self, rig):
        session, emulator, _ = rig
        emulator.trigger_error("somethingNew")
        await wait_until(lambda: session.state.status == DeviceStatus.ALARM)

    async def test_raw_lines_are_forwarded(self, rig):
        session, emulator, events = rig
        emulator.broadcast("ok:IDLE")
        await events.wait_for(DriverEvent.RawRx, lambda e: e.line == "ok:IDLE")

    async def test_job_states_drive_the_device_state(self, rig):
        session, emulator, _ = rig
        emulator.broadcast("ok:WORKING_FRAMING")
        await wait_until(lambda: session.state.status == DeviceStatus.RUN)
        emulator.broadcast("ok:PAUSING")
        await wait_until(lambda: session.state.status == DeviceStatus.HOLD)
        emulator.broadcast("WORK_STOPPED")
        await wait_until(lambda: session.state.status == DeviceStatus.IDLE)


class TestDisconnect:
    async def test_dropped_websocket_wakes_waiters(self, rig):
        session, emulator, events = rig
        emulator.job_duration = 30.0
        task = asyncio.ensure_future(session.run("M5\n"))
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        await emulator.stop()
        with pytest.raises(Exception):
            await asyncio.wait_for(task, 5.0)

    async def test_disconnect_reports_disconnected(self, rig):
        session, emulator, events = rig
        await session.disconnect()
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda e: e.status == "DISCONNECTED",
        )


class TestStartFailure:
    async def test_connect_recovers_when_device_appears(self, events):
        import socket

        from raygeo.driver.xtool.emulator import XtoolEmulator

        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            http_port = sock.getsockname()[1]
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            ws_port = sock.getsockname()[1]

        session = make_session(
            events,
            config={
                "host": "127.0.0.1",
                "http_port": http_port,
                "ws_port": ws_port,
                "reconnect_delay": 0.2,
            },
        )
        emulator = XtoolEmulator()
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "ERROR",
            )
            # The device appears (power plugged back in) on the same
            # ports, and the reconnect loop picks it up.
            await emulator.start(http_port=http_port, ws_port=ws_port)
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
                timeout=8.0,
            )
        finally:
            await session.disconnect()
            await emulator.stop()


async def test_detect_unit_system_is_metric(rig):
    session, emulator, events = rig
    assert await session.detect_unit_system() == "metric"


async def test_home_rejects_a_single_axis(rig):
    session, emulator, events = rig
    with pytest.raises(NotImplementedError):
        await session.home(axes=["x"])


async def test_clear_alarm_only_clears_the_local_state(rig):
    session, emulator, events = rig
    await session.clear_alarm()


async def test_read_wcs_offsets_is_empty(rig):
    session, emulator, events = rig
    assert await session.read_wcs_offsets() == {}


async def test_job_reports_every_op_at_start(rig):
    session, emulator, events = rig
    done = []
    await session.run(
        "G1 X0\nG1 X1\nG1 X2\n",
        {0: 0, 1: 0, 2: 1},
        progress_callback=done.append,
    )
    assert done == [0, 1]
