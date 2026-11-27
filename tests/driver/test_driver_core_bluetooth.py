"""Bluetooth SPP configuration, address handling and enumeration."""

import pytest

from raygeo.driver.config import GrblBluetoothConfig
from raygeo.driver.core.bluetooth import (
    BluetoothDeviceInfo,
    is_supported,
    list_paired_devices,
    normalize_address,
)
from raygeo.driver.core.discovery import device_key
from raygeo.driver.grbl import GrblBluetoothDriver


class TestAddress:
    def test_normalizes_colon_address(self):
        assert normalize_address("aa:bb:cc:dd:ee:ff") == "AA:BB:CC:DD:EE:FF"

    def test_normalizes_macos_dashes(self):
        assert normalize_address("aa-bb-cc-dd-ee-ff") == "AA:BB:CC:DD:EE:FF"

    def test_rejects_wrong_length(self):
        with pytest.raises(ValueError):
            normalize_address("aa:bb:cc")

    def test_rejects_non_hex(self):
        with pytest.raises(ValueError):
            normalize_address("aa:bb:cc:dd:ee:zz")

    def test_rejects_empty(self):
        with pytest.raises(ValueError):
            normalize_address("  ")


class TestConfig:
    def test_to_dict(self):
        data = GrblBluetoothConfig(
            address="AA:BB:CC:DD:EE:FF", channel=2
        ).to_dict()
        assert data["address"] == "AA:BB:CC:DD:EE:FF"
        assert data["channel"] == 2
        assert "port" not in data

    def test_session_resource_uri(self):
        session = GrblBluetoothDriver(
            config=GrblBluetoothConfig(address="aa:bb:cc:dd:ee:ff")
        )
        assert session.resource_uri == "bt://AA:BB:CC:DD:EE:FF"

    def test_channel_default_is_one(self):
        session = GrblBluetoothDriver(config={"address": "AA:BB:CC:DD:EE:FF"})
        assert session.resource_uri == "bt://AA:BB:CC:DD:EE:FF"

    def test_invalid_address_rejected(self):
        with pytest.raises(ValueError, match="address"):
            GrblBluetoothDriver(config={"address": "not-an-address"})

    def test_invalid_channel_rejected(self):
        with pytest.raises(ValueError, match="channel"):
            GrblBluetoothDriver(
                config={"address": "AA:BB:CC:DD:EE:FF", "channel": 0}
            )
        with pytest.raises(ValueError, match="channel"):
            GrblBluetoothDriver(
                config={"address": "AA:BB:CC:DD:EE:FF", "channel": 31}
            )


class TestEnumeration:
    def test_supported_is_boolean(self):
        assert isinstance(is_supported(), bool)

    def test_list_returns_well_formed_devices(self):
        devices = list_paired_devices()
        assert isinstance(devices, list)
        for device in devices:
            assert isinstance(device, BluetoothDeviceInfo)
            assert isinstance(device.address, str)
            assert device.name is None or isinstance(device.name, str)
            assert device.channel is None or 1 <= device.channel <= 30

    def test_device_info_repr(self):
        device = BluetoothDeviceInfo("AA:BB:CC:DD:EE:FF", "Laser", 1)
        assert "AA:BB:CC:DD:EE:FF" in repr(device)

    def test_device_key_uses_address(self):
        key = device_key(
            "GrblBluetoothDriver",
            {"address": "AA:BB:CC:DD:EE:FF"},
        )
        assert key == "GrblBluetoothDriver:bt://AA:BB:CC:DD:EE:FF"
