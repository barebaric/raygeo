"""Tests for the Rust Epilog print-job encoder.

The encoder turns machine-space Ops into a PJL/PCL/HPGL job. It
returns the exact device byte stream as ``payload`` and a printable
``text`` listing for the UI, with an op-to-line map over the listing.

The wire layout follows LibLaserCut's ``EpilogCutter``: a PJL envelope,
an optional PCL raster part, an HPGL vector part, and 4096 zero bytes
of trailing padding.
"""

import json

import numpy as np
import pytest

from raygeo.cnc.execution.specs import ComputePayload, EncodeSpec
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.assembly.raster import RasterSpec
from raygeo.ops.convert import EncodeOutput, Encoder, EpilogSpec
from raygeo.ops.part import Part
from raygeo.ops.state import PowerMode
from raygeo.pipeline.execute import clear_cache
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

JOB_HEADER = b"\x1b%-12345X@PJL JOB NAME="
JOB_ENTER_PCL = b"\x1bE@PJL ENTER LANGUAGE=PCL\r\n"
JOB_FOOTER = b"\x1bE\x1b%-12345X@PJL EOJ \r\n"


def _ctx(**overrides) -> str:
    ctx = dict(
        title="Test Job",
        resolution=500,
        frequency=1000,
        bed_mm=[609.6, 304.8],
        max_cut_speed=6000.0,
        origin_top_left=True,
        rotary=False,
    )
    ctx.update(overrides)
    return json.dumps(ctx)


def _collect(nodes):
    completed = []

    def _batch(frac: float, msg: str) -> None:
        pass

    from raygeo.pipeline.execute import execute_stages

    try:
        execute_stages(nodes, completed.append, _batch)
    except RuntimeError:
        pass
    return {n.key: n for n in completed}


def _square_part() -> Part:
    from raygeo.geo import Geometry

    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.line_to(0, 0)
    return Part(geometry=g, size_mm=(10.0, 10.0))


def _raster_part(fill: int) -> Part:
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    part.image = np.full((100, 100), fill, dtype=np.uint8)
    return part


def _encode_part(part, assembler, ctx=None) -> EncodeOutput:
    clear_cache()
    src = NodeRequest(
        key="src",
        generation_id=1,
        stage=StageSpec.Compute(
            part=part,
            params=ComputePayload(
                assembler=Assembler(assembler),
                power=0.5,
                cut_speed=600,
            ),
        ),
    )
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src",
            encoder=Encoder(EpilogSpec(ctx or _ctx())),
        ),
    )
    node = _collect([src, enc])["enc"]
    assert node.error is None, node.error
    assert isinstance(node.output, EncodeOutput)
    return node.output


@pytest.fixture(autouse=True)
def _clear_cache():
    clear_cache()
    yield
    clear_cache()


# ── Job envelope ──────────────────────────────────────────────────


def test_payload_is_a_pjl_job_with_padding():
    out = _encode_part(_square_part(), ContourSpec())
    payload = bytes(out.payload)
    assert payload.startswith(JOB_HEADER + b"Test Job\r\n")
    assert JOB_ENTER_PCL in payload
    assert payload.rstrip(b"\x00").endswith(JOB_FOOTER)
    assert payload.endswith(bytes(4096))


def test_text_is_printable_and_escaped():
    out = _encode_part(_square_part(), ContourSpec())
    assert "\x1b" not in out.text
    assert "\\e%-12345X@PJL JOB NAME=Test Job" in out.text
    assert "@PJL ENTER LANGUAGE=PCL" in out.text


def test_vector_coordinates_use_device_dots():
    out = _encode_part(_square_part(), ContourSpec())
    # 10 mm at 500 DPI is 197 dots.
    assert "PD197,0;" in out.text
    assert "PD197,197;" in out.text
    assert "PU0,0;" in out.text


def test_power_and_speed_percentages():
    out = _encode_part(_square_part(), ContourSpec())
    # 0.5 power is 50 %; 600 mm/min of a 6000 reference is 10 %.
    assert "YP050;" in out.text
    assert "ZS010;" in out.text


def test_op_map_covers_every_op():
    out = _encode_part(_square_part(), ContourSpec())
    spans = np.frombuffer(out.op_to_machine_code, dtype=np.int32).reshape(
        -1, 2
    )
    line_ops = np.frombuffer(out.machine_code_to_op, dtype=np.int32)
    assert len(spans) >= 8
    assert line_ops.max() < len(spans)
    # The cut lines carry the moves.
    assert spans[3][1] >= 1


