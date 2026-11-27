"""Ruida session tests, driven against the Ruida controller emulator
(raygeo.driver.ruida.emulator), plus byte-level golden tests for the
rpascript codec."""

from pathlib import Path

import pytest
import pytest_asyncio
from conftest import EventRecorder, wait_until

from raygeo.driver import driver_info
from raygeo.driver.analyzer.capture import RX, TX, Recorder
from raygeo.driver.core import DriverEvent
from raygeo.driver.ruida import RuidaDriver, encode_script
from raygeo.driver.ruida.emulator import RuidaEmulator, swizzle_byte

GOLDEN = Path(__file__).parent / "golden"
FAST = {
    "timeout": 0.2,
    "query_interval": 0.1,
    "ping_retries": 2,
    "ping_delay": 0.05,
}


def make_session(events, emulator, **config):
    callback = events.callback if hasattr(events, "callback") else events
    return RuidaDriver(
        config={
            "host": "127.0.0.1",
            "port": emulator.port,
            "protocol": "tcp",
            **FAST,
            **config,
        },
        event_callback=callback,
    )


@pytest_asyncio.fixture
async def emulator():
    emu = RuidaEmulator()
    await emu.start()
    try:
        yield emu
    finally:
        await emu.stop()


@pytest_asyncio.fixture
async def rig(emulator, events):
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


class TestCodec:
    def test_staged_job_matches_golden_bytes(self):
        lines = (GOLDEN / "job.rds").read_text().splitlines()
        expected = (GOLDEN / "job.hex").read_text().strip()
        actual = bytes(encode_script(lines)).hex().upper()
        assert actual == expected

    def test_live_commands_match_golden_bytes(self):
        lines = (GOLDEN / "live.rds").read_text().splitlines()
        expected = (GOLDEN / "live.hex").read_text().strip()
        actual = bytes(encode_script(lines)).hex().upper()
        assert actual == expected

    def test_comments_and_expected_values_are_ignored(self):
        script = [
            "# a comment",
            "SPEED_LASER_1 100.0mm/S  # trailing",
            "JOG_XY Rel:MACHINE X=1mm Y=2mm = 0x00",
        ]
        with_comments = encode_script(script)
        without_comments = encode_script(
            [
                "SPEED_LASER_1 100.0mm/S",
                "JOG_XY Rel:MACHINE X=1mm Y=2mm",
            ]
        )
        assert with_comments == without_comments

    def test_unknown_command_raises(self):
        with pytest.raises(ValueError, match="unknown rpascript"):
            encode_script(["NOT_A_COMMAND"])

    def test_end_job_checksum_covers_eof(self):
        raw = bytes(encode_script(["START_JOB", "END_JOB", "EOF"]))
        checksum = raw[raw.index(0xE5) + 2 : raw.index(0xE5) + 7]
        # START_JOB (D8 00) + EOF (D7) = 431.
        assert checksum == b"\x00\x00\x00\x03\x2f"


class TestMetadata:
    def test_info(self):
        info = driver_info("RuidaDriver")
        assert info.name == "RuidaDriver"
        assert info.uses_gcode is False
        assert info.capabilities["supports_hold"] is True
        assert info.capabilities["supports_jog"] is True
        assert info.machine_space_wcs == "MACHINE"

    def test_repr_and_uri(self, emulator, events):
        session = make_session(events, emulator)
        assert session.resource_uri == f"tcp://127.0.0.1:{emulator.port}"
        assert "RuidaDriver" in repr(session)


class TestConnection:
    async def test_connect_reports_identity(self, rig):
        session, emulator, _events = rig
        status = session.machine_status
        assert status["card_id"] == emulator.card_id
        assert status["bed_size"] == [400.0, 400.0]
        assert status["mainboard_version"] == "RDC6442S"
        assert status["features"] == 1

    async def test_connect_failure_reports_error(self, events):
        emulator = RuidaEmulator()
        await emulator.start()
        port = emulator.port
        await emulator.stop()
        session = make_session(events, type("E", (), {"port": port})())
        await session.connect()
        event = await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "ERROR",
        )
        assert event.message
        await session.disconnect()

    async def test_magic_is_detected_from_the_ack(self, events):
        emulator = RuidaEmulator(magic=0x99)
        await emulator.start()
        try:
            session = make_session(events, emulator)
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda event: event.status == "CONNECTED",
            )
            await session.disconnect()
        finally:
            await emulator.stop()


