"""Serial helper tests: baud rates, vid:pid resolution and the
permission diagnostic (against fake port listings — no hardware)."""

import sys

import pytest

from raygeo.driver.core.discovery import ports as ports_module
from raygeo.driver.core.discovery.ports import (
    SerialPortInfo,
    check_permission_issue,
    list_baud_rates,
    resolve_vidpid,
)


@pytest.fixture
def fake_ports(monkeypatch):
    """Replaces the enumeration backend seam the Rust helpers read:
    `list_port_info` returns canned SerialPortInfo values, and
    `list_ports` — the raw platform backend — the matching paths."""

    def install(*infos):
        monkeypatch.setattr(
            ports_module, "list_port_info", lambda: list(infos)
        )
        monkeypatch.setattr(
            ports_module, "list_ports", lambda: [i.device for i in infos]
        )

    return install


class TestListBaudRates:
    def test_contains_common_rates(self):
        rates = list_baud_rates()
        assert 9600 in rates
        assert 115200 in rates
        assert 250000 not in rates

    def test_returns_a_copy(self):
        rates = list_baud_rates()
        rates.append(123456)
        assert 123456 not in list_baud_rates()


class TestResolveVidpid:
    def test_resolves_matching_device(self, fake_ports):
        fake_ports(
            SerialPortInfo(device="/dev/ttyS0"),
            SerialPortInfo(device="/dev/ttyUSB0", vid=0x0403, pid=0x6001),
        )
        assert resolve_vidpid(0x0403, 0x6001) == "/dev/ttyUSB0"

    def test_first_match_wins(self, fake_ports):
        fake_ports(
            SerialPortInfo(device="/dev/ttyUSB0", vid=0x0403, pid=0x6001),
            SerialPortInfo(device="/dev/ttyUSB1", vid=0x0403, pid=0x6001),
        )
        assert resolve_vidpid(0x0403, 0x6001) == "/dev/ttyUSB0"

    def test_no_match_returns_none(self, fake_ports):
        fake_ports(
            SerialPortInfo(device="/dev/ttyUSB0", vid=0x1234, pid=0x5678)
        )
        assert resolve_vidpid(0x0403, 0x6001) is None

    def test_empty_bus_returns_none(self, fake_ports):
        fake_ports()
        assert resolve_vidpid(0x0403, 0x6001) is None


@pytest.mark.skipif(sys.platform == "win32", reason="POSIX-only check")
class TestCheckPermissionIssue:
    def test_no_ports(self, fake_ports, monkeypatch):
        monkeypatch.delenv("SNAP", raising=False)
        fake_ports()
        assert check_permission_issue() == "no_ports"

    def test_accessible_port(self, fake_ports, monkeypatch, tmp_path):
        monkeypatch.delenv("SNAP", raising=False)
        # The enumeration backend filters by USB markers, so the
        # fake port needs a USB-looking path (created for real: the
        # diagnostic is an access(2) check).
        port = tmp_path / "ttyUSB0"
        port.write_bytes(b"")
        fake_ports(SerialPortInfo(device=str(port)))
        assert check_permission_issue() == "none"

    def test_no_access(self, fake_ports, monkeypatch):
        monkeypatch.delenv("SNAP", raising=False)
        # A device path nobody can open: guaranteed absent.
        fake_ports(SerialPortInfo(device="/nonexistent/ttyUSB0"))
        assert check_permission_issue() == "no_access"

    def test_snap_no_ports(self, fake_ports, monkeypatch):
        monkeypatch.setenv("SNAP", "/snap/rayforge")
        fake_ports()
        assert check_permission_issue() == "snap_no_ports"

    def test_snap_no_access(self, fake_ports, monkeypatch):
        monkeypatch.setenv("SNAP", "/snap/rayforge")
        fake_ports(SerialPortInfo(device="/nonexistent/ttyUSB0"))
        assert check_permission_issue() == "snap_no_access"
