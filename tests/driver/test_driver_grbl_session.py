"""Driver lifecycle and interactive command tests, driven against
the GRBL firmware emulator."""

import asyncio

import pytest
from conftest import make_session, sent_bytes, start_device

from raygeo.driver.core import DriverEvent
from raygeo.driver.core.types import DeviceStatus


class TestConnection:
    async def test_handshake_and_build_info(self, mock, events):
        emulator, device_task = await start_device(mock)
        session = make_session(mock, events)
        try:
            await session.connect()
            event = await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
            )
            assert event.message is None
            # The connection loop queried $I, whose OPT line
            # advertised the emulated 128-byte RX buffer.
            assert b"$I\n" in sent_bytes(mock)
            assert session.rx_buffer_size == 128
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_handshake_timeout_on_phantom_port(self, mock, events):
        # No emulator attached: nothing ever answers.
        session = make_session(mock, events, {"handshake_timeout": 0.15})
        try:
            await session.connect()
            event = await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "ERROR",
            )
            assert "No response" in event.message
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "SLEEPING",
            )
        finally:
            await session.disconnect()

    async def test_reconnect_after_recovery(self, mock, events):
        session = make_session(mock, events, {"handshake_timeout": 0.15})
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "ERROR",
            )
            # The device appears (cable plugged back in).
            emulator, device_task = await start_device(mock)
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
                timeout=8.0,
            )
            device_task.cancel()
        finally:
            await session.disconnect()

    async def test_command_while_disconnected_raises(self, mock, events):
        session = make_session(mock, events)
        with pytest.raises(ConnectionError):
            await session.execute_interactive_command("$X")


class TestStatusReports:
    async def test_status_report_updates_state(self, rig):
        session, mock, emulator, events = rig
        # Handshake polls already delivered the first report.
        state = session.state
        assert state.status == DeviceStatus.IDLE
        assert state.machine_pos == [0.0, 0.0, 0.0]

    async def test_rx_buffer_size_override_wins(self, mock, events):
        emulator, device_task = await start_device(mock)
        session = make_session(mock, events, {"rx_buffer_size_override": 64})
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
            )
            # OPT and Bf advertise 128, but the override sticks.
            await asyncio.sleep(0.15)
            assert session.rx_buffer_size == 64
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_fragmented_status_report_ignored(self, rig):
        session, mock, emulator, events = rig
        states_before = len(
            [
                e
                for e in events.events
                if isinstance(e, DriverEvent.StateChanged)
            ]
        )
        mock.push(b"<Idle|MPos")
        await asyncio.sleep(0.1)
        states_after = len(
            [
                e
                for e in events.events
                if isinstance(e, DriverEvent.StateChanged)
            ]
        )
        assert states_after == states_before


class TestInteractiveCommands:
    async def test_ok_response(self, rig):
        session, mock, emulator, events = rig
        lines = await session.execute_interactive_command("$X")
        assert lines == ["ok"]

    async def test_multi_line_response(self, rig):
        session, mock, emulator, events = rig
        lines = await session.execute_interactive_command("$G")
        assert lines[0].startswith("[G54")
        assert lines[-1] == "ok"

    async def test_error_response(self, rig):
        session, mock, emulator, events = rig
        lines = await session.execute_interactive_command("G999")
        assert lines == ["error:20"]

    async def test_queued_command(self, rig):
        session, mock, emulator, events = rig
        lines = await session.execute_command("$X")
        assert lines == ["ok"]

    async def test_interleaved_status_and_ok(self, rig):
        session, mock, emulator, events = rig
        # A status report fragment and the ack in one chunk: the
        # ack must still be extracted before line-based parsing.
        fut = asyncio.ensure_future(session.execute_interactive_command("$X"))
        await asyncio.sleep(0.05)
        mock.push(b"<Idle|MPos:0.000,0.000,0.000|Bf:15,128>\r\nok\r\n")
        assert await fut == ["ok"]
        assert session.buffer_count == 0

    async def test_commands_rejected_while_alarmed(self, rig):
        session, mock, emulator, events = rig
        emulator.settings["22"] = 1
        emulator.settings["20"] = 1
        # A move beyond the travel limit: the line is rejected with
        # an error (after ALARM:2) and the machine locks.
        lines = await session.execute_interactive_command("G0 X10000")
        assert "ALARM:2" in lines
        assert lines[-1] == "error:5"
        await events.wait_for(
            DriverEvent.StateChanged, lambda e: e.state.error is not None
        )
        # Like real Grbl, subsequent gcode is rejected.
        lines = await session.execute_interactive_command("G0 X1")
        assert lines == ["error:9"]
        # $X unlocks.
        lines = await session.execute_interactive_command("$X")
        assert "[MSG:Caution: Unlocked]" in lines
        assert lines[-1] == "ok"


