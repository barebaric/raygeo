"""Tests for the transport-level discovery scanners.

Exercises the Rust scanners through their PyO3 bindings: subnet
host enumeration and TCP scanning, serial capture against a pty,
and an mDNS browse against a locally announced service.
"""

import asyncio
import ipaddress
import os
import sys
import threading
import time

import pytest

from raygeo.driver.core import discovery


class TestSubnetHosts:
    def test_class_c_excludes_network_and_broadcast(self):
        hosts = discovery.subnet_hosts("192.168.1.5", 24)
        assert len(hosts) == 254
        assert hosts[0] == "192.168.1.1"
        assert hosts[-1] == "192.168.1.254"
        assert "192.168.1.0" not in hosts
        assert "192.168.1.255" not in hosts

    def test_wide_network_narrowed_to_class_c(self):
        hosts = discovery.subnet_hosts("10.1.2.3", 16)
        assert len(hosts) == 254
        assert hosts[0] == "10.1.2.1"
        assert hosts[-1] == "10.1.2.254"

    def test_host_route_yields_single_address(self):
        assert discovery.subnet_hosts("10.0.0.7", 32) == ["10.0.0.7"]

    def test_point_to_point_yields_both_addresses(self):
        assert discovery.subnet_hosts("10.0.0.0", 31) == [
            "10.0.0.0",
            "10.0.0.1",
        ]

    def test_invalid_address_raises(self):
        with pytest.raises(ValueError):
            discovery.subnet_hosts("not-an-ip", 24)


class TestLocalSubnets:
    def test_returns_parseable_non_loopback_interfaces(self):
        subnets = discovery.local_subnets()
        assert isinstance(subnets, list)
        for address, prefixlen in subnets:
            addr = ipaddress.IPv4Address(address)
            assert not addr.is_loopback
            assert 0 <= prefixlen <= 32


class TestScanSubnet:
    async def test_finds_listening_host_only(self):
        server = await asyncio.start_server(
            lambda r, w: w.close(), "127.0.0.1", 0
        )
        port = server.sockets[0].getsockname()[1]
        try:
            # 127.0.0.2 is loopback too, but nothing listens there.
            reachable = await discovery.scan_subnet(
                ["127.0.0.1", "127.0.0.2"], port, 0.5, 8
            )
            assert reachable == ["127.0.0.1"]
        finally:
            server.close()
            await server.wait_closed()

    async def test_empty_hosts(self):
        assert await discovery.scan_subnet([], 23, 0.1, 4) == []


class TestScanSerialPorts:
    async def test_absent_port_yields_nothing(self):
        observations = await discovery.scan_serial_ports(
            ["/dev/raydriver-does-not-exist"], [115200], 5.0
        )
        assert observations == []

    @pytest.mark.skipif(
        not hasattr(os, "openpty"), reason="requires a POSIX pty"
    )
    @pytest.mark.skipif(
        sys.platform == "darwin",
        reason="macOS ptys do not deliver master writes to a "
        "serialport reader on CI runners",
    )
    async def test_captures_device_banner(self):
        import tty

        master, slave = os.openpty()
        # Raw mode: no echo of the scanner's nudges and no line
        # buffering, so captured bytes match what the device sent.
        tty.setraw(slave)
        stop = threading.Event()

        def chatter():
            # macOS drops master writes made before the slave is
            # opened, so keep sending the banner until the scanner
            # has it rather than writing once up front.
            while not stop.is_set():
                try:
                    os.write(master, b"Grbl 1.1h ['$' for help]\n")
                except OSError:
                    return
                time.sleep(0.1)

        thread = threading.Thread(target=chatter, daemon=True)
        thread.start()
        try:
            observations = await discovery.scan_serial_ports(
                [os.ttyname(slave)], [115200], 10.0
            )
            assert len(observations) == 1
            observation = observations[0]
            assert observation["baud_rate"] == 115200
            data = observation["data"]
            assert isinstance(data, bytes)
            assert b"Grbl 1.1h" in data
        finally:
            stop.set()
            thread.join(timeout=1.0)
            os.close(master)
            os.close(slave)


class TestCancelledScan:
    async def test_cancelling_scan_does_not_error_on_completion(self):
        """Cancelling the awaiting task (e.g. app shutdown) must not
        make the Rust side's later completion raise InvalidStateError
        in the event loop."""
        errors = []
        loop = asyncio.get_running_loop()
        loop.set_exception_handler(lambda _loop, ctx: errors.append(ctx))
        # The binding returns a bare asyncio Future, not a coroutine.
        task = asyncio.ensure_future(
            discovery.scan_mdns_services(["_rdisccancel._tcp"], 0.5)
        )
        await asyncio.sleep(0.05)
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task
        # Let the browse finish and attempt to complete the future.
        await asyncio.sleep(2.0)
        assert not [
            e
            for e in errors
            if isinstance(e.get("exception"), asyncio.InvalidStateError)
        ]


