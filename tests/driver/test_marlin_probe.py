"""Device census tests: `describe()` parity fixtures and the
`probe_device()` session method against the Marlin firmware
emulator."""

import asyncio

import pytest
from conftest import DEFAULT_DIALECT

from raygeo.driver.core import DriverEvent
from raygeo.driver.marlin import MarlinSerialDriver
from raygeo.driver.marlin.emulator import MarlinEmulator
from raygeo.driver.marlin.probe import MarlinDeviceInfo, describe

TIMINGS = {
    "handshake_timeout": 2.0,
    "handshake_poll_interval": 0.02,
    "status_poll_interval": 0.05,
    "reconnect_delay": 0.2,
    "command_timeout": 2.0,
    "connect_settle": 0.05,
}

FULL_M115_LINES = [
    ("FIRMWARE_NAME:Marlin 2.1.2.7 MACHINE_TYPE:CustomLaser EXTRUDER_COUNT:1")
]
FULL_M211_LINES = ["X: 400.00 Y: 300.00 Z: 200.00 S: 1 (Enabled)"]
FULL_M503_LINES = [
    "echo: M203 X300.00 Y300.00 Z5.00 E25.00",
    "echo: M204 S3000.00 T3000.00",
]


def make_marlin_session(mock, events, config=None):
    return MarlinSerialDriver.with_transport(
        mock,
        config={**TIMINGS, **(config or {})},
        dialect=DEFAULT_DIALECT,
        event_callback=events.callback,
    )


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


class TestDescribe:
    def test_full_census(self):
        info = describe(
            FULL_M115_LINES,
            FULL_M211_LINES,
            FULL_M503_LINES,
            [],
            ["Marlin 2.1.2.7"],
        )
        assert isinstance(info, MarlinDeviceInfo)
        assert info.name == "CustomLaser"
        assert info.firmware_version == "2.1.2.7"
        assert info.machine_type == "CustomLaser"
        assert info.max_travel == (400.0, 300.0)
        assert info.max_speed == 18000
        assert info.acceleration == 3000
        assert info.units is None

    def test_minimal_census(self):
        info = describe([], [], [], [])
        assert info.name == "Unknown Marlin Device"
        assert info.firmware_version is None
        assert info.machine_type is None
        assert info.max_travel is None
        assert info.max_speed is None
        assert info.acceleration is None
        assert info.units is None

    def test_different_feedrates_picks_lower(self):
        m503_lines = [
            "echo: M203 X500.00 Y300.00 Z5.00 E25.00",
        ]
        info = describe([], [], m503_lines, [])
        assert info.max_speed == 18000

    def test_no_boot_lines_keeps_m115_token(self):
        m115_lines = ["FIRMWARE_NAME:Marlin 2.1.2.7 MACHINE_TYPE:TestDevice"]
        info = describe(m115_lines, [], [], [], None)
        assert info.name == "TestDevice"
        assert info.firmware_version == "Marlin"

    def test_units_metric(self):
        info = describe([], [], [], ["echo: M149 Units in mm"])
        assert info.units == "metric"

    def test_units_imperial(self):
        info = describe([], [], [], ["echo: M149 Units in inches"])
        assert info.units == "imperial"

    def test_units_unknown(self):
        info = describe([], [], [], [])
        assert info.units is None


class TestProbeDevice:
    async def test_probe_device_against_emulator(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        info = await session.probe_device(["Marlin 2.1.2"])
        assert isinstance(info, MarlinDeviceInfo)
        assert info.machine_type == "3D Printer"
        assert info.name == "Marlin"
        assert info.firmware_version == "2.1.2"
        assert info.units == "metric"
        assert info.max_travel is None
        assert info.max_speed is None
        assert info.acceleration is None

    async def test_probe_device_uses_captured_boot_lines(self, marlin_rig):
        session, mock, emulator, events = marlin_rig
        info = await session.probe_device()
        assert info.name == "Marlin"
        assert info.firmware_version == "2.1.2"
