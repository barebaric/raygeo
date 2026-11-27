"""Tests for marshalling driver declarations into subnet probe specs.

The scan, probe, claim and result-type logic lives in raydriver and
is tested there; these tests cover the declaration to spec conversion
and the engine's timeout/failure handling.
"""

import asyncio

import pytest

from raygeo.driver.core.discovery import DiscoveredDevice, engine
from raygeo.driver.core.discovery.engine import find_subnet_devices
from raygeo.driver.core.discovery.spec import (
    DiscoverySpec,
    TcpRecognizer,
)


class FakeTcpDriver:
    DISCOVERY = DiscoverySpec(
        tcp=TcpRecognizer(
            port=2323, probe_command="$I\n", match_prefix="[VER:"
        )
    )
    label = "Fake TCP device"


@pytest.fixture
def fake_scan(monkeypatch):
    calls = []

    async def fake(specs, subnets=None):
        calls.append(specs)
        return [
            DiscoveredDevice(
                driver_name="FakeTcpDriver",
                params={"host": "10.0.0.5", "port": 2323},
                label="Fake TCP device",
                detail="10.0.0.5:2323",
            )
        ]

    monkeypatch.setattr(engine, "_rust_find_subnet_devices", fake)
    return calls


@pytest.mark.asyncio
async def test_declarations_become_probe_specs(fake_scan):
    devices = await find_subnet_devices([FakeTcpDriver])
    assert fake_scan == [
        [
            {
                "id": "FakeTcpDriver",
                "port": 2323,
                "probe_command": "$I\n",
                "match_prefix": "[VER:",
                "driver_name": "FakeTcpDriver",
                "label": "Fake TCP device",
                "firmware": "faketcp",
            }
        ]
    ]
    assert len(devices) == 1
    assert devices[0].key == "FakeTcpDriver:10.0.0.5:2323"


@pytest.mark.asyncio
async def test_drivers_without_tcp_spec_are_skipped(fake_scan):
    class SerialOnly:
        DISCOVERY = DiscoverySpec()

    assert await find_subnet_devices([SerialOnly]) == []
    assert fake_scan == []


@pytest.mark.asyncio
async def test_scan_failure_returns_empty(monkeypatch):
    async def broken_scan(specs, subnets=None):
        raise OSError("boom")

    monkeypatch.setattr(engine, "_rust_find_subnet_devices", broken_scan)
    assert await find_subnet_devices([FakeTcpDriver]) == []


@pytest.mark.asyncio
async def test_scan_timeout_returns_empty(monkeypatch):
    async def slow_scan(specs, subnets=None):
        await asyncio.sleep(5)

    monkeypatch.setattr(engine, "_rust_find_subnet_devices", slow_scan)
    monkeypatch.setattr(engine, "SUBNET_SCAN_TIMEOUT", 0.05)
    assert await find_subnet_devices([FakeTcpDriver]) == []
