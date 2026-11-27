"""Tests for raygeo.driver.remote.protocol.

Codec tests run over in-memory streams; real sockets appear only
where the framing must survive a real transport.
"""

import asyncio
import json

import pytest
from conftest import server_port

from raygeo.driver.core import DriverEvent
from raygeo.driver.core.types import DeviceError, DeviceState, DeviceStatus
from raygeo.driver.remote import protocol


def reader_with(data: bytes) -> asyncio.StreamReader:
    """An in-memory stream that yields *data* and then EOF."""
    reader = asyncio.StreamReader()
    reader.feed_data(data)
    reader.feed_eof()
    return reader


def frame_bytes(payload: bytes) -> bytes:
    return protocol.LENGTH_PREFIX.pack(len(payload)) + payload


def full_state() -> DeviceState:
    state = DeviceState()
    state.status = DeviceStatus.RUN
    state.error = DeviceError(1, "Hard limit", "Hard limit tripped")
    state.machine_pos = [1.0, None, 3.5]
    state.work_pos = [0.5, 2.0, None]
    state.wco = [0.5, -2.0, 0.0]
    state.feed_rate = 1200
    state.spindle_speed = 1000
    state.buffer_available = 12
    state.buffer_rx_available = 1024
    return state


async def test_frame_roundtrip_in_memory():
    message = protocol.request(7, protocol.DEVICE_LIST, {"note": "ümlaut"})
    reader = reader_with(protocol.encode_frame(message))
    assert await protocol.read_frame(reader) == message


async def test_two_frames_in_order():
    first = protocol.request(1, protocol.PING)
    second = protocol.request(2, protocol.SESSION_LIST)
    reader = reader_with(
        protocol.encode_frame(first) + protocol.encode_frame(second)
    )
    assert await protocol.read_frame(reader) == first
    assert await protocol.read_frame(reader) == second


async def test_write_frame_real_socket():
    async def handler(reader, writer):
        try:
            message = await protocol.read_frame(reader)
            msg_id = message["id"]
            assert isinstance(msg_id, int)
            await protocol.write_frame(
                writer,
                protocol.response(msg_id, {"echo": message["payload"]}),
            )
        finally:
            writer.close()
            await writer.wait_closed()

    server = await asyncio.start_server(handler, "127.0.0.1", 0)
    port = server_port(server)
    reader, writer = await asyncio.open_connection("127.0.0.1", port)
    await protocol.write_frame(
        writer, protocol.request(7, protocol.DEVICE_LIST, {"x": "ü"})
    )
    reply = await protocol.read_frame(reader)
    assert reply == protocol.response(7, {"echo": {"x": "ü"}})
    writer.close()
    await writer.wait_closed()
    server.close()
    await server.wait_closed()


def test_encode_rejects_oversize(monkeypatch):
    monkeypatch.setattr(protocol, "MAX_FRAME_BYTES", 32)
    with pytest.raises(protocol.ProtocolError) as err:
        protocol.encode_frame({"data": "x" * 64})
    assert err.value.code == protocol.FRAME_TOO_LARGE


async def test_read_rejects_oversize_prefix(monkeypatch):
    monkeypatch.setattr(protocol, "MAX_FRAME_BYTES", 32)
    reader = reader_with(frame_bytes(b"x" * 64))
    with pytest.raises(protocol.ProtocolError) as err:
        await protocol.read_frame(reader)
    assert err.value.code == protocol.FRAME_TOO_LARGE


@pytest.mark.parametrize("data", [b"", b"\x00", frame_bytes(b"abc")[:6]])
async def test_read_truncated_raises_connection_error(data):
    with pytest.raises(ConnectionError):
        await protocol.read_frame(reader_with(data))


@pytest.mark.parametrize("payload", [b"not json", b"[1,2]", b"\xff\xfe"])
async def test_read_bad_payload(payload):
    with pytest.raises(protocol.ProtocolError) as err:
        await protocol.read_frame(reader_with(frame_bytes(payload)))
    assert err.value.code == protocol.BAD_REQUEST


