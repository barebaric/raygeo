"""Stress tests against the firmware emulator: fragmented responses,
status reports interleaved with acknowledgements, long jobs."""

import asyncio

import pytest
from conftest import make_session, start_device

from raygeo.driver.core import DriverEvent


def gcode(n_lines):
    return "\n".join(f"G1 X{i % 50} F2000" for i in range(n_lines))


class TestStress:
    async def test_fragmented_output_long_job(self, mock, events):
        # The emulator splits every response into 1-12 byte pieces,
        # exactly like noisy serial delivery.
        emulator, device_task = await start_device(
            mock, speed_factor=5000, fragment_output=True
        )
        session = make_session(mock, events)
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
                timeout=5.0,
            )
            done = []
            n = 400
            op_map = {i: i for i in range(n)}
            await asyncio.wait_for(
                session.run(gcode(n), op_map, [], done.append),
                timeout=60.0,
            )
            await asyncio.sleep(0.1)
            assert session.buffer_count == 0
            assert session.pending_commands == []
            assert len(done) == n
            assert emulator.rx_overrun is False
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_polling_during_job_stays_stable(self, mock, events):
        emulator, device_task = await start_device(mock, speed_factor=1000)
        session = make_session(
            mock, events, {"poll_status_while_running": True}
        )
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
                timeout=5.0,
            )
            n = 200
            await asyncio.wait_for(session.run(gcode(n)), timeout=60.0)
            await asyncio.sleep(0.2)
            assert session.buffer_count == 0
            assert emulator.rx_overrun is False
            # Status reports kept flowing throughout the job.
            reports = emulator.tx_log.count(b"<")
            assert reports > 5
        finally:
            await session.disconnect()
            device_task.cancel()

    async def test_1000_line_job(self, mock, events):
        emulator, device_task = await start_device(mock, speed_factor=10000)
        session = make_session(mock, events)
        try:
            await session.connect()
            await events.wait_for(
                DriverEvent.ConnectionStatusChanged,
                lambda e: e.status == "CONNECTED",
                timeout=5.0,
            )
            done = []
            n = 1000
            op_map = {i: i for i in range(n)}
            await asyncio.wait_for(
                session.run(gcode(n), op_map, [], done.append),
                timeout=120.0,
            )
            await asyncio.sleep(0.1)
            assert session.buffer_count == 0
            assert session.pending_commands == []
            assert len(done) == n
            assert emulator.rx_overrun is False
        finally:
            await session.disconnect()
            device_task.cancel()


pytestmark = pytest.mark.stress
