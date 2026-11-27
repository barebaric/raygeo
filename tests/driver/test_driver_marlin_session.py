"""The Marlin session: `start` handshake, phantom-port detection,
M114 position reports, strict ping-pong ordering and cancellation.

The strict-ordering tests drive a scripted MockTransport without the
emulator; the behavioral tests run against the Marlin firmware
emulator."""

import asyncio

import pytest
from conftest import DEFAULT_DIALECT, sent_bytes, wait_until

from raygeo.driver.core import DriverEvent
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.core.types import DeviceStatus
from raygeo.driver.marlin import MarlinSerialDriver
from raygeo.driver.marlin.emulator import MarlinEmulator
from raygeo.driver.marlin.parser import parse_m115_firmware_info

GCODE = "G1 X1 F1500\nG1 X2 F1500\nG1 X3 F1500\n"

TIMINGS = {
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 0.05,
    "reconnect_delay": 0.2,
    "command_timeout": 2.0,
    "connect_settle": 0.05,
}

BOOT = b"start\r\nMarlin 2.1.2\r\necho:Last Updated: 2024-01-01\r\n"


def gcode_sends(mock):
    """Command lines written by the session (M114 polls
    excluded)."""
    return [chunk for chunk in mock.sent() if chunk.startswith(b"G1 ")]


def make_marlin_session(mock, events, config=None):
    return MarlinSerialDriver.with_transport(
        mock,
        config={**TIMINGS, **(config or {})},
        dialect=DEFAULT_DIALECT,
        event_callback=events.callback,
    )


async def connect_scripted(mock, events):
    """Connect a session to a device that sends the `start`
    handshake but is otherwise mute.  The idle M114 poll is
    disabled so the test's scripted acknowledgements map one-to-one
    to the streamed lines."""
    session = make_marlin_session(
        mock, events, config={"status_poll_interval": 60.0}
    )
    connect_task = asyncio.ensure_future(session.connect())
    # The session sends nothing on connect; wait for the transport
    # link to open before scripting the handshake.
    await wait_until(lambda: mock.is_connected)
    mock.push(BOOT)
    await connect_task
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
    )
    mock.clear_sent()
    events.events.clear()
    return session


class TestHandshake:
    async def test_start_completes_connection(self, mock, events):
        session = await connect_scripted(mock, events)
        assert session.state is not None
        await session.disconnect()

    async def test_phantom_port_reports_error_and_retries(self, mock, events):
        session = make_marlin_session(mock, events)
        await session.connect()
        # No `start` within the handshake timeout: the port is
        # flagged as a phantom, then the loop retries (SLEEPING).
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda e: (
                e.status == "ERROR" and e.message == "No response from device"
            ),
            timeout=5,
        )
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda e: e.status == "SLEEPING",
            timeout=5,
        )
        await session.disconnect()

    async def test_unsolicited_ack_connects_without_start(self, mock, events):
        # Custom forks announce themselves with a banner and bare
        # `ok` acks instead of `start` (e.g. plotter firmware
        # printing `PR` on boot); any unprompted output proves a
        # live device.
        session = make_marlin_session(
            mock, events, config={"status_poll_interval": 60.0}
        )
        await session.connect()
        await wait_until(lambda: mock.is_connected)
        mock.push(b"PR\r\n")
        mock.push(b"ok\r\n")
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "CONNECTED",
            timeout=5,
        )
        await session.disconnect()

    async def test_boot_banner_connects_without_start(self, mock, events):
        session = make_marlin_session(
            mock, events, config={"status_poll_interval": 60.0}
        )
        await session.connect()
        await wait_until(lambda: mock.is_connected)
        mock.push(b"Marlin 2.1.2\r\necho:SD card ok\r\n")
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "CONNECTED",
            timeout=5,
        )
        await session.disconnect()


class TestPositionReports:
    async def test_m114_updates_state(self, mock, events):
        session = await connect_scripted(mock, events)
        mock.push(b"X:1.50 Y:-2.00 Z:0.00 E:0.00 Count X:150 Y:-200 Z:0\r\n")
        mock.push(b"ok\r\n")
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.machine_pos == [1.5, -2.0, 0.0],
        )
        assert session.state.status == DeviceStatus.IDLE
        await session.disconnect()


