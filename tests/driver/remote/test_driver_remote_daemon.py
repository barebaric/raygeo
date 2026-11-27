"""Integration tests for raygeo.driver.remote.daemon.

Drives the real daemon over real sockets against the GRBL firmware
emulator: open/attach/command/event flows, job survival across
client disconnects, exact reattach replay, lease contention, auth
and TLS, keepalive and slow-client protection.
"""

import asyncio
import json
import struct
import time
from pathlib import Path
from typing import Any

import pytest
from conftest import DEFAULT_DIALECT, FAST_TIMINGS, server_port

from raygeo.driver.analyzer.listener import (
    InsecureListenerError,
    make_client_ssl_context,
)
from raygeo.driver.core.transport import MockTransport
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.grbl.emulator import GrblEmulator
from raygeo.driver.remote import protocol
from raygeo.driver.remote.daemon import ClientQueue, Daemon, DaemonConfig

CERTS = Path(__file__).parents[1] / "certs"
CLIENTS: list["DaemonClient"] = []


@pytest.fixture(autouse=True)
async def _cleanup_clients():
    yield
    for client in list(CLIENTS):
        client.writer.transport.abort()
    CLIENTS.clear()
    current = asyncio.current_task()
    for task in asyncio.all_tasks():
        if task is not current:
            task.cancel()
    await asyncio.sleep(0)


CA = CERTS / "daemon-test-ca.pem"
CERT = CERTS / "daemon-test-cert.pem"
KEY = CERTS / "daemon-test-key.pem"


def job_gcode(lines):
    return "".join(f"G0 X{i}\n" for i in range(lines))


def op_map(lines):
    return [[i, i] for i in range(lines)]


async def read_frame(reader):
    prefix = await reader.readexactly(4)
    (size,) = struct.unpack(">I", prefix)
    payload = await reader.readexactly(size)
    return json.loads(payload)


class DaemonClient:
    """Minimal protocol client: requests, event log, auto-pong."""

    def __init__(self, reader, writer):
        self.reader = reader
        self.writer = writer
        self.events = []
        self.welcome: dict[str, Any] | None = None
        self._id = 0

    async def _pump(self):
        msg = await read_frame(self.reader)
        type_ = msg.get("type")
        if type_ == protocol.PING:
            await protocol.write_frame(self.writer, {"type": protocol.PONG})
            return None
        if type_ == protocol.EVENT:
            self.events.append(msg["payload"])
            return None
        return msg

    async def request(self, type_, payload=None):
        self._id += 1
        await protocol.write_frame(
            self.writer, protocol.request(self._id, type_, payload)
        )
        while True:
            msg = await self._pump()
            if msg is not None:
                assert msg.get("id") == self._id, msg
                return msg

    async def wait_for_event(self, kind, pred=None, timeout=10.0):
        deadline = time.monotonic() + timeout
        while True:
            for payload in self.events:
                event = payload["event"]
                if event.get("kind") == kind and (pred is None or pred(event)):
                    return payload
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                kinds = [p["event"].get("kind") for p in self.events]
                raise AssertionError(
                    f"event {kind!r} not observed; got {kinds}"
                )
            try:
                await asyncio.wait_for(self._pump(), timeout=remaining)
            except asyncio.TimeoutError:
                kinds = [p["event"].get("kind") for p in self.events]
                raise AssertionError(
                    f"event {kind!r} timed out; got {kinds}"
                ) from None

    async def collect(self, duration):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            try:
                await asyncio.wait_for(
                    self._pump(), timeout=deadline - time.monotonic()
                )
            except asyncio.TimeoutError:
                break

    async def open_session(self, role="control", force=False):
        reply = await self.request(
            protocol.SESSION_OPEN,
            {"protocol": "GrblSerialDriver", "role": role, "force": force},
        )
        assert reply["ok"], reply
        return reply["payload"]["session_id"]

    async def connect_session(self, sid):
        reply = await self.request(
            protocol.SESSION_COMMAND,
            {"session_id": sid, "method": "connect"},
        )
        assert reply["ok"], reply
        await self.wait_for_event(
            "connection_status", lambda e: e["status"] == "CONNECTED"
        )

    async def run_job(self, sid, lines):
        reply = await self.request(
            protocol.SESSION_COMMAND,
            {
                "session_id": sid,
                "method": "run",
                "args": {
                    "gcode": job_gcode(lines),
                    "op_line_map": op_map(lines),
                },
            },
        )
        assert reply["ok"], reply
        return reply["payload"]["job"]

    async def wait_job_done(self, sid, timeout=20.0):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            reply = await self.request(protocol.SESSION_LIST)
            sessions = reply["payload"]["sessions"]
            job = next(s["job"] for s in sessions if s["session_id"] == sid)
            if job is not None and not job["running"]:
                return job
            await asyncio.sleep(0.05)
        raise AssertionError("job did not finish in time")


