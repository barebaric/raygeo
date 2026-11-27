"""Sessions dropped without disconnect() must stop their background
tasks — no orphaned polling on the shared runtime, which marshals
events onto event loops that may already be closed (the Windows
access-violation source). Smoothie is covered in
test_smoothie_teardown_stress.py; this covers GRBL and Marlin.
"""

import asyncio
import gc

from conftest import DEFAULT_DIALECT

from raygeo.driver.core.transport import MockTransport
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.grbl.emulator import GrblEmulator
from raygeo.driver.marlin import MarlinSerialDriver
from raygeo.driver.marlin.emulator import MarlinEmulator

TIMINGS = {
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 0.05,
    "reconnect_delay": 0.2,
    "command_timeout": 2.0,
    "connect_settle": 0.05,
}


async def test_grbl_drop_without_disconnect_stops_tasks():
    mock = MockTransport()
    device_task = asyncio.ensure_future(GrblEmulator(mock).run())
    session = GrblSerialDriver.with_transport(
        mock,
        config=TIMINGS,
        dialect=DEFAULT_DIALECT,
        event_callback=lambda event: None,
    )
    try:
        await session.connect()
        for _ in range(200):
            if session.state.status.name != "UNKNOWN":
                break
            await asyncio.sleep(0.005)
        await asyncio.sleep(0.1)
        mock.clear_sent()

        del session
        gc.collect()
        # Let the Drop-triggered abort settle, then assert no further
        # writes (an in-flight write during the abort is tolerated).
        await asyncio.sleep(0.3)
        mock.clear_sent()
        await asyncio.sleep(0.5)
        assert mock.sent() == []
    finally:
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


async def test_marlin_drop_without_disconnect_stops_tasks():
    mock = MockTransport()
    device_task = asyncio.ensure_future(MarlinEmulator(mock).run())
    session = MarlinSerialDriver.with_transport(
        mock,
        config=TIMINGS,
        dialect=DEFAULT_DIALECT,
        event_callback=lambda event: None,
    )
    try:
        await session.connect()
        for _ in range(200):
            if session.state.status.name != "UNKNOWN":
                break
            await asyncio.sleep(0.005)
        await asyncio.sleep(0.1)
        mock.clear_sent()

        del session
        gc.collect()
        await asyncio.sleep(0.3)
        mock.clear_sent()
        await asyncio.sleep(0.5)
        assert mock.sent() == []
    finally:
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass
