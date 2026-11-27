"""Tests for the discovery result types and the subnet wrapper."""

import asyncio

import pytest

from raygeo.driver.core.discovery import (
    DeviceIdentity,
    DiscoveredDevice,
    device_key,
    find_subnet_devices,
)


def test_discovered_device_key():
    device = DiscoveredDevice(
        driver_name="GrblSerialDriver",
        params={"port": "/dev/ttyUSB0", "baudrate": 115200},
        label="GRBL device",
        detail="/dev/ttyUSB0 at 115200 baud",
    )
    assert device.key == "GrblSerialDriver:/dev/ttyUSB0"
    assert DeviceIdentity() == DeviceIdentity()


def test_discovered_device_key_network_unique_per_host():
    def _device(host):
        return DiscoveredDevice(
            driver_name="OctoPrintDriver",
            params={"host": host, "port": 80},
            label="OctoPrint",
            detail=f"{host}:80",
        )

    # Two servers behind the same TCP port stay distinguishable.
    assert _device("192.168.1.42").key != _device("192.168.1.43").key
    assert _device("192.168.1.42").key == "OctoPrintDriver:192.168.1.42:80"


@pytest.mark.parametrize(
    "params,expected",
    [
        ({"host": "h", "port": 80}, "D:h:80"),
        ({}, "D:None"),
        ({"port": 1234}, "D:1234"),
    ],
)
def test_discovered_device_key_fallbacks(params, expected):
    device = DiscoveredDevice(
        driver_name="D", params=params, label="", detail=""
    )
    assert device.key == expected


def test_device_key_remote():
    params = {
        "remote_host": "pi.local",
        "remote_port": 8765,
        "port": "/dev/ttyUSB0",
    }
    assert device_key("RaydriverRemoteDriver", params) == (
        "RaydriverRemoteDriver:raydriver://pi.local:8765:/dev/ttyUSB0"
    )


def test_probe_name_skips_unknown():
    class Meta:
        name = "Unknown machine"

    class Profile:
        meta = Meta()

    device = DiscoveredDevice(
        driver_name="D",
        params={},
        label="",
        detail="",
        probe_profile=Profile(),
    )
    assert device.probe_name is None
    Meta.name = " My Laser "
    assert device.probe_name == "My Laser"


class TestSubnetWrapper:
    async def test_returns_discovered_devices(self):
        async def handle_client(reader, writer):
            await reader.read(256)
            writer.write(b"[VER:1.1f.20230812:]\r\n")
            await writer.drain()
            await asyncio.sleep(0.5)
            writer.close()

        server = await asyncio.start_server(handle_client, "127.0.0.1", 0)
        port = server.sockets[0].getsockname()[1]
        try:
            devices = await find_subnet_devices(
                [
                    {
                        "id": "grbl",
                        "port": port,
                        "probe_command": "$I\n",
                        "match_prefix": "[VER:",
                        "driver_name": "GrblTelnetDriver",
                        "label": "grblHAL (Telnet)",
                        "firmware": "grbltelnet",
                    }
                ],
                [("127.0.0.1", 32)],
            )
            assert len(devices) == 1
            device = devices[0]
            assert isinstance(device, DiscoveredDevice)
            assert device.driver_name == "GrblTelnetDriver"
            assert device.params == {"host": "127.0.0.1", "port": port}
            assert device.label == "grblHAL (Telnet)"
            assert device.detail == f"127.0.0.1:{port}"
            assert device.identity.firmware == "grbltelnet"
            assert device.identity.banner == "[VER:1.1f.20230812:]"
            assert device.key == f"GrblTelnetDriver:127.0.0.1:{port}"
        finally:
            server.close()
            await server.wait_closed()

    async def test_no_specs_yields_nothing(self):
        assert await find_subnet_devices([], [("127.0.0.1", 32)]) == []
