"""Tests for mDNS network service scanning.

The browse runs in Rust (raygeo.driver.core.discovery), so the service
side is simulated with a real zeroconf announcement on the local
machine.
"""

import asyncio

import pytest
import pytest_asyncio

from raygeo.driver.core.discovery.mdns_scan import (
    MDNSService,
    normalize_service_type,
    scan_mdns_services,
)

_OCTOPRINT_TYPE = "_octoprint._tcp.local."
_OCTOPRINT_NAME = f"OctoPrint on octopi.{_OCTOPRINT_TYPE}"


@pytest_asyncio.fixture
async def announced_service():
    """Registers a known service via zeroconf for the duration of a
    test, so the Rust browse has something real to resolve."""
    zeroconf = pytest.importorskip("zeroconf")
    from zeroconf.asyncio import AsyncZeroconf

    info = zeroconf.ServiceInfo(
        _OCTOPRINT_TYPE,
        _OCTOPRINT_NAME,
        port=80,
        addresses=[b"\xc0\xa8\x01\x2a"],  # 192.168.1.42
        properties={b"path": b"/", b"version": b"1.9.3"},
        server="octopi.local.",
    )
    aiozc = AsyncZeroconf()
    await aiozc.async_register_service(info)
    try:
        yield info
    finally:
        await aiozc.async_unregister_service(info)
        await aiozc.async_close()


def test_normalize_service_type():
    assert (
        normalize_service_type("_octoprint._tcp.local.") == "_octoprint._tcp"
    )
    assert normalize_service_type("_octoprint._tcp") == "_octoprint._tcp"
    assert normalize_service_type("_ruida._udp.local") == "_ruida._udp"
    assert normalize_service_type("local.") == ""
    assert normalize_service_type("") == ""


@pytest.mark.asyncio
async def test_scan_returns_resolved_services(announced_service):
    services = await scan_mdns_services(["_octoprint._tcp.local."])
    match = [s for s in services if s.name == "OctoPrint on octopi"]
    assert len(match) == 1
    service = match[0]
    assert isinstance(service, MDNSService)
    assert service.host == "192.168.1.42"
    assert service.port == 80
    assert service.server == "octopi.local"
    assert service.service_type == "_octoprint._tcp"
    assert service.txt == {"path": "/", "version": "1.9.3"}


@pytest.mark.asyncio
async def test_scan_without_service_types_is_noop():
    assert await scan_mdns_services([]) == []
    assert await scan_mdns_services(["local."]) == []


@pytest.mark.asyncio
async def test_scan_cancellable_from_caller(announced_service):
    """A caller-side timeout cancels the await without blocking the
    caller's event loop (the browse itself runs off the loop)."""
    with pytest.raises(asyncio.TimeoutError):
        await asyncio.wait_for(
            scan_mdns_services(["_octoprint._tcp.local."], browse_window=5.0),
            timeout=0.05,
        )


@pytest.mark.asyncio
async def test_scan_does_not_block_caller_loop(announced_service):
    """The browse runs off the caller's event loop, so other asyncio
    work keeps making progress while the scan is in flight. This is
    the regression guard: the scanner must not attach to the app
    loop."""
    ticks = 0

    async def _ticker():
        nonlocal ticks
        for _ in range(5):
            await asyncio.sleep(0.02)
            ticks += 1

    await asyncio.gather(
        scan_mdns_services(["_octoprint._tcp.local."], browse_window=0.2),
        _ticker(),
    )
    assert ticks == 5


async def test_scan_accepts_set_of_service_types(announced_service):
    """The network channel hands the declared types over as a set;
    the scanner must accept any iterable, not just sequences."""
    services = await scan_mdns_services({"_octoprint._tcp"}, 2.0)
    assert any(s.name == "OctoPrint on octopi" for s in services)
