"""Tests for serial port enumeration (Rust-backed).

The enumeration backend (``list_serial_ports``) and the sandbox-safe
globber (``glob_ports``) are module attributes, so tests replace them
with canned data instead of touching real /dev nodes.
"""

import os

import pytest

from raygeo.driver.core.discovery import ports
from raygeo.driver.core.discovery.ports import (
    list_port_info,
    list_usb_ports,
    safe_list_ports_linux,
)

BY_ID_LINK = "/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_AH03K1A0-if00-port0"


def _fake_glob(mapping):
    def glob_ports(dir, prefix):
        return list(mapping.get((dir, prefix), []))

    return glob_ports


def test_safe_list_ports_linux(monkeypatch):
    """Test the sandbox-safe globbing logic."""
    mapping = {
        ("/dev", "ttyUSB"): ["/dev/ttyUSB1", "/dev/ttyUSB0"],
        ("/dev", "ttyACM"): ["/dev/ttyACM0"],
        ("/dev", "rfcomm"): ["/dev/rfcomm0"],
        ("/dev/serial/by-id", ""): [BY_ID_LINK],
        ("/dev/serial/by-path", ""): [
            "/dev/serial/by-path/pci-0000:00:14.0-usb-0:1:1.0-port0"
        ],
    }
    monkeypatch.setattr(ports, "glob_ports", _fake_glob(mapping))
    ports_list = safe_list_ports_linux()
    assert ports_list == [
        "/dev/rfcomm0",
        BY_ID_LINK,
        "/dev/serial/by-path/pci-0000:00:14.0-usb-0:1:1.0-port0",
        "/dev/ttyACM0",
        "/dev/ttyUSB0",
        "/dev/ttyUSB1",
    ]


def test_safe_list_ports_linux_symlinks_only(monkeypatch):
    """Test that symlinks are included when no standard ports exist."""
    mapping = {("/dev/serial/by-id", ""): [BY_ID_LINK]}
    monkeypatch.setattr(ports, "glob_ports", _fake_glob(mapping))
    assert safe_list_ports_linux() == [BY_ID_LINK]


@pytest.mark.skipif(os.name != "posix", reason="POSIX USB filtering")
def test_list_usb_ports_filtering(monkeypatch):
    """Test the USB port filtering logic."""
    mock_ports = [
        "/dev/ttyS0",
        "/dev/ttyUSB0",
        "/dev/ttyACM1",
        "/dev/rfcomm0",
        "COM3",
        "/dev/cu.usbmodem123",
    ]
    monkeypatch.setattr(ports, "list_ports", lambda: mock_ports)
    assert list_usb_ports() == [
        "/dev/ttyUSB0",
        "/dev/ttyACM1",
        "/dev/rfcomm0",
    ]


@pytest.mark.skipif(os.name == "posix", reason="non-POSIX returns all")
def test_list_usb_ports_returns_all_on_windows(monkeypatch):
    mock_ports = ["COM3", "COM4"]
    monkeypatch.setattr(ports, "list_ports", lambda: mock_ports)
    assert list_usb_ports() == mock_ports


@pytest.mark.skipif(os.name != "posix", reason="POSIX sort order")
def test_list_port_info_with_descriptions(monkeypatch):
    """
    Outside a Snap, the platform descriptions are used. Placeholder
    descriptions ('n/a') are discarded.
    """
    monkeypatch.delenv("SNAP", raising=False)
    devices = [
        {"port": "/dev/ttyS0", "product": "ttyS0"},
        {"port": "/dev/ttyUSB10", "product": "CH340"},
        {"port": "/dev/ttyACM0", "product": "n/a"},
        {"port": "/dev/ttyUSB2", "product": None},
    ]
    monkeypatch.setattr(ports, "list_serial_ports", lambda: devices)
    infos = list_port_info()
    assert [i.device for i in infos] == [
        "/dev/ttyACM0",
        "/dev/ttyUSB2",
        "/dev/ttyUSB10",
        "/dev/ttyS0",
    ]
    by_device = {i.device: i for i in infos}
    assert by_device["/dev/ttyUSB10"].description == "CH340"
    assert by_device["/dev/ttyACM0"].description is None
    assert by_device["/dev/ttyUSB2"].description is None
    assert by_device["/dev/ttyS0"].description == "ttyS0"


@pytest.mark.skipif(os.name != "posix", reason="Snap is POSIX-only")
def test_list_port_info_snap_by_id_descriptions(monkeypatch, tmp_path):
    """
    In a Snap, descriptions are derived from /dev/serial/by-id symlink
    names and also applied to the resolved device paths.
    """
    monkeypatch.setenv("SNAP", "/snap/rayforge/123")
    by_id_dir = tmp_path / "serial" / "by-id"
    by_id_dir.mkdir(parents=True)
    target = tmp_path / "ttyUSB0"
    target.touch()
    link = by_id_dir / "usb-FTDI_FT232R_USB_UART_AH03K1A0-if00-port0"
    link.symlink_to(target)
    mapping = {
        ("/dev/serial/by-id", ""): [str(link)],
        ("/dev", "ttyUSB"): [str(target)],
    }
    monkeypatch.setattr(ports, "glob_ports", _fake_glob(mapping))
    infos = list_port_info()
    descriptions = {i.device: i.description for i in infos}
    desc = "FTDI FT232R USB UART AH03K1A0"
    assert descriptions[str(target)] == desc
    assert descriptions[str(link)] == desc


def test_list_port_info_pyserial_error(monkeypatch):
    """If enumeration fails, an empty list is returned instead of raising."""
    monkeypatch.delenv("SNAP", raising=False)

    def raise_error():
        raise OSError("boom")

    monkeypatch.setattr(ports, "list_serial_ports", raise_error)
    assert list_port_info() == []
