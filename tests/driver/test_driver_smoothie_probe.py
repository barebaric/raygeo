"""Device census tests: ``describe()`` parity fixtures and
``probe_device()`` against the Smoothieware firmware emulator."""

import asyncio

import pytest
from conftest import DEFAULT_DIALECT

from raygeo.driver.core import DriverEvent
from raygeo.driver.smoothie import SmoothieDriver
from raygeo.driver.smoothie.emulator import SmoothieEmulator
from raygeo.driver.smoothie.probe import SmoothieDeviceInfo, describe

TIMINGS = {
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 0.05,
    "reconnect_delay": 0.2,
    "command_timeout": 2.0,
}


def make_smoothie_session(mock, events, config=None):
    return SmoothieDriver.with_transport(
        mock,
        config={**TIMINGS, **(config or {})},
        dialect=DEFAULT_DIALECT,
        event_callback=events.callback,
    )


@pytest.fixture
async def smoothie_rig(mock, events):
    """A connected session running against the Smoothieware
    firmware emulator."""
    emulator = SmoothieEmulator(mock)
    device_task = asyncio.ensure_future(emulator.run())
    session = make_smoothie_session(mock, events)
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


class TestDescribe:
    def test_full_census(self):
        info = describe(
            ["Smoothieware edge-1234abc1"],
            ["200.000"],
            ["300.000"],
            ["500.000"],
            ["600.000"],
            ["1000.000"],
        )
        assert isinstance(info, SmoothieDeviceInfo)
        assert info.firmware_version == "Smoothieware edge-1234abc1"
        assert info.alpha_max == 200.0
        assert info.beta_max == 300.0
        assert info.max_travel == (200.0, 300.0)
        assert info.max_speed == 30000
        assert info.acceleration_steps == 1000
        assert info.acceleration == 1000.0

    def test_minimal_census(self):
        info = describe([], [], [], [], [], [])
        assert info.firmware_version is None
        assert info.alpha_max is None
        assert info.beta_max is None
        assert info.max_travel is None
        assert info.max_speed is None
        assert info.acceleration_steps is None

    def test_different_rates_picks_lower(self):
        info = describe([], [], [], ["500.000"], ["300.000"], ["1000.000"])
        assert info.max_speed == 18000

    def test_non_positive_travel_yields_no_extents(self):
        info = describe(
            [], ["0.000"], ["300.000"], ["500.000"], ["500.000"], ["1000.000"]
        )
        assert info.max_travel is None
        assert info.alpha_max == 0.0
        assert info.beta_max == 300.0

    def test_malformed_value_is_ignored(self):
        info = describe(
            [],
            ["not-a-number"],
            ["300.000"],
            ["500.000"],
            ["500.000"],
            ["1000.000"],
        )
        assert info.alpha_max is None
        assert info.max_travel is None

    def test_status_reports_and_ok_are_dropped(self):
        info = describe(
            ["<Idle|MPos:0,0,0>", "ok"],
            ["ok"],
            ["<Idle|MPos:0,0,0>", "200.000"],
            ["ok"],
            ["ok"],
            ["ok"],
        )
        assert info.firmware_version is None
        assert info.alpha_max is None
        assert info.beta_max == 200.0


class TestProbeDevice:
    async def test_probe_device_against_emulator(self, smoothie_rig):
        session, mock, emulator, events = smoothie_rig
        info = await session.probe_device()
        # The emulator emits its version banner as one line.
        assert (
            info.firmware_version
            == "Build version: edge-94de12c, Build date: 20240404"
        )
        assert info.max_travel == (200.0, 200.0)
        assert info.max_speed == 9000
        assert info.acceleration_steps == 3000