def test_zero_power_job_has_no_powered_paths():
    clear_cache()
    src = NodeRequest(
        key="src",
        generation_id=1,
        stage=StageSpec.Compute(
            part=_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()), power=0.0, cut_speed=600
            ),
        ),
    )
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src", encoder=Encoder(EpilogSpec(_ctx()))
        ),
    )
    node = _collect([src, enc])["enc"]
    assert node.error is not None
    assert "no powered paths" in node.error


def test_power_mode_op_is_accepted():
    """Laser steps and framing emit SET_POWER_MODE; the encoder must
    ignore it instead of rejecting the job (issue #800)."""

    clear_cache()
    src = NodeRequest(
        key="src",
        generation_id=1,
        stage=StageSpec.Compute(
            part=_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()),
                power=0.5,
                cut_speed=600,
                power_mode=PowerMode.CONSTANT,
            ),
        ),
    )
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src", encoder=Encoder(EpilogSpec(_ctx()))
        ),
    )
    node = _collect([src, enc])["enc"]
    assert node.error is None, node.error
    assert isinstance(node.output, EncodeOutput)


def test_dither_raster_uses_packbits_one_bit_rows():
    spec = RasterSpec(mode="dither", line_interval_mm=1.0, step_power=0.5)
    out = _encode_part(_raster_part(255), spec)
    assert "\\e*b2M" in out.text
    assert "\\e*b1M" not in out.text
    payload = bytes(out.payload)
    assert b"\x1b*b2M" in payload
    assert b"\x1b*rC" in payload


def test_power_modulated_raster_uses_8_bit_rows():
    spec = RasterSpec(
        mode="power_modulated", line_interval_mm=1.0, step_power=0.5
    )
    out = _encode_part(_raster_part(128), spec)
    assert "\\e*b1M\\e*b7W" in out.text
    payload = bytes(out.payload)
    assert b"\x1b*b1M" in payload


def test_raster_rows_are_inside_the_raster_part():
    spec = RasterSpec(mode="dither", line_interval_mm=1.0, step_power=0.5)
    out = _encode_part(_raster_part(255), spec)
    payload = bytes(out.payload)
    row = payload.find(b"\x1b*p")
    assert 0 < row < payload.find(b"\x1b*rC")


def test_raster_scan_moves_do_not_leak_into_vectors():
    spec = RasterSpec(mode="dither", line_interval_mm=1.0, step_power=0.5)
    out = _encode_part(_raster_part(255), spec)
    # No HPGL pen moves; only the closing pen-up before the footer.
    vectors = [
        line for line in out.text.splitlines() if line.startswith(("PU", "PD"))
    ]
    assert vectors == ["PU;YP000;WF0;"]


# ── Validation ────────────────────────────────────────────────────


def test_rejects_rotary_context():
    out_ctx = _ctx(rotary=True)
    clear_cache()
    src = NodeRequest(
        key="src",
        generation_id=1,
        stage=StageSpec.Compute(
            part=_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()), power=0.5, cut_speed=600
            ),
        ),
    )
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src", encoder=Encoder(EpilogSpec(out_ctx))
        ),
    )
    node = _collect([src, enc])["enc"]
    assert node.error is not None
    assert "flat XY" in node.error


def test_rejects_unsupported_resolution():
    clear_cache()
    src = NodeRequest(
        key="src",
        generation_id=1,
        stage=StageSpec.Compute(
            part=_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()), power=0.5, cut_speed=600
            ),
        ),
    )
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src",
            encoder=Encoder(EpilogSpec(_ctx(resolution=123))),
        ),
    )
    node = _collect([src, enc])["enc"]
    assert node.error is not None
    assert "resolution" in node.error


def test_rejects_bottom_origin_as_an_invariant_violation():
    """The embedding application constrains the machine origin to the
    driver's layout, so a bottom-left origin reaching the encoder is a
    caller bug. It still fails loudly instead of emitting a job whose
    coordinates would be wrong."""
    clear_cache()
    src = NodeRequest(
        key="src",
        generation_id=1,
        stage=StageSpec.Compute(
            part=_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()), power=0.5, cut_speed=600
            ),
        ),
    )
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src",
            encoder=Encoder(EpilogSpec(_ctx(origin_top_left=False))),
        ),
    )
    node = _collect([src, enc])["enc"]
    assert node.error is not None
    assert "internal error" in node.error