def test_envelope_shapes():
    req = protocol.request(7, protocol.SESSION_COMMAND, {"method": "run"})
    assert req == {
        "id": 7,
        "type": "session/command",
        "payload": {"method": "run"},
    }
    assert protocol.response(7) == {"id": 7, "ok": True, "payload": {}}
    err = protocol.error_response(7, protocol.LEASE_TAKEN, "held by a")
    assert err["id"] == 7
    assert err["ok"] is False
    assert err["error"] == {"code": "lease_taken", "message": "held by a"}
    event = protocol.event_frame("s1", 3, {"kind": "job_finished"})
    assert event == {
        "type": "event",
        "payload": {
            "session": "s1",
            "seq": 3,
            "event": {"kind": "job_finished"},
        },
    }
    welcome = protocol.welcome_payload("1.0.0", True)
    assert welcome == {
        "protocol": 1,
        "raydriver": "1.0.0",
        "auth_required": True,
    }


def all_events():
    return [
        DriverEvent.StateChanged(full_state()),
        DriverEvent.ConnectionStatusChanged("CONNECTED", None),
        DriverEvent.ConnectionStatusChanged("SLEEPING", "link lost"),
        DriverEvent.CommandStatusChanged("ok", None),
        DriverEvent.JobFinished(),
        DriverEvent.ProbeStatusChanged("probing Z"),
        DriverEvent.WcsUpdated({"G54": (1.0, 2.0, 3.0)}),
        DriverEvent.ConfigChanged("rx_buffer_size", 128),
        DriverEvent.RawRx("ok"),
        DriverEvent.RawTx("G0 X1"),
        DriverEvent.RawRxBytes([0x88, 0xDA, 0x01]),
        DriverEvent.RawTxBytes([0xCC]),
    ]


@pytest.mark.parametrize(
    "event", all_events(), ids=lambda event: type(event).__name__
)
def test_event_roundtrip(event):
    wire = json.loads(json.dumps(protocol.encode_event(event)))
    decoded = protocol.decode_event(wire)
    assert decoded is not None
    assert type(decoded) is type(event)
    assert protocol.encode_event(decoded) == protocol.encode_event(event)


def test_decode_unknown_kind_returns_none():
    assert protocol.decode_event({"kind": "future_thing"}) is None
    assert protocol.decode_event({}) is None


def test_state_roundtrip_full():
    state = full_state()
    wire = json.loads(json.dumps(protocol.encode_state(state)))
    assert protocol.decode_state(wire) == state


def test_state_roundtrip_empty():
    state = DeviceState()
    wire = json.loads(json.dumps(protocol.encode_state(state)))
    assert protocol.decode_state(wire) == state


def test_decode_state_unknown_status():
    with pytest.raises(protocol.ProtocolError) as err:
        protocol.decode_state({"status": "NOT_A_STATUS"})
    assert err.value.code == protocol.BAD_REQUEST


def test_wcs_offsets_restored_as_tuples():
    event = DriverEvent.WcsUpdated(
        {"G54": (1.0, 2.0, 3.0), "G55": (0.0, 0.0, 0.0)}
    )
    wire = json.loads(json.dumps(protocol.encode_event(event)))
    decoded = protocol.decode_event(wire)
    assert isinstance(decoded, DriverEvent.WcsUpdated)
    assert decoded.offsets == event.offsets
    assert all(isinstance(value, tuple) for value in decoded.offsets.values())


def test_protocol_error_message():
    err = protocol.ProtocolError("busy")
    assert str(err) == "busy"


def test_request_types_exclude_pushes():
    assert protocol.WELCOME not in protocol.REQUEST_TYPES
    assert protocol.EVENT not in protocol.REQUEST_TYPES
    assert protocol.SESSION_COMMAND in protocol.REQUEST_TYPES