async def connect_client(port, token=None, ssl_context=None):
    reader, writer = await asyncio.open_connection(
        "127.0.0.1",
        port,
        ssl=ssl_context,
        server_hostname="localhost" if ssl_context else None,
    )
    client = DaemonClient(reader, writer)
    welcome = await read_frame(reader)
    assert welcome["type"] == protocol.WELCOME
    client.welcome = welcome["payload"]
    if welcome["payload"]["auth_required"]:
        await protocol.write_frame(
            writer, protocol.request(0, protocol.AUTH, {"token": token})
        )
        reply = await read_frame(reader)
        assert reply["ok"], reply
    CLIENTS.append(client)
    return client


@pytest.fixture
async def rig():
    """A daemon hosting emulator-backed grbl sessions."""
    mock = MockTransport()
    emulator = GrblEmulator(mock, speed_factor=10)
    device_task = asyncio.ensure_future(emulator.run())

    def factory(protocol_name, config, dialect, event_callback):
        assert protocol_name == "GrblSerialDriver"
        return GrblSerialDriver.with_transport(
            mock,
            config={**FAST_TIMINGS, **(config or {})},
            dialect={**DEFAULT_DIALECT, **(dialect or {})},
            event_callback=event_callback,
        )

    daemon = Daemon(
        DaemonConfig(port=0, keepalive_interval=0.5, lease_grace=0.15),
        session_factory=factory,
    )
    server = await daemon.start()
    port = server_port(server)
    try:
        yield daemon, port, mock
    finally:
        await daemon.stop()
        device_task.cancel()
        try:
            await device_task
        except asyncio.CancelledError:
            pass


async def test_welcome_and_device_list(rig):
    daemon, port, mock = rig
    daemon.config.cameras = [
        {"name": "cam", "uri": "http://pi:8080/stream", "type": "http_stream"}
    ]
    client = await connect_client(port)
    assert client.welcome is not None
    assert client.welcome["protocol"] == protocol.PROTOCOL_VERSION
    assert isinstance(client.welcome["raydriver"], str)
    assert client.welcome["auth_required"] is False
    reply = await client.request(protocol.DEVICE_LIST)
    assert reply["ok"]
    assert isinstance(reply["payload"]["ports"], list)
    assert reply["payload"]["cameras"] == daemon.config.cameras


async def test_open_connect_command_events(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    await client.connect_session(sid)
    reply = await client.request(
        protocol.SESSION_COMMAND,
        {
            "session_id": sid,
            "method": "execute_interactive_command",
            "args": {"command": "$I"},
        },
    )
    assert reply["ok"], reply
    await client.wait_for_event("state")
    reply = await client.request(
        protocol.SESSION_PROPERTY,
        {"session_id": sid, "name": "state"},
    )
    assert reply["ok"]
    assert "status" in reply["payload"]["value"]
    reply = await client.request(
        protocol.SESSION_PROPERTY,
        {"session_id": sid, "name": "job_running"},
    )
    assert reply["payload"]["value"] is False


async def test_run_job_progress_and_finish(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    await client.connect_session(sid)
    handle = await client.run_job(sid, 10)
    assert isinstance(handle, str)
    await client.wait_for_event("job_finished")
    progress = [
        p["event"]["op"]
        for p in client.events
        if p["event"].get("kind") == "progress"
    ]
    assert progress == list(range(10))
    job = await client.wait_job_done(sid)
    assert job["handle"] == handle
    assert job["total_ops"] == 10
    assert job["current_op"] == 9


async def test_job_survives_client_disconnect(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    await client.connect_session(sid)
    await client.run_job(sid, 40)
    await client.wait_for_event("progress")
    client.writer.close()

    watcher = await connect_client(port)
    job = await watcher.wait_job_done(sid)
    assert job["current_op"] == 39
    assert b"G0 X39" in b"".join(mock.sent())


async def test_reattach_replays_exact_events(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    await client.connect_session(sid)
    await client.run_job(sid, 10)
    await client.wait_for_event("job_finished")
    await client.collect(0.05)
    recorded = list(client.events)
    mid = recorded[len(recorded) // 2]["seq"]

    other = await connect_client(port)
    reply = await other.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "after_seq": mid, "role": "watch"},
    )
    assert reply["ok"], reply
    assert "replayed" in reply["payload"]
    expected = [p for p in recorded if p["seq"] > mid]
    got = [p for p in other.events if p["seq"] > mid]
    # The daemon keeps emitting status polls while the test runs, so
    # the replayed ring can extend beyond what the first client read.
    # The replay must still reproduce the recorded window exactly.
    assert got[: len(expected)] == expected
    assert [p["seq"] for p in got] == list(range(mid + 1, mid + 1 + len(got)))


async def test_first_attach_gets_snapshot(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    other = await connect_client(port)
    reply = await other.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "role": "watch"},
    )
    snapshot = reply["payload"]["snapshot"]
    assert snapshot["connection_status"] == "DISCONNECTED"
    assert snapshot["job"] is None
    assert snapshot["state"]["status"] == "UNKNOWN"
    assert snapshot["next_seq"] >= 1


async def test_lease_contention_and_force_steal(rig):
    daemon, port, mock = rig
    holder = await connect_client(port)
    sid = await holder.open_session()
    contender = await connect_client(port)
    reply = await contender.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "role": "control"},
    )
    assert not reply["ok"]
    assert reply["error"]["code"] == protocol.LEASE_TAKEN

    reply = await contender.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "role": "control", "force": True},
    )
    assert reply["ok"], reply
    assert reply["payload"]["snapshot"] is not None
    await holder.wait_for_event("lease", lambda e: e["reason"] == "stolen")
    reply = await holder.request(
        protocol.SESSION_COMMAND,
        {"session_id": sid, "method": "connect"},
    )
    assert not reply["ok"]
    assert reply["error"]["code"] == protocol.LEASE_HELD


