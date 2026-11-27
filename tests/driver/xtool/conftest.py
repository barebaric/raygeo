"""Shared fixtures: xTool sessions driven against the xTool WiFi
firmware emulator (raygeo.driver.xtool.emulator)."""

import asyncio
import time

import pytest

from raygeo.driver.core import DriverEvent
from raygeo.driver.xtool import XtoolNetworkDriver
from raygeo.driver.xtool.emulator import XtoolEmulator

FAST_CONFIG = {
    "host": "127.0.0.1",
    "request_timeout": 2.0,
    "connect_timeout": 2.0,
    "reconnect_delay": 0.2,
    "homing_timeout": 5.0,
    "job_start_timeout": 5.0,
    "poll_interval": 0.05,
}

DIALECT = {
    "laser_on": "M4 S{power:.0f}",
    "laser_off": "M5",
    "focus_laser_on": "G1 F1 M3 S{power:.0f}",
    "safety_off_commands": ["M5", "M9"],
}


class EventRecorder:
    def __init__(self):
        self.events = []

    def callback(self, event):
        self.events.append(event)

    def names(self):
        return [type(event).__name__ for event in self.events]

    async def wait_for(self, kind, pred=None, timeout=5.0):
        """Wait until an event of *kind* (a DriverEvent variant
        class) matching *pred* was recorded."""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for event in self.events:
                if not isinstance(event, kind):
                    continue
                if pred is None or pred(event):
                    return event
            await asyncio.sleep(0.01)
        raise AssertionError(
            f"event {kind!r} not observed within {timeout:.1f}s; "
            f"got {self.names()}"
        )


async def wait_until(predicate, timeout=5.0, interval=0.01):
    """Poll *predicate* until it returns a truthy value."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        await asyncio.sleep(interval)
    raise AssertionError(f"condition not met within {timeout:.1f}s")


def make_session(events, config=None, dialect=None):
    return XtoolNetworkDriver(
        config={**FAST_CONFIG, **(config or {})},
        dialect={**DIALECT, **(dialect or {})},
        event_callback=events.callback,
    )


async def start_emulator(**kwargs):
    emulator = XtoolEmulator(**kwargs)
    http_port, ws_port = await emulator.start()
    return emulator


@pytest.fixture
def events():
    return EventRecorder()


@pytest.fixture
async def emulator():
    emu = XtoolEmulator()
    http_port, ws_port = await emu.start()
    try:
        yield emu
    finally:
        await emu.stop()


@pytest.fixture
async def rig(emulator, events):
    """A connected session running against the firmware emulator.

    Yields ``(session, emulator, events)`` with the connection
    established and the recorded events cleared.
    """
    session = make_session(
        events,
        config={
            "http_port": emulator.http_port,
            "ws_port": emulator.ws_port,
        },
    )
    await session.connect()
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
    )
    events.events.clear()
    try:
        yield session, emulator, events
    finally:
        await session.disconnect()
