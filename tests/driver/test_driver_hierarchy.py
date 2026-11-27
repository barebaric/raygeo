"""The driver hierarchy contract.

Every built-in driver is a real Python subclass of the common
`Driver` base, its typed metadata is reachable without an instance,
and every capability a driver advertises is implemented: unsupported
operations raise `NotImplementedError`, supported ones never do.
"""

import asyncio

import pytest
from conftest import DEFAULT_DIALECT

from raygeo.driver import Driver, driver_info, drivers, get_driver_cls
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.epilog import EpilogDriver
from raygeo.driver.octoprint import OctoPrintDriver
from raygeo.driver.ruida import RuidaDriver
from raygeo.driver.simulator import NoDeviceDriver
from raygeo.driver.xtool_d1 import XtoolNetworkDriver
from raygeo.driver.xtool_m2 import XtoolM2Driver

#: Session classes with a direct (non-Mock) constructor.
NETWORK_DRIVERS = {
    OctoPrintDriver,
    XtoolNetworkDriver,
    XtoolM2Driver,
    EpilogDriver,
    RuidaDriver,
}


def make_unconnected(cls):
    """Construct the driver without opening a connection."""
    if cls is NoDeviceDriver:
        return cls(config={"step_delay": 0.0})
    if cls in NETWORK_DRIVERS:
        return cls(config={"host": "127.0.0.1", "port": 1})
    return cls.with_transport(MockTransport(), dialect=DEFAULT_DIALECT)


def test_every_driver_shares_the_base():
    assert drivers
    assert len({cls.__name__ for cls in drivers}) == len(drivers)
    for cls in drivers:
        assert issubclass(cls, Driver)
        assert cls is not Driver
        assert Driver in cls.__mro__


def test_driver_lookup_round_trips():
    for cls in drivers:
        assert get_driver_cls(cls.__name__) is cls
        info = driver_info(cls.__name__)
        assert info.name == cls.__name__
        assert info.connection
        assert info.maturity
        assert isinstance(info.capabilities, dict)
        assert info.capabilities["uses_gcode"] is info.uses_gcode


def test_driver_info_carries_the_setup_schema():
    info = driver_info("GrblSerialDriver")
    keys = [field.key for field in info.setup]
    assert keys[:2] == ["port", "baudrate"]
    port, baudrate = info.setup[0], info.setup[1]
    assert port.kind == "serial_port"
    assert port.required is True
    assert baudrate.default == "115200"
    assert port.to_dict()["key"] == "port"
    for cls in drivers:
        assert driver_info(cls.__name__).setup


def test_instance_info_matches_class_info():
    for cls in drivers:
        session = make_unconnected(cls)
        info = session.info
        assert info.name == cls.__name__
        assert info.capabilities == driver_info(cls.__name__).capabilities


async def test_builtin_drivers_have_no_alternative_base():
    """The base cannot be instantiated and declares the uniform surface."""
    with pytest.raises(TypeError):
        Driver()
    for name in (
        "connect",
        "disconnect",
        "run",
        "run_raw",
        "cancel",
        "set_hold",
        "home",
        "move_to",
        "jog",
        "select_tool",
        "set_power",
        "set_focus_power",
        "clear_alarm",
        "set_wcs_offset",
        "read_settings",
        "write_setting",
        "read_wcs_offsets",
        "read_parser_state",
        "run_probe_cycle",
        "detect_unit_system",
        "execute_command",
        "execute_interactive_command",
        "update_dialect",
    ):
        assert callable(getattr(Driver, name))
        for cls in drivers:
            assert callable(getattr(cls, name))


CAPABILITY_CALLS = {
    "supports_settings": lambda session: session.read_settings(),
    "supports_hold": lambda session: session.set_hold(True),
    "supports_clear_alarm": lambda session: session.clear_alarm(),
    "supports_unit_detection": lambda session: session.detect_unit_system(),
}


@pytest.mark.parametrize("cls", drivers, ids=[c.__name__ for c in drivers])
async def test_advertised_capabilities_are_implemented(cls):
    """A capability claim never ends in NotImplementedError."""
    session = make_unconnected(cls)
    caps = driver_info(cls.__name__).capabilities
    for capability, call in CAPABILITY_CALLS.items():
        if not caps[capability]:
            continue
        try:
            await asyncio.wait_for(call(session), timeout=0.05)
        except NotImplementedError:
            pytest.fail(
                f"{cls.__name__} claims {capability} but raises "
                "NotImplementedError"
            )
        except (ConnectionError, asyncio.TimeoutError):
            pass