async def test_watcher_gets_events_but_no_commands(rig):
    daemon, port, mock = rig
    holder = await connect_client(port)
    sid = await holder.open_session()
    watcher = await connect_client(port)
    reply = await watcher.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "role": "watch"},
    )
    assert reply["ok"]
    reply = await watcher.request(
        protocol.SESSION_COMMAND,
        {"session_id": sid, "method": "connect"},
    )
    assert reply["error"]["code"] == protocol.LEASE_HELD
    await holder.connect_session(sid)
    await watcher.wait_for_event(
        "connection_status", lambda e: e["status"] == "CONNECTED"
    )


async def test_grace_reconnect_keeps_control(rig):
    daemon, port, mock = rig
    holder = await connect_client(port)
    sid = await holder.open_session()
    holder.writer.close()
    await asyncio.sleep(0.05)
    returning = await connect_client(port)
    reply = await returning.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "role": "control"},
    )
    assert reply["ok"], reply
    reply = await returning.request(
        protocol.SESSION_COMMAND,
        {"session_id": sid, "method": "connect"},
    )
    assert reply["ok"], reply


async def test_grace_expires_and_releases_lease(rig):
    daemon, port, mock = rig
    holder = await connect_client(port)
    sid = await holder.open_session()
    watcher = await connect_client(port)
    await watcher.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "role": "watch"},
    )
    holder.writer.close()
    await watcher.collect(0.4)
    await watcher.wait_for_event("lease", lambda e: e["reason"] == "released")
    reply = await watcher.request(
        protocol.SESSION_ATTACH,
        {"session_id": sid, "role": "control"},
    )
    assert reply["ok"], reply