class TestStrictOrdering:
    async def test_next_line_waits_for_ack(self, mock, events):
        session = await connect_scripted(mock, events)
        job = asyncio.ensure_future(session.run(GCODE))

        await wait_until(lambda: len(gcode_sends(mock)) >= 1)
        await asyncio.sleep(0.3)
        assert len(gcode_sends(mock)) == 1

        mock.push(b"ok\r\n")
        await wait_until(lambda: len(gcode_sends(mock)) >= 2)
        await asyncio.sleep(0.3)
        assert len(gcode_sends(mock)) == 2

        mock.push(b"ok\r\n")
        await wait_until(lambda: len(gcode_sends(mock)) >= 3)
        mock.push(b"ok\r\n")

        await asyncio.wait_for(job, 5)
        await events.wait_for(DriverEvent.JobFinished)
        await session.disconnect()

    async def test_device_error_halts_stream_with_safety_shutdown(
        self, mock, events
    ):
        session = await connect_scripted(mock, events)
        job = asyncio.ensure_future(session.run(GCODE))
        await wait_until(lambda: len(gcode_sends(mock)) >= 1)
        mock.push(b"ok\r\n")
        await wait_until(lambda: len(gcode_sends(mock)) >= 2)
        mock.push(b"Error:Line number is not a line number\r\n")

        await asyncio.wait_for(job, 5)
        await events.wait_for(DriverEvent.JobFinished)
        await asyncio.sleep(0.2)
        assert len(gcode_sends(mock)) == 2
        assert session.state.error is not None
        assert b"M5\n" in sent_bytes(mock)
        await session.disconnect()

    async def test_cancel_sends_m410(self, mock, events):
        session = await connect_scripted(mock, events)
        job = asyncio.ensure_future(session.run(GCODE))
        await wait_until(lambda: len(gcode_sends(mock)) >= 1)

        await session.cancel()
        await asyncio.wait_for(job, 5)
        await events.wait_for(DriverEvent.JobFinished)
        sent = sent_bytes(mock)
        assert b"M410\n" in sent
        assert b"M5\n" in sent
        await session.disconnect()


@pytest.fixture
async def marlin_rig(mock, events):
    """A connected session running against the Marlin firmware
    emulator."""
    emulator = MarlinEmulator(mock)
    device_task = asyncio.ensure_future(emulator.run())
    session = make_marlin_session(mock, events)
    await session.connect()
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
    )
    mock.clear_sent()
    events.events.clear()
    try:
        yield session, mock, emulator, events
    finally:
        await session.disconnect()
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


class TestEmulatorJobs:
    async def test_job_completes_and_moves(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        await session.run("G1 X10 F90000\nG1 Y5 F90000\n")
        await events.wait_for(DriverEvent.JobFinished)
        await asyncio.sleep(0.1)
        assert emulator.mpos == pytest.approx([10.0, 5.0, 0.0])

    async def test_m114_reports_reach_state(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        await session.run_raw("G1 X7 F90000")
        await events.wait_for(DriverEvent.JobFinished)
        # The periodic M114 poll picks up the new position.
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.machine_pos == [7.0, 0.0, 0.0],
            timeout=10,
        )

    async def test_progress_callbacks_fire_per_op(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        emulator.speed_factor = 50
        done = []
        text = "\n".join(f"G1 X{i % 50} F90000" for i in range(40))
        op_map = {**{i: 0 for i in range(20)}, 30: 1}
        await session.run(text, op_map, progress_callback=done.append)
        await events.wait_for(DriverEvent.JobFinished, timeout=30)
        assert done == [0, 1]

    async def test_m115_probing(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        lines = await session.execute_interactive_command("M115")
        assert "ok" not in lines
        info = parse_m115_firmware_info(lines)
        assert info["firmware_name"].startswith("Marlin")

    async def test_detect_unit_system(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        assert await session.detect_unit_system() == "metric"


class TestMarlinDriverBehavior:
    async def test_boot_lines_are_captured(self, events):
        mock = MockTransport()
        session = await connect_scripted(mock, events)
        try:
            assert any("Marlin" in line for line in session.boot_lines)
        finally:
            await session.disconnect()

    async def test_set_hold_is_a_best_effort_noop(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        await session.set_hold(True)
        await session.set_hold(False)

    async def test_jog_restores_absolute_mode(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        before = len(mock.sent())
        await session.jog(1000, deltas=[("x", 1.0)])
        sent = b"".join(mock.sent()[before:])
        assert b"G90" in sent

    async def test_disconnect_resets_the_device_status(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        await session.disconnect()
        assert session.state.status == DeviceStatus.UNKNOWN
