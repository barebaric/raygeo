"""Helpers for the xTool D-series tests.

Kept in a uniquely named module (not ``conftest``) so the driver test
tree has exactly one ``conftest`` module and bare ``from conftest``
imports in the flat driver tests stay unambiguous.
"""

import asyncio
import time

from raygeo.driver.xtool_d1 import XtoolNetworkDriver
from raygeo.driver.xtool_d1.emulator import XtoolEmulator

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
