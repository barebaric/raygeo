"""Tests for raygeo.driver.analyzer.capture."""

from raygeo.driver.analyzer.capture import Capture, Frame, Recorder
from raygeo.driver.core import DriverEvent


def test_frame_payload_roundtrip():
    text_frame = Frame("tx", 0.0, text="$I")
    binary_frame = Frame("rx", 0.1, data=b"\x88\xa8\x00")
    assert text_frame.payload() == b"$I"
    assert binary_frame.payload() == b"\x88\xa8\x00"


def test_capture_save_load_roundtrip(tmp_path):
    path = tmp_path / "cap.jsonl"
    capture = Capture(protocol="grbl", source="probe")
    capture.append("tx", text="$I")
    capture.append("rx", text="[VER:1.1h.20190825:]")
    capture.append("rx", data=b"\x88\xa8")
    capture.append_note("client connected from 127.0.0.1:5000")
    capture.save(path)

    loaded = Capture.load(path)
    assert loaded.protocol == "grbl"
    assert loaded.source == "probe"
    assert len(loaded.frames) == 4
    assert loaded.frames[0].text == "$I"
    assert loaded.frames[2].data == b"\x88\xa8"
    assert (loaded.frames[3].note or "").startswith("client connected")
    assert [f.offset for f in loaded.frames] == sorted(
        f.offset for f in loaded.frames
    )


def test_capture_load_rejects_garbage(tmp_path):
    path = tmp_path / "bad.jsonl"
    path.write_text('{"meta": {}}\n{"nonsense": true}\n')
    try:
        Capture.load(path)
    except ValueError as err:
        assert "bad frame record" in str(err)
    else:
        raise AssertionError("expected ValueError")


def test_recorder_records_raw_events_and_forwards():
    forwarded = []
    recorder = Recorder(forward=forwarded.append)
    recorder(DriverEvent.RawTx("$I"))
    recorder(DriverEvent.RawRx("ok"))
    directions = [f.direction for f in recorder.capture.frames]
    assert directions == ["tx", "rx"]
    assert recorder.capture.frames[0].text == "$I"
    assert len(forwarded) == 2
    assert forwarded[0].line == "$I"
    assert forwarded[1].line == "ok"


def test_recorder_records_binary_events_and_forwards():
    forwarded = []
    recorder = Recorder(forward=forwarded.append)
    recorder(DriverEvent.RawTxBytes(data=b"\x88\x01"))
    recorder(DriverEvent.RawRxBytes(data=[0x88, 0x02]))
    directions = [f.direction for f in recorder.capture.frames]
    assert directions == ["tx", "rx"]
    assert recorder.capture.frames[0].data == b"\x88\x01"
    assert recorder.capture.frames[1].data == b"\x88\x02"
    assert all(frame.text is None for frame in recorder.capture.frames)
    assert len(forwarded) == 2
    assert bytes(forwarded[0].data) == b"\x88\x01"
    assert bytes(forwarded[1].data) == b"\x88\x02"
