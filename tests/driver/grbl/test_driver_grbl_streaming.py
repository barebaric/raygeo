"""Job streaming tests against the firmware emulator: backpressure,
planner-deferred acknowledgements, stalls, alarms and cancel."""

import asyncio
import time

import pytest
from conftest import (
    make_session,
    sent_bytes,
    start_device,
    wait_until,
)

from raygeo.driver.core import DriverEvent
from raygeo.driver.core.types import DeviceStatus
from raygeo.driver.grbl.emulator import WELCOME


def gcode(n_lines):
    return "\n".join(f"G1 X{i % 50} F1000" for i in range(n_lines))


def slow_gcode(n_lines):
    """Long alternating moves: each block runs for seconds at low
    speed factors, keeping the planner full and the acks deferred."""
    return "\n".join(f"G1 X{50 if i % 2 else 0} F600" for i in range(n_lines))


class TestRunBasics:
    async def test_simple_job_completes_and_moves(self, rig):
        session, mock, emulator, events = rig
        await session.run("G1 X10 F1000\nG1 Y5 F1000\n")
        await events.wait_for(DriverEvent.JobFinished)
        await asyncio.sleep(0.1)
        assert session.buffer_count == 0
        assert session.pending_commands == []
        assert emulator.mpos == pytest.approx([10.0, 5.0, 0.0])

    async def test_progress_callbacks_fire_per_op(self, rig):
        session, mock, emulator, events = rig
        done = []
        gcode_text = "G1 X1 F1000\nG1 X2 F1000\nG1 X3 F1000\nG1 X4 F1000\n"
        op_map = {0: 0, 1: 0, 3: 1}
        await session.run(gcode_text, op_map, [], done.append)
        await asyncio.sleep(0.1)
        assert done == [0, 1]

    async def test_comments_and_empty_lines_skipped(self, rig):
        session, mock, emulator, events = rig
        await session.run(
            "G1 X1 F1000 ; comment\n\n(comment only)\nG1 X2 F1000\n"
        )
        await events.wait_for(DriverEvent.JobFinished)
        sent = sent_bytes(mock)
        assert b"comment" not in sent
        assert b"G1 X1 F1000\n" in sent
        assert b"G1 X2 F1000\n" in sent

    async def test_job_status_reflected_as_run(self, rig):
        session, mock, emulator, events = rig
        emulator.speed_factor = 50  # ~0.5 s of motion
        job = asyncio.ensure_future(session.run(gcode(30)))
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        emulator.speed_factor = 100000
        await asyncio.wait_for(job, timeout=10)
        await events.wait_for(DriverEvent.JobFinished)

    async def test_long_job_never_overflows_rx_buffer(self, rig):
        session, mock, emulator, events = rig
        emulator.speed_factor = 500
        # More lines than planner + RX buffer combined: the emulator
        # must never see an RX overflow from the character-counting
        # sender.
        await asyncio.wait_for(session.run(gcode(60)), timeout=30)
        await events.wait_for(DriverEvent.JobFinished)
        await asyncio.sleep(0.1)
        assert emulator.rx_overrun is False
        assert session.buffer_count == 0


class TestErrorsDuringJob:
    async def test_error_halts_stream(self, rig):
        session, mock, emulator, events = rig
        emulator.speed_factor = 100
        lines = [f"G1 X{i} F1000" for i in range(40)]
        lines[5] = "G999"
        job = asyncio.ensure_future(session.run("\n".join(lines)))
        await asyncio.wait_for(job, timeout=10)
        await events.wait_for(DriverEvent.JobFinished)
        await asyncio.sleep(0.1)
        sent = sent_bytes(mock)
        assert sent.count(b"G1 X") < 40
        # The interrupt handler ran an emergency cancel.
        assert b"\x18" in sent
        # Soft reset reboots the emulator: welcome banner again.
        assert emulator.tx_log.count(WELCOME) >= 2
        states = [
            e.state
            for e in events.events
            if isinstance(e, DriverEvent.StateChanged)
        ]
        assert any(s.error is not None and s.error.code == 20 for s in states)
        # run() resolves normally even on failure, so the terminal
        # error must be readable from the device state afterwards.
        # The protocol error (proper GRBL code) is preserved.
        assert session.state.error is not None
        assert session.state.error.code == 20
        # An emergency reset racing in-flight motion leaves the
        # machine alarm-locked (real Grbl behaves the same).  Recover
        # like an operator would, then wait for the between-jobs
        # status poller to reflect Idle before starting the next job.
        await session.execute_interactive_command("$X")
        deadline = time.monotonic() + 5
        while (
            session.state.status != DeviceStatus.IDLE
            and time.monotonic() < deadline
        ):
            await asyncio.sleep(0.05)
        assert session.state.status == DeviceStatus.IDLE
        # A subsequent job start clears the error again.
        job = asyncio.ensure_future(session.run(gcode(5)))
        await asyncio.wait_for(job, timeout=10)
        await events.wait_for(DriverEvent.JobFinished)
        assert session.state.error is None

    async def test_soft_limit_alarm_halts_stream(self, rig):
        session, mock, emulator, events = rig
        emulator.settings["22"] = 1
        emulator.settings["20"] = 1
        emulator.speed_factor = 100
        lines = [f"G1 X{i} F1000" for i in range(40)]
        lines[3] = "G1 X10000 F1000"
        job = asyncio.ensure_future(session.run("\n".join(lines)))
        await asyncio.wait_for(job, timeout=10)
        await events.wait_for(DriverEvent.JobFinished)
        await asyncio.sleep(0.1)
        assert sent_bytes(mock).count(b"G1 X") < 40
        states = [
            e.state
            for e in events.events
            if isinstance(e, DriverEvent.StateChanged)
        ]
        assert any(
            s.error is not None and s.error.title == "Soft Limit"
            for s in states
        )