class TestHold:
    async def test_hold_sends_realtime_and_updates_state(self, rig):
        session, mock, emulator, events = rig
        await session.set_hold(True)
        assert b"!" in mock.sent()
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status == DeviceStatus.HOLD,
        )
        await session.set_hold(False)
        assert b"~" in mock.sent()

    async def test_resume_without_job_reports_idle(self, rig):
        session, mock, emulator, events = rig
        await session.set_hold(True)
        await session.set_hold(False)
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status == DeviceStatus.IDLE,
        )


class TestDeviceOperations:
    async def test_move_to_reaches_target(self, rig):
        session, mock, emulator, events = rig
        await session.move_to(1500, 100, -50)
        await asyncio.sleep(0.1)
        assert emulator.mpos == pytest.approx([100.0, -50.0, 0.0])
        assert b"$J=G90 G21 F1500 X100.0 Y-50.0\n" in sent_bytes(mock)

    async def test_jog_reaches_target(self, rig):
        session, mock, emulator, events = rig
        await session.jog(1000, [("x", 10), ("y", -2.5)])
        await asyncio.sleep(0.1)
        assert emulator.mpos == pytest.approx([10.0, -2.5, 0.0])
        assert b"$J=G91 G21 F1000 X10.0 Y-2.5\n" in sent_bytes(mock)

    async def test_jog_no_axes_is_noop(self, rig):
        session, mock, emulator, events = rig
        mock.clear_sent()
        await session.jog(1000, [])
        assert sent_bytes(mock) == b""

    async def test_select_tool(self, rig):
        session, mock, emulator, events = rig
        await session.select_tool(2)
        assert b"T2\n" in sent_bytes(mock)

    async def test_set_power_reflected_in_parser_state(self, rig):
        session, mock, emulator, events = rig
        await session.set_power(500)
        lines = await session.execute_interactive_command("$G")
        assert "M4" in lines[0]
        assert "S500" in lines[0]

    async def test_set_power_off(self, rig):
        session, mock, emulator, events = rig
        await session.set_power(None)
        lines = await session.execute_interactive_command("$G")
        assert "M5" in lines[0]
        assert b"M5\n" in sent_bytes(mock)

    async def test_set_focus_power(self, rig):
        session, mock, emulator, events = rig
        await session.set_focus_power(100)
        lines = await session.execute_interactive_command("$G")
        assert "M3" in lines[0]

    async def test_home_cycle(self, rig):
        session, mock, emulator, events = rig
        await session.move_to(1500, 50, 50)
        await asyncio.sleep(0.05)
        emulator.settings["22"] = 1
        await session.home(None, "G54")
        await asyncio.sleep(0.1)
        assert emulator.mpos == pytest.approx([0.0, 0.0, 0.0])
        text = sent_bytes(mock)
        assert b"$H\n" in text
        assert b"G4 P0.01\n" in text
        assert b"G55\n" in text
        assert b"G54\n" in text

    async def test_home_disabled_reports_error(self, rig):
        session, mock, emulator, events = rig
        assert emulator.settings["22"] == 0
        lines = await session.execute_command("$H")
        assert lines == ["error:5"]

    async def test_home_uses_homing_timeout(self, mock, events):
        emulator, device_task = await start_device(mock, home_ack_delay=0.4)
        session = make_session(
            mock,
            events,
            {"command_timeout": 0.15, "homing_timeout": 2.0},
        )
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda event: event.status == "CONNECTED",
            )
            emulator.settings["22"] = 1
            await session.home(None, "G54")
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_home_times_out_with_short_homing_timeout(
        self, mock, events
    ):
        emulator, device_task = await start_device(mock, home_ack_delay=0.4)
        session = make_session(
            mock,
            events,
            {"command_timeout": 2.0, "homing_timeout": 0.05},
        )
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda event: event.status == "CONNECTED",
            )
            emulator.settings["22"] = 1
            with pytest.raises(asyncio.TimeoutError):
                await session.home(None, "G54")
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_home_without_homing_timeout_uses_command_timeout(
        self, mock, events
    ):
        emulator, device_task = await start_device(mock, home_ack_delay=0.4)
        session = make_session(
            mock,
            events,
            {"command_timeout": 0.05, "homing_timeout": 0.0},
        )
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda event: event.status == "CONNECTED",
            )
            emulator.settings["22"] = 1
            with pytest.raises(asyncio.TimeoutError):
                await session.home(None, "G54")
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_update_dialect(self, rig):
        session, mock, emulator, events = rig
        session.update_dialect({"move_to": "G0 X{x} Y{y}"})
        await session.move_to(1500, 10, 20)
        assert b"G0 X10.0 Y20.0\n" in sent_bytes(mock)


