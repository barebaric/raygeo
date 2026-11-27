"""Stress the GRBL reconnect/teardown path.

The intermittent Windows access violation happened when a failed
reconnect attempt left its RX reader behind and the session was torn
down at the same time. Churn connect/disconnect cycles against a
silent transport so every reader must be reaped before the next
connection starts.
"""

import asyncio
import gc

import pytest
from conftest import DEFAULT_DIALECT

from raygeo.driver.core.transport import MockTransport
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.grbl.emulator import GrblEmulator

TIMINGS = {
    "handshake_timeout": 0.05,
    "handshake_poll_interval": 0.01,
    "status_poll_interval": 60.0,
    "reconnect_delay": 0.01,
    "command_timeout": 1.0,
}

JOB_TIMINGS = {
    **TIMINGS,
    "handshake_timeout": 2.0,
    "status_poll_interval": 0.05,
    "reconnect_delay": 0.2,
    "command_timeout": 2.0,
}


async def _reconnect_cycle(disconnect: bool) -> None:
    mock = MockTransport()
    session = GrblSerialDriver.with_transport(
        mock,
        config=TIMINGS,
        dialect=DEFAULT_DIALECT,
        event_callback=lambda event: None,
    )
    try:
        await session.connect()
        # Let the handshake fail so the connection loop reopens the
        # transport and spawns another reader.
        await asyncio.sleep(0.06)
    finally:
        if disconnect:
            await session.disconnect()
    del session
    gc.collect()


@pytest.mark.asyncio
async def test_reconnect_disconnect_churn():
    for _ in range(40):
        await _reconnect_cycle(disconnect=True)


@pytest.mark.asyncio
async def test_reconnect_drop_churn():
    for _ in range(40):
        await _reconnect_cycle(disconnect=False)


async def _stream_disconnect_cycle() -> None:
    mock = MockTransport()
    emulator = GrblEmulator(mock)
    device_task = asyncio.ensure_future(emulator.run())
    session = GrblSerialDriver.with_transport(
        mock,
        config=JOB_TIMINGS,
        dialect=DEFAULT_DIALECT,
        event_callback=lambda event: None,
    )
    try:
        await session.connect()
        for _ in range(200):
            if session.state.status.name != "UNKNOWN":
                break
            await asyncio.sleep(0.005)
        job = asyncio.ensure_future(
            session.run("".join(f"G1 X{i % 50} F600\n" for i in range(60)))
        )
        await asyncio.sleep(0.02)
        await session.disconnect()
        try:
            await asyncio.wait_for(job, 2.0)
        except Exception:
            pass
    finally:
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass
    del session
    gc.collect()


@pytest.mark.asyncio
async def test_stream_disconnect_churn():
    for _ in range(30):
        await _stream_disconnect_cycle()


pytestmark = pytest.mark.stress