class TestStallBehavior:
    async def test_stall_retries_while_device_alive(self, mock, events):
        emulator, device_task = await start_device(mock, speed_factor=2)
        session = make_session(mock, events)
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
            )
            mock.clear_sent()
            # Slow motion: the planner stays full and acks stop,
            # filling the host buffer.  The device still answers
            # polls (Run), so the streamer must keep retrying
            # rather than abort.
            job = asyncio.ensure_future(session.run(slow_gcode(40)))
            await asyncio.sleep(1.5)
            assert not job.done()
            assert sent_bytes(mock).count(b"G1 X") < 40
            # Speed the machine up: the job drains and completes.
            emulator.speed_factor = 100000
            await asyncio.wait_for(job, timeout=30)
            await events.wait_for(DriverEvent.JobFinished)
            assert emulator.rx_overrun is False
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_stall_aborts_when_device_dies(self, mock, events):
        emulator, device_task = await start_device(mock, speed_factor=2)
        session = make_session(mock, events)
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
            )
            mock.clear_sent()
            emulator.silent = True  # the cable is gone
            await asyncio.wait_for(session.run(slow_gcode(40)), timeout=40)
            await events.wait_for(DriverEvent.JobFinished)
            assert sent_bytes(mock).count(b"G1 X") < 40
            # No protocol error was seen, so run() must surface the
            # job-level failure ("device stopped responding") through
            # the device state, otherwise callers cannot tell a dead
            # job from a completed one.
            error = session.state.error
            assert error is not None
            assert error.title == "Job Error"
            assert "stopped responding" in error.description
        finally:
            await session.disconnect()
            device_task.cancel()


class TestCancel:
    async def test_cancel_mid_job_sends_reset_and_safety_shutdown(self, rig):
        session, mock, emulator, events = rig
        emulator.speed_factor = 2
        job = asyncio.ensure_future(session.run(gcode(40)))
        await asyncio.sleep(0.3)
        assert not job.done()
        mock.clear_sent()
        emulator.tx_log.clear()
        await session.cancel()
        await asyncio.sleep(0.2)
        sent = sent_bytes(mock)
        assert b"\x18" in sent
        assert b"M5\n" in sent
        assert b"M9\n" in sent
        # Soft reset mid-motion reboots Grbl into the alarm lock.
        assert emulator.tx_log.count(WELCOME) >= 1
        finished = [
            e for e in events.events if isinstance(e, DriverEvent.JobFinished)
        ]
        assert len(finished) == 1

    async def test_cancel_without_job(self, rig):
        session, mock, emulator, events = rig
        mock.clear_sent()
        await session.cancel()
        await asyncio.sleep(0.1)
        sent = sent_bytes(mock)
        assert b"\x18" in sent
        assert b"M5\n" in sent

    async def test_cancel_emergency_adds_estop(self, rig):
        session, mock, emulator, events = rig
        session.update_dialect({"emergency_stop": "M112"})
        mock.clear_sent()
        await session.cancel(emergency=True)
        await asyncio.sleep(0.1)
        assert b"M112\n" in sent_bytes(mock)

    async def test_cancelling_the_python_task_stops_the_job(self, rig):
        """Cancelling await session.run() runs the safety shutdown."""
        session, mock, emulator, events = rig
        emulator.speed_factor = 2
        job = asyncio.ensure_future(session.run(slow_gcode(200)))
        await events.wait_for(
            DriverEvent.StateChanged,
            lambda e: e.state.status.name == "RUN",
        )
        await asyncio.sleep(0.2)
        job.cancel()
        with pytest.raises(asyncio.CancelledError):
            await job
        await wait_until(lambda: not session.job_running, timeout=5)
        await wait_until(lambda: b"M5\n" in sent_bytes(mock), timeout=5)
        assert b"\x18" in sent_bytes(mock)

    async def test_cancelled_sender_does_not_resurrect(self, rig):
        """Issue #428 regression: after cancel(), a sender parked on a
        full buffer must never resume streaming the cancelled job."""
        session, mock, emulator, events = rig
        emulator.speed_factor = 2
        job = asyncio.ensure_future(session.run(slow_gcode(40)))
        await asyncio.sleep(0.5)
        assert not job.done()
        await session.cancel()
        await asyncio.sleep(0.2)
        assert job.done()

        def data_sent():
            return b"".join(c for c in mock.sent() if c != b"?")

        sent_after = data_sent()
        assert sent_after.count(b"G1 X") < 40
        # Late acks and free buffer must not resurrect the job.
        mock.push(b"ok\r\n" * 60)
        await asyncio.sleep(0.5)
        assert data_sent() == sent_after

        # Commands work again once the cancelled job wound down.  The
        # soft reset locked the emulator, so unlock first.
        lines = await asyncio.wait_for(
            session.execute_command("$X"), timeout=5
        )
        assert lines[-1] == "ok"


class TestRunRaw:
    async def test_realtime_commands_bypass_stream(self, rig):
        session, mock, emulator, events = rig
        mock.clear_sent()
        await session.run_raw("?")
        await asyncio.sleep(0.1)
        sent = sent_bytes(mock)
        assert b"?" in sent
        assert sent.count(b"\n") == 0

    async def test_mixed_raw(self, rig):
        session, mock, emulator, events = rig
        mock.clear_sent()
        await session.run_raw("G1 X1 F1000\n!\nG1 X2 F1000\n")
        await events.wait_for(DriverEvent.JobFinished)
        sent = sent_bytes(mock)
        assert b"G1 X1 F1000\n" in sent
        assert b"!" in sent
        assert b"G1 X2 F1000\n" in sent