class TestStatus:
    async def test_running_and_paused_states(self, rig):
        session, emulator, _events = rig
        emulator.set_job_running(True)
        await wait_until(lambda: session.state.status.name == "RUN")
        emulator.set_paused(True)
        await wait_until(lambda: session.state.status.name == "HOLD")
        emulator.set_job_running(False)
        emulator.set_paused(False)
        await wait_until(lambda: session.state.status.name == "IDLE")

    async def test_position_is_reported_in_mm(self, rig):
        session, emulator, _events = rig
        emulator.set_position(12.5, -3.25, 1.0)
        await wait_until(
            lambda: (
                session.state.machine_pos[0] is not None
                and session.state.machine_pos[0] > 12.0
            )
        )
        assert session.state.machine_pos[0] == pytest.approx(12.5)
        assert session.state.machine_pos[1] == pytest.approx(-3.25)
        assert session.state.machine_pos[2] == pytest.approx(1.0)


class TestJobs:
    async def test_run_sends_the_script_and_finishes(self, rig):
        session, emulator, events = rig
        await session.run("SPEED_LASER_1 100.0mm/S\nEND_JOB\n", {}, [])
        await events.wait_for(DriverEvent.JobFinished)
        assert any("e505" in command for command in emulator.commands)

    async def test_run_raw_sends_commands(self, rig):
        session, emulator, events = rig
        await session.run_raw("SPEED_LASER_1 100.0mm/S\nEND_JOB")
        await events.wait_for(DriverEvent.JobFinished)
        assert emulator.commands

    async def test_empty_script_still_finishes(self, rig):
        _session, emulator, events = rig
        before = len(emulator.commands)
        await _session.run("", {}, [])
        await events.wait_for(DriverEvent.JobFinished)
        assert len(emulator.commands) == before


class TestLiveCommands:
    async def test_home_and_jog(self, rig):
        session, emulator, _events = rig
        await session.home_xy()
        await session.jog_xy_to(10.0, 20.0)
        await session.jog_x_to(5.0)
        await session.jog_y_to(6.0)
        await session.jog_xy_rel(1.0, -1.0)
        assert any("d910" in command for command in emulator.commands)
        assert any("d900" in command for command in emulator.commands)
        assert any("d901" in command for command in emulator.commands)

    async def test_z_and_u_commands(self, rig):
        session, emulator, _events = rig
        await session.home_z()
        await session.focus_z()
        await session.jog_z_to(3.0)
        await session.jog_u_to(4.0)
        assert any("d82c" in command for command in emulator.commands)
        assert any("d82e" in command for command in emulator.commands)

    async def test_jog_z_and_u_axes(self, rig):
        session, emulator, _events = rig
        await session.jog(600.0, [("z", 1.0), ("u", 2.0)])
        assert any("d902" in command for command in emulator.commands)
        assert any("d903" in command for command in emulator.commands)

    async def test_hold_and_cancel(self, rig):
        session, emulator, _events = rig
        emulator.set_job_running(True)
        await wait_until(lambda: session.state.status.name == "RUN")
        await session.set_hold(True)
        await wait_until(lambda: session.state.status.name == "HOLD")
        await session.set_hold(False)
        await session.cancel()
        assert any("d801" in command for command in emulator.commands)


class TestRawTraffic:
    async def test_raw_bytes_are_reported(self, rig):
        session, emulator, events = rig
        events.events.clear()
        await session.jog_x_to(5.0)
        tx = await events.wait_for(DriverEvent.RawTxBytes)
        assert bytes(tx.data)
        ack = bytes([swizzle_byte(0xCC, emulator.magic)])
        await events.wait_for(
            DriverEvent.RawRxBytes,
            lambda event: bytes(event.data) == ack,
        )

    async def test_analyzer_recorder_captures_wire_frames(self, emulator):
        events = EventRecorder()
        recorder = Recorder(forward=events.callback)
        session = make_session(recorder, emulator)
        await session.connect()
        await events.wait_for(
            DriverEvent.ConnectionStatusChanged,
            lambda event: event.status == "CONNECTED",
        )
        try:
            await session.jog_x_to(5.0)
            await events.wait_for(DriverEvent.RawRxBytes)
        finally:
            await session.disconnect()
        directions = {frame.direction for frame in recorder.capture.frames}
        assert directions == {TX, RX}
        assert all(frame.data is not None for frame in recorder.capture.frames)
        assert all(frame.text is None for frame in recorder.capture.frames)


class TestMemory:
    async def test_get_setting(self, rig):
        session, emulator, _events = rig
        replies = await session.get_setting(0x0400)
        assert replies
        assert replies[0][0] == 0x0400

    async def test_set_setting_is_protected_by_default(self, rig):
        session, _emulator, _events = rig
        with pytest.raises(NotImplementedError):
            await session.set_setting(0x0012, 100, 100)

    async def test_set_setting_with_protect_off(self, rig):
        session, emulator, _events = rig
        session.set_protect(False)
        await session.set_setting(0x0012, 100, 100)
        assert any("da01" in command for command in emulator.commands)
