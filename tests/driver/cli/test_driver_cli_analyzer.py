"""Tests for the raydriver analyzer CLI."""

import asyncio
import inspect
import json
from pathlib import Path

import pytest

from raygeo.driver.analyzer.capture import Capture
from raygeo.driver.analyzer.decode import render_listing
from raygeo.driver.cli.analyzer import (
    _parse_endpoint,
    _validate_source,
    cmd_decode,
    cmd_replay,
    cmd_tui,
)
from raygeo.driver.cli.main import build_parser

GRBL_GOLDEN = Path(__file__).parents[1] / "grbl" / "golden"


def run_cli(argv):
    args = build_parser().parse_args(argv)
    return asyncio.run(args.func(args))


def test_parse_endpoint():
    assert _parse_endpoint("192.168.1.5:23") == ("192.168.1.5", 23)
    assert _parse_endpoint("host.example:8080") == (
        "host.example",
        8080,
    )


def test_parser_wires_analyzer_subcommands():
    parser = build_parser()
    args = parser.parse_args(["analyzer", "decode", "cap.jsonl"])
    assert args.func is cmd_decode
    args = parser.parse_args(
        ["analyzer", "proxy", "--upstream", "h:1", "--out", "x.jsonl"]
    )
    assert args.upstream == ("h", 1)
    assert args.listen_host == "127.0.0.1"
    assert "analyzer" in parser.format_help()


def test_decode_prints_listing(capsys):
    code = run_cli(
        ["analyzer", "decode", str(GRBL_GOLDEN / "grbl-session.jsonl")]
    )
    assert code == 0
    out = capsys.readouterr().out
    assert "build info request" in out
    assert "ack" in out


def test_record_emulator_writes_capture(tmp_path, capsys):
    out = tmp_path / "cap.jsonl"
    code = run_cli(
        [
            "analyzer",
            "record",
            "--emulator",
            "--seconds",
            "0.5",
            "--out",
            str(out),
        ]
    )
    assert code == 0
    lines = out.read_text().splitlines()
    meta = json.loads(lines[0])["meta"]
    assert meta["protocol"] == "grbl"
    frames = [json.loads(line) for line in lines[1:]]
    assert any(frame["d"] == "tx" for frame in frames)
    assert any(frame["d"] == "rx" for frame in frames)


async def test_replay_command_against_fake_device(tmp_path):
    async def grblish_device(reader, writer):
        try:
            while True:
                line = await reader.readline()
                if not line:
                    return
                if line.decode().strip() == "$I":
                    writer.write(b"[VER:1.1h.20190825:]\nok\n")
                else:
                    writer.write(b"ok\n")
                await writer.drain()
        finally:
            writer.close()

    device = await asyncio.start_server(grblish_device, "127.0.0.1", 0)
    port = device.sockets[0].getsockname()[1]
    capture_path = tmp_path / "cap.jsonl"
    from raygeo.driver.analyzer.capture import Capture

    capture = Capture(protocol="grbl")
    capture.append("tx", text="$I")
    capture.append("tx", text="G1 X10")
    capture.save(capture_path)

    class Args:
        file = str(capture_path)
        to = ("127.0.0.1", port)
        speed = 0.0
        wait = 0.3
        token = None
        tls_ca = None
        tls_hostname = None
        protocol = None

    try:
        assert await cmd_replay(Args()) == 0
    finally:
        device.close()
        await device.wait_closed()


def test_parser_wires_ruida_analyzer_options():
    args = build_parser().parse_args(
        [
            "analyzer",
            "record",
            "--protocol",
            "ruida",
            "--host",
            "192.168.1.50",
            "--net-port",
            "50201",
            "--transport",
            "tcp",
            "--magic",
            "0x99",
        ]
    )
    assert args.protocol == "ruida"
    assert args.host == "192.168.1.50"
    assert args.net_port == 50201
    assert args.transport == "tcp"
    assert args.magic == 0x99


def test_ruida_source_validation():
    args = build_parser().parse_args(
        ["analyzer", "record", "--protocol", "ruida"]
    )
    with pytest.raises(ValueError, match="--host"):
        _validate_source(args)
    args = build_parser().parse_args(
        [
            "analyzer",
            "record",
            "--protocol",
            "ruida",
            "--host",
            "h",
            "--usb-device",
            "/dev/ttyUSB0",
        ]
    )
    with pytest.raises(ValueError, match="either"):
        _validate_source(args)
    args = build_parser().parse_args(
        ["analyzer", "record", "--protocol", "ruida", "--emulator"]
    )
    _validate_source(args)


def test_record_ruida_emulator_writes_binary_capture(tmp_path):
    out = tmp_path / "ruida.jsonl"
    code = run_cli(
        [
            "analyzer",
            "record",
            "--protocol",
            "ruida",
            "--emulator",
            "--seconds",
            "0.8",
            "--out",
            str(out),
        ]
    )
    assert code == 0
    capture = Capture.load(out)
    assert capture.protocol == "ruida"
    assert capture.source == "emulated Ruida controller"
    assert {frame.direction for frame in capture.frames} == {"tx", "rx"}
    assert any(frame.data is not None for frame in capture.frames)
    listing = render_listing(capture, with_times=False)
    assert "get 0x057e (card id)" in listing
    assert "ack" in listing


def test_tui_is_synchronous_and_uses_a_work_factory(monkeypatch):
    pytest.importorskip("curses")
    assert not inspect.iscoroutinefunction(cmd_tui)
    seen = {}

    def fake_run_tui(recorder, decoder, *, source, save_path, work):
        seen["source"] = source
        seen["save_path"] = save_path
        seen["work"] = work

    import raygeo.driver.analyzer.tui as tui

    monkeypatch.setattr(tui, "run_tui", fake_run_tui)
    args = build_parser().parse_args(
        ["analyzer", "tui", "--protocol", "ruida", "--emulator"]
    )
    assert cmd_tui(args) == 0
    assert seen["source"] == "emulated Ruida controller"
    assert seen["save_path"] is None
    assert callable(seen["work"])