async def test_detach_keeps_job_running(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    await client.connect_session(sid)
    await client.run_job(sid, 10)
    reply = await client.request(protocol.SESSION_DETACH, {"session_id": sid})
    assert reply["ok"]
    job = await client.wait_job_done(sid)
    assert job["current_op"] == 9


async def test_session_close(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    await client.connect_session(sid)
    reply = await client.request(protocol.SESSION_CLOSE, {"session_id": sid})
    assert reply["ok"]
    listing = await client.request(protocol.SESSION_LIST)
    assert listing["payload"]["sessions"] == []
    assert not mock.is_connected


async def test_run_while_running_rejected(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    await client.connect_session(sid)
    await client.run_job(sid, 60)
    reply = await client.request(
        protocol.SESSION_COMMAND,
        {
            "session_id": sid,
            "method": "run",
            "args": {"gcode": "G0 X0\n"},
        },
    )
    assert reply["error"]["code"] == protocol.JOB_RUNNING


async def test_unknown_type_method_session_property(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    reply = await client.request("bogus/type")
    assert reply["error"]["code"] == protocol.UNKNOWN_TYPE
    sid = await client.open_session()
    reply = await client.request(
        protocol.SESSION_COMMAND,
        {"session_id": sid, "method": "bogus"},
    )
    assert reply["error"]["kind"] == "unsupported"
    reply = await client.request(
        protocol.SESSION_PROPERTY,
        {"session_id": sid, "name": "bogus"},
    )
    assert reply["error"]["code"] == protocol.UNKNOWN_PROPERTY
    reply = await client.request(
        protocol.SESSION_COMMAND,
        {"session_id": "nope", "method": "connect"},
    )
    assert reply["error"]["code"] == protocol.UNKNOWN_SESSION


async def test_auto_open_sessions():
    def factory(protocol_name, config, dialect, event_callback):
        return GrblSerialDriver.with_transport(
            MockTransport(),
            config=config,
            dialect=dialect,
            event_callback=event_callback,
        )

    daemon = Daemon(
        DaemonConfig(port=0, sessions=[{"protocol": "GrblSerialDriver"}]),
        session_factory=factory,
    )
    server = await daemon.start()
    try:
        client = await connect_client(server_port(server))
        listing = await client.request(protocol.SESSION_LIST)
        sessions = listing["payload"]["sessions"]
        assert len(sessions) == 1
        assert sessions[0]["config"] == {}
    finally:
        await daemon.stop()


async def test_token_auth_roundtrip():
    daemon = Daemon(DaemonConfig(port=0, token="secret"))
    server = await daemon.start()
    try:
        client = await connect_client(server_port(server), token="secret")
        reply = await client.request(protocol.DEVICE_LIST)
        assert reply["ok"]
    finally:
        await daemon.stop()


async def test_wrong_token_closes_connection():
    daemon = Daemon(DaemonConfig(port=0, token="secret"))
    server = await daemon.start()
    try:
        reader, writer = await asyncio.open_connection(
            "127.0.0.1", server_port(server)
        )
        welcome = await read_frame(reader)
        assert welcome["payload"]["auth_required"] is True
        await protocol.write_frame(
            writer, protocol.request(1, protocol.AUTH, {"token": "x"})
        )
        reply = await read_frame(reader)
        assert reply["error"]["code"] == protocol.AUTH_FAILED
        assert await asyncio.wait_for(reader.read(), 5.0) == b""
        writer.close()
    finally:
        await daemon.stop()


async def test_tls_roundtrip():
    daemon = Daemon(
        DaemonConfig(
            port=0,
            token="secret",
            tls_cert=str(CERT),
            tls_key=str(KEY),
        )
    )
    server = await daemon.start()
    try:
        client = await connect_client(
            server_port(server),
            token="secret",
            ssl_context=make_client_ssl_context(str(CA)),
        )
        reply = await client.request(protocol.DEVICE_LIST)
        assert reply["ok"]
    finally:
        await daemon.stop()


async def test_bind_policy_rejects_exposed_without_auth():
    with pytest.raises(InsecureListenerError):
        await Daemon(DaemonConfig(host="0.0.0.0")).start()
    with pytest.raises(InsecureListenerError):
        await Daemon(DaemonConfig(host="10.0.0.5", token="secret")).start()


async def test_keepalive_drops_silent_peer():
    daemon = Daemon(
        DaemonConfig(port=0, keepalive_interval=0.05, keepalive_misses=2)
    )
    server = await daemon.start()
    try:
        reader, writer = await asyncio.open_connection(
            "127.0.0.1", server_port(server)
        )
        await protocol.read_frame(reader)  # welcome, then silence
        await asyncio.wait_for(reader.read(), 3.0)
        assert reader.at_eof()
        writer.close()
    finally:
        await daemon.stop()


async def test_client_queue_drops_raw_first():
    queue = ClientQueue(limit=4)
    raw = protocol.event_frame("s", 1, {"kind": "raw_rx", "line": "ok"})
    state = protocol.event_frame("s", 2, {"kind": "state"})
    queue.put(raw)
    queue.put(raw)
    queue.put(state)
    queue.put(state)
    queue.put(state)  # overflow: the two raw frames make room
    drained = [await queue.get() for _ in range(3)]
    assert all(
        isinstance(frame["payload"], dict)
        and isinstance(frame["payload"]["event"], dict)
        and frame["payload"]["event"]["kind"] == "state"
        for frame in drained
    )


async def test_client_queue_overflow_without_raw():
    queue = ClientQueue(limit=2)
    state = protocol.event_frame("s", 1, {"kind": "state"})
    queue.put(state)
    queue.put(state)
    with pytest.raises(OverflowError):
        queue.put(state)


async def test_client_list_and_connection_status(rig):
    daemon, port, mock = rig
    client = await connect_client(port)
    sid = await client.open_session()
    reply = await client.request(protocol.CLIENT_LIST)
    assert reply["ok"]
    clients = reply["payload"]["clients"]
    assert len(clients) == 1
    assert clients[0]["address"].startswith("127.0.0.1:")
    assert clients[0]["authenticated"] is True
    assert clients[0]["roles"] == {sid: "control"}
    listing = await client.request(protocol.SESSION_LIST)
    session = listing["payload"]["sessions"][0]
    assert session["connection_status"] == "DISCONNECTED"