class TestScanMdnsServices:
    async def test_resolves_announced_service(self):
        zeroconf = pytest.importorskip("zeroconf")
        from zeroconf.asyncio import AsyncZeroconf

        service_type = "_rdisctest._tcp.local."
        info = zeroconf.ServiceInfo(
            service_type,
            f"disc.{service_type}",
            port=12345,
            addresses=[b"\x7f\x00\x00\x01"],
            properties={b"version": b"1.2.3"},
            server="disc.local.",
        )
        aiozc = AsyncZeroconf()
        await aiozc.async_register_service(info)
        try:
            services = await discovery.scan_mdns_services(
                ["_rdisctest._tcp"], 2.0
            )
            match = [s for s in services if s["name"] == "disc"]
            assert len(match) == 1
            service = match[0]
            assert service["service_type"] == "_rdisctest._tcp"
            assert service["host"] == "127.0.0.1"
            assert service["port"] == 12345
            assert service["server"] == "disc.local"
            txt = service["txt"]
            assert isinstance(txt, dict)
            assert txt["version"] == "1.2.3"
        finally:
            await aiozc.async_unregister_service(info)
            await aiozc.async_close()

    async def test_empty_types_yield_nothing(self):
        assert await discovery.scan_mdns_services([], 0.1) == []


class TestFindSubnetDevices:
    async def _serve(self, handler):
        return await asyncio.start_server(handler, "127.0.0.1", 0)

    async def test_confirms_grblhal_device(self):
        async def device(reader, writer):
            await reader.read(256)
            writer.write(b"[VER:1.1f.20230812:]\r\n[OPT:VNMSL,35,1024]\r\n")
            await writer.drain()
            await asyncio.sleep(0.5)
            writer.close()

        server = await self._serve(device)
        port = server.sockets[0].getsockname()[1]
        try:
            hits = await discovery.find_subnet_devices(
                [
                    {
                        "id": "grbl",
                        "driver_name": "grblDriver",
                        "label": "grbl",
                        "firmware": "grbl",
                        "port": port,
                        "probe_command": "$I\n",
                        "match_prefix": "[VER:",
                    }
                ],
                [("127.0.0.1", 32)],
            )
            assert len(hits) == 1
            assert hits[0].driver_name == "grblDriver"
            assert hits[0].params["host"] == "127.0.0.1"
            assert hits[0].params["port"] == port
            assert hits[0].identity.banner == "[VER:1.1f.20230812:]"
        finally:
            server.close()
            await server.wait_closed()

    async def test_non_matching_listener_is_not_claimed(self):
        async def device(reader, writer):
            await reader.read(256)
            writer.write(b"220 router.example.com ESMTP\r\n")
            await writer.drain()
            writer.close()

        server = await self._serve(device)
        port = server.sockets[0].getsockname()[1]
        try:
            hits = await discovery.find_subnet_devices(
                [
                    {
                        "id": "grbl",
                        "driver_name": "grblDriver",
                        "label": "grbl",
                        "firmware": "grbl",
                        "port": port,
                        "probe_command": "$I\n",
                        "match_prefix": "[VER:",
                    }
                ],
                [("127.0.0.1", 32)],
            )
            assert hits == []
        finally:
            server.close()
            await server.wait_closed()

    async def test_spec_without_probe_claims_any_listener(self):
        async def device(reader, writer):
            writer.close()

        server = await self._serve(device)
        port = server.sockets[0].getsockname()[1]
        try:
            hits = await discovery.find_subnet_devices(
                [
                    {
                        "id": "any",
                        "driver_name": "anyDriver",
                        "label": "any",
                        "firmware": "any",
                        "port": port,
                    }
                ],
                [("127.0.0.1", 32)],
            )
            assert len(hits) == 1
            assert hits[0].driver_name == "anyDriver"
            assert hits[0].identity.banner is None
        finally:
            server.close()
            await server.wait_closed()

    async def test_first_matching_spec_claims_host(self):
        async def device(reader, writer):
            await reader.read(256)
            writer.write(b"[VER:1.1f.20230812:]\r\n")
            await writer.drain()
            await asyncio.sleep(0.5)
            writer.close()

        server = await self._serve(device)
        port = server.sockets[0].getsockname()[1]
        try:
            hits = await discovery.find_subnet_devices(
                [
                    {
                        "id": "first",
                        "driver_name": "firstDriver",
                        "label": "first",
                        "firmware": "first",
                        "port": port,
                        "probe_command": "$I\n",
                        "match_prefix": "[VER:",
                    },
                    {
                        "id": "second",
                        "driver_name": "secondDriver",
                        "label": "second",
                        "firmware": "second",
                        "port": port,
                        "probe_command": "$I\n",
                        "match_prefix": "[VER:",
                    },
                ],
                [("127.0.0.1", 32)],
            )
            assert len(hits) == 1
            assert hits[0].driver_name == "firstDriver"
        finally:
            server.close()
            await server.wait_closed()

    async def test_no_specs_yields_nothing(self):
        hits = await discovery.find_subnet_devices([], [("127.0.0.1", 32)])
        assert hits == []
