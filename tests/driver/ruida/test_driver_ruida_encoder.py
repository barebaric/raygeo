"""Ruida Ops encoder tests: the native encoder must reproduce the
Python encoder's golden transcript and the analyzer's wire bytes."""

import json
import re
from pathlib import Path

import pytest

from raygeo.driver.ruida import encode_script
from raygeo.ops import Ops
from raygeo.ops.convert import Encoder, RuidaSpec
from raygeo.ops.state import AirAssistMode
from raygeo.ops.types import RasterMode, SectionType

GOLDEN = Path(__file__).parent / "golden"
FLOAT = re.compile(r"-?\d+\.\d+(?:[eE][+-]?\d+)?|-?\d+[eE][+-]?\d+")


def normalize(text):
    return FLOAT.sub(lambda match: f"{float(match.group()):.10f}", text)


def build_representative_ops():
    ops = Ops()
    ops.job_start()
    ops.layer_start(layer_uid="layer-0")
    ops.workpiece_start("wp-0")
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp-0")
    ops.set_power(0.6)
    ops.set_feed_rate(250)
    ops.set_frequency(25000)
    ops.set_pulse_width(50)
    ops.set_air_assist(AirAssistMode.ON)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(5.0, 5.0, 0.0)
    ops.arc_to(10.0, 0.0, 5.0, 0.0, clockwise=True)
    ops.set_air_assist(AirAssistMode.OFF)
    ops.dwell(250)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    ops.workpiece_end("wp-0")
    ops.layer_end(layer_uid="layer-0")

    ops.layer_start(layer_uid="layer-1")
    ops.workpiece_start("wp-1")
    ops.ops_section_start(
        SectionType.RASTER_FILL,
        "wp-1",
        raster_mode=RasterMode.VARIABLE_POWER,
    )
    ops.set_power(0.05)
    ops.set_feed_rate(150)
    ops.set_frequency(30000)
    ops.move_to(20.0, 20.0, 0.0)
    ops.line_to(50.0, 20.0, 0.0)
    ops.move_to(0.0, 0.0, 0.0)
    ops.scan_to(5.0, 0.0, 0.0, bytearray([0, 128, 255, 128, 0]))
    ops.ops_section_end(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.workpiece_end("wp-1")
    ops.layer_end(layer_uid="layer-1")

    ops.layer_start(layer_uid="layer-2")
    ops.workpiece_start("wp-2")
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp-2")
    ops.move_to(55.0, 22.0, 0.0)
    ops.line_to(57.0, 22.0, 0.0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    ops.workpiece_end("wp-2")
    ops.layer_end(layer_uid="layer-2")
    ops.job_end()
    return ops


def context():
    return {
        "job_label": "Rayforge Job",
        "ref_point": "MACHINE",
        "power_cap": None,
        "power_floor_pct": 8.0,
        "power_floor_enabled": False,
        "image_power_bias_pct": 8.0,
        "power_scaling_enabled": True,
        "head_tool_numbers": {"laser-1": 1, "laser-2": 2},
        "layers": [
            {
                "uid": "layer-0",
                "name": "Cut",
                "color": "#ff6600",
                "speed_mm_s": 5.0,
                "frequency_khz": 20.0,
                "power_pct": 50.0,
            },
            {
                "uid": "layer-1",
                "name": "Engrave",
                "color": "#33cc33",
                "speed_mm_s": 2.5,
                "frequency_khz": 30.0,
                "power_pct": 50.0,
            },
            {
                "uid": "layer-2",
                "name": "Default",
                "color": "#00ccff",
                "speed_mm_s": 100.0,
                "frequency_khz": 20.0,
                "power_pct": 20.0,
            },
        ],
    }


def payload_bytes(result):
    payload = result.payload
    assert payload is not None
    return bytes(payload)


def encode():
    ops = build_representative_ops()
    spec = RuidaSpec(json.dumps(context()))
    return ops, ops.encode(Encoder(spec))


class TestGolden:
    def test_transcript_matches_the_python_encoder(self):
        _ops, result = encode()
        expected = (GOLDEN / "transcript.cglu").read_text()
        assert normalize(result.text) + "\n" == normalize(expected)

    def test_payload_encodes_to_the_golden_bytes(self):
        _ops, result = encode()
        lines = payload_bytes(result).decode().splitlines()
        actual = bytes(encode_script(lines)).hex().upper()
        assert actual == (GOLDEN / "job.hex").read_text().strip()

    def test_op_map_covers_every_op(self):
        ops, result = encode()
        op_map = result.op_to_machine_code
        line_ops = result.machine_code_to_op
        assert op_map is not None
        assert line_ops is not None
        assert len(op_map) == ops.len() * 8
        assert len(line_ops) >= 1
        assert payload_bytes(result)

    def test_scan_only_raster_derives_overscan(self):
        # A raster layer whose fill is only SCAN_LINE ops still derives
        # the horizontal overscan form.
        ops = Ops()
        ops.job_start()
        ops.layer_start(layer_uid="layer-0")
        ops.workpiece_start("wp")
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.move_to(0.0, 0.0, 0.0)
        ops.scan_to(5.0, 0.0, 0.0, bytearray([128, 128]))
        ops.ops_section_end(
            SectionType.RASTER_FILL,
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.workpiece_end("wp")
        ops.layer_end(layer_uid="layer-0")
        ops.job_end()
        result = ops.encode(Encoder(RuidaSpec(json.dumps(context()))))
        text = result.text or ""
        assert "declare_layer('Cut', '#ff6600', 'IMAGE', 'X_BI'," in text


class TestPolicy:
    def test_empty_ops(self):
        result = Ops().encode(Encoder(RuidaSpec(json.dumps(context()))))
        assert result.text == ""
        assert payload_bytes(result) == b""

    def test_missing_job_end_raises(self):
        ops = Ops()
        ops.job_start()
        with pytest.raises(Exception, match="JOB_START"):
            ops.encode(Encoder(RuidaSpec(json.dumps(context()))))

    def test_unknown_layer_raises(self):
        ops = Ops()
        ops.job_start()
        ops.layer_start(layer_uid="missing")
        ops.workpiece_start("wp")
        ops.workpiece_end("wp")
        ops.layer_end(layer_uid="missing")
        ops.job_end()
        with pytest.raises(Exception, match="not found"):
            ops.encode(Encoder(RuidaSpec(json.dumps(context()))))
