"""Tests for raygeo.driver.core.list_serial_ports.

Exercises the real enumeration binding. Hosts without serial ports
yield an empty list, so the shape assertions apply to whatever the
host actually has.
"""

from raygeo.driver.core import list_serial_ports

PORT_KEYS = {
    "port",
    "vid",
    "pid",
    "serial_number",
    "manufacturer",
    "product",
}


def test_returns_a_list_of_dicts():
    ports = list_serial_ports()
    assert isinstance(ports, list)
    for entry in ports:
        assert isinstance(entry, dict)
        assert set(entry) == PORT_KEYS
        assert isinstance(entry["port"], str)
        assert entry["port"]
        for key in ("vid", "pid"):
            assert entry[key] is None or isinstance(entry[key], int)
        for key in ("serial_number", "manufacturer", "product"):
            assert entry[key] is None or isinstance(entry[key], str)


def test_usb_fields_are_consistent():
    for entry in list_serial_ports():
        if entry["vid"] is None:
            assert entry["pid"] is None
        else:
            assert isinstance(entry["pid"], int)