class TestSettings:
    async def test_read_settings(self, rig):
        session, mock, emulator, events = rig
        pairs = await session.read_settings()
        as_dict = dict(pairs)
        assert as_dict["13"] == "0"
        assert as_dict["110"] == "500.000"

    async def test_write_setting_persists(self, rig):
        session, mock, emulator, events = rig
        await session.write_setting("110", "750.000")
        assert emulator.settings["110"] == 750.0
        assert b"$110=750.000\n" in sent_bytes(mock)
        pairs = await session.read_settings()
        assert dict(pairs)["110"] == "750.000"

    async def test_detect_unit_system_metric(self, rig):
        session, mock, emulator, events = rig
        assert await session.detect_unit_system() == "metric"

    async def test_detect_unit_system_imperial(self, rig):
        session, mock, emulator, events = rig
        emulator.settings["13"] = 1
        assert await session.detect_unit_system() == "imperial"


class TestWcs:
    async def test_read_wcs_offsets(self, rig):
        session, mock, emulator, events = rig
        offsets = await session.read_wcs_offsets()
        assert offsets["G54"] == (0.0, 0.0, 0.0)
        await events.wait_for(DriverEvent.WcsUpdated)

    async def test_set_then_read_wcs_offset(self, rig):
        session, mock, emulator, events = rig
        await session.set_wcs_offset("G55", 10, 20, None)
        assert b"G10 L2 P2 X10.0 Y20.0\n" in sent_bytes(mock)
        offsets = await session.read_wcs_offsets()
        assert offsets["G55"] == (10.0, 20.0, 0.0)

    async def test_set_wcs_offset_with_z(self, rig):
        session, mock, emulator, events = rig
        await session.set_wcs_offset("G54", 1, 2, 3)
        assert b"G10 L2 P1 X1.0 Y2.0 Z3.0\n" in sent_bytes(mock)
        assert emulator.wcs["G54"] == [1.0, 2.0, 3.0]

    async def test_set_wcs_offset_invalid_slot(self, rig):
        session, mock, emulator, events = rig
        with pytest.raises(RuntimeError):
            await session.set_wcs_offset("G53", 1, 2, None)

    async def test_read_parser_state(self, rig):
        session, mock, emulator, events = rig
        assert await session.read_parser_state() == "G54"


class TestProbe:
    async def test_successful_probe(self, rig):
        session, mock, emulator, events = rig
        emulator.probe_touch = (0.0, 0.0, -5.0)
        pos = await session.run_probe_cycle("Z", 10.0, 100)
        assert pos == pytest.approx((0.0, 0.0, -5.0))
        await events.wait_for(
            DriverEvent.ProbeStatusChanged,
            lambda e: "triggered" in e.message,
        )
        assert emulator.mpos == pytest.approx([0.0, 0.0, -5.0])

    async def test_failed_probe_raises_alarm(self, rig):
        session, mock, emulator, events = rig
        # No contact configured: Grbl reports PRB:0 + ALARM:4.
        pos = await session.run_probe_cycle("Z", 10.0, 100)
        assert pos is None
        await events.wait_for(
            DriverEvent.ProbeStatusChanged,
            lambda e: e.message == "Probe failed",
        )
        event = await events.wait_for(
            DriverEvent.StateChanged, lambda e: e.state.error is not None
        )
        assert event.state.error.code == 4
        assert event.state.error.title.startswith("Probe Fail")

    async def test_probe_after_alarm_lock_can_unlock(self, rig):
        session, mock, emulator, events = rig
        await session.run_probe_cycle("Z", 10.0, 100)
        await events.wait_for(
            DriverEvent.StateChanged, lambda e: e.state.error is not None
        )
        lines = await session.execute_command("$X")
        assert "[MSG:Caution: Unlocked]" in lines

    async def test_probe_unit_conversion(self, rig):
        session, mock, emulator, events = rig
        emulator.settings["13"] = 1
        assert await session.detect_unit_system() == "imperial"
        emulator.probe_touch = (0.0, 0.0, -25.4)
        pos = await session.run_probe_cycle("Z", 50.0, 100)
        assert pos[2] == pytest.approx(-25.4)


class TestInchReports:
    async def test_imperial_reports_converted_to_mm(self, rig):
        session, mock, emulator, events = rig
        await session.move_to(1500, 25.4, 0)
        emulator.settings["13"] = 1
        assert await session.detect_unit_system() == "imperial"
        event = await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: (
                e.state.machine_pos[0] is not None
                and e.state.machine_pos[0] > 20.0
            ),
        )
        # The report arrived in inches; the driver reports mm.
        assert event.state.machine_pos[0] == pytest.approx(25.4, abs=0.1)


class TestAlarm:
    async def test_clear_alarm_sends_the_dialect_command(self, rig):
        session, mock, emulator, events = rig
        await session.clear_alarm()
        assert b"$X\n" in sent_bytes(mock)
