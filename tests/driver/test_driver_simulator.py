"""The NoDeviceDriver simulator: job/progress simulation, connection
events, coordinate mirrors and the fixed probe point."""

import asyncio

from conftest import EventRecorder

from raygeo.driver import Driver, driver_info
from raygeo.driver.core import DriverEvent
from raygeo.driver.core.types import DeviceStatus
from raygeo.driver.simulator import NoDeviceDriver


def make_driver(events, **config):
    return NoDeviceDriver(
        config={"step_delay": 0.0, **config},
        event_callback=events.callback,
    )


async def connected_driver(events, **config):
    """A connected simulator with the setup events discarded."""
    session = make_driver(events, **config)
    await session.connect()
    events.events.clear()
    return session


def test_identity_and_capabilities():
    info = driver_info("NoDeviceDriver")
    assert info.name == "NoDeviceDriver"
    assert info.connection == "none"
    assert info.capabilities["reports_granular_progress"] is True
    assert info.capabilities["supports_settings"] is False
    assert issubclass(NoDeviceDriver, Driver)


async def test_connect_reports_idle_and_offsets():
    events = EventRecorder()
    session = make_driver(events)
    await session.connect()
    await events.wait_for(
        DriverEvent.ConnectionStatusChanged,
        lambda event: event.status == "CONNECTED",
    )
    assert session.state.status == DeviceStatus.IDLE
    offsets = await session.read_wcs_offsets()
    assert set(offsets) == {"G54", "G55", "G56", "G57", "G58", "G59"}


async def test_run_reports_each_op_once():
    events = EventRecorder()
    session = await connected_driver(events)
    done = []
    await session.run(
        "G1 X0\nG1 X1\nG1 X2\nG1 X3",
        {0: 0, 1: 0, 2: 1, 3: 1},
        progress_callback=done.append,
    )
    await events.wait_for(DriverEvent.JobFinished)
    assert done == [0, 1]
    assert session.state.status == DeviceStatus.IDLE
    assert session.job_running is False


async def test_run_raw_streams_and_finishes():
    events = EventRecorder()
    session = await connected_driver(events)
    await session.run_raw("G1 X1\nG1 X2\n")
    await events.wait_for(DriverEvent.JobFinished)


async def test_wcs_offsets_round_trip():
    events = EventRecorder()
    session = await connected_driver(events)
    await session.set_wcs_offset("G54", 1.0, 2.0, 3.0)
    offsets = await session.read_wcs_offsets()
    assert offsets["G54"] == (1.0, 2.0, 3.0)
    await events.wait_for(
        DriverEvent.WcsUpdated,
        lambda event: event.offsets["G54"] == (1.0, 2.0, 3.0),
    )


async def test_probe_cycle_returns_the_fixed_point():
    events = EventRecorder()
    session = await connected_driver(events)
    result = await session.run_probe_cycle("Z", 10.0, 100.0)
    assert result == (10.0, 15.0, -1.0)
    await events.wait_for(DriverEvent.ProbeStatusChanged)


async def test_moves_and_settings_are_accepted():
    events = EventRecorder()
    session = await connected_driver(events)
    await session.home()
    await session.move_to(1000.0, 1.0, 2.0)
    await session.jog(1000.0, [("x", 1.0)])
    await session.select_tool(1)
    await session.set_power(50.0)
    await session.set_focus_power(10.0)
    await session.write_setting("x", "1")
    assert await session.read_settings() == []
    assert await session.read_parser_state() == "G54"


async def test_step_delay_slows_the_job():
    events = EventRecorder()
    session = await connected_driver(events, step_delay=0.02)
    started = asyncio.get_running_loop().time()
    await session.run("G1 X0\nG1 X1", {0: 0, 1: 1})
    elapsed = asyncio.get_running_loop().time() - started
    assert elapsed >= 0.03
