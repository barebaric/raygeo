"""Stress the Smoothie cancel/teardown path.

Reproduces the churn that preceded the intermittent Windows access
violation: many connect/run/cancel/disconnect/drop cycles with GC,
including dropping a session without an explicit disconnect so the
last session reference is released from a tokio worker. Guards the
teardown path against regressions; the runtime's GIL watchdog is
opt-in (RAYDRIVER_GIL_WATCHDOG) so it does not perturb these runs.
"""

import asyncio
import gc

import pytest

from raygeo.driver.core.transport import MockTransport
from raygeo.driver.smoothie import SmoothieDriver
from raygeo.driver.smoothie.emulator import SmoothieEmulator

GCODE = "".join(f"G1 X{i} F6000\n" for i in range(20))
TIMINGS = {
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 60.0,
    "command_timeout": 2.0,
    "safety_shutdown_delay": 0.01,
}
DIALECT = {"safety_off_commands": ["M5"]}


async def _cancel_cycle(disconnect: bool) -> None:
    mock = MockTransport()
    emulator = SmoothieEmulator(mock)
    device_task = asyncio.ensure_future(emulator.run())
    session = SmoothieDriver.with_transport(
        mock,
        config=TIMINGS,
        dialect=DIALECT,
        event_callback=lambda event: None,
    )
    try:
        await session.connect()
        for _ in range(100):
            if session.state.status.name != "UNKNOWN":
                break
            await asyncio.sleep(0.002)
        job = asyncio.ensure_future(session.run(GCODE))
        for _ in range(200):
            if len(mock.sent()) >= 2:
                break
            await asyncio.sleep(0.001)
        await session.cancel()
        try:
            await asyncio.wait_for(job, 3.0)
        except Exception:
            pass
        if disconnect:
            await session.disconnect()
    finally:
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass
    del session
    gc.collect()


@pytest.mark.asyncio
async def test_cancel_disconnect_churn():
    for _ in range(60):
        await _cancel_cycle(disconnect=True)


@pytest.mark.asyncio
async def test_cancel_drop_without_disconnect_churn():
    for _ in range(60):
        await _cancel_cycle(disconnect=False)


@pytest.mark.asyncio
async def test_drop_without_disconnect_stops_background_tasks():
    """A session dropped without disconnect() must not orphan its
    polling tasks on the shared runtime (the leak that destabilized
    the Windows CI): after the drop, no further status polls reach
    the transport."""
    status = b"<Idle|MPos:0.000,0.000,0.000|WPos:0.000,0.000,0.000|FS:0,0>\r\n"
    mock = MockTransport()
    session = SmoothieDriver.with_transport(
        mock,
        config={**TIMINGS, "status_poll_interval": 0.05},
        dialect=DIALECT,
        event_callback=lambda event: None,
    )
    connect_task = asyncio.ensure_future(session.connect())
    for _ in range(200):
        if mock.sent():
            break
        await asyncio.sleep(0.005)
    mock.push(status)
    await connect_task
    await asyncio.sleep(0.1)
    mock.clear_sent()

    del session
    gc.collect()
    # Let the Drop-triggered abort take effect. A poll already in
    # flight when the session is dropped may still land, so settle
    # first, then assert that no *further* polling happens.
    await asyncio.sleep(0.3)
    mock.clear_sent()
    await asyncio.sleep(0.5)

    assert [chunk for chunk in mock.sent() if chunk == b"?"] == []


pytestmark = pytest.mark.stress
