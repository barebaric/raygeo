"""Tests for the Encode stage.

The Encode stage takes an upstream `Ops` (from a Compute or Aggregate
node) and produces a non-Ops artifact via the polymorphic `Encoder`
trait. Three encoder specs are tested here:

- `GcodeSpec` — produces machine-code text with op-index maps.
- `VertexSpec` — produces a vertex-array repr string.
- `TextureSpec` — produces a (width x height) uint8 power texture.

All three are exercised through the pipeline as `StageSpec.Encode`
nodes with `source_key` pointing at an upstream Compute node.
"""

import numpy as np
import pytest
from conftest import (
    collect_completions,
    encode_result,
    make_square_part,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    EncodeSpec,
    MachineParams,
)
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.convert import (
    Encoder,
    GcodeDialectSpec,
    GcodeSpec,
    SceneSpec,
    TextureSpec,
    VertexSpec,
)
from raygeo.pipeline.execute import execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _compute_src(key: str = "src") -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(assembler=Assembler(ContourSpec())),
        ),
    )


def _encode_node(key: str, source_key: str, encoder: Encoder) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=EncodeSpec(
            source_key=source_key,
            encoder=encoder,
        ),
    )


def _by_key(completed):
    return {c.key: c for c in completed}


# ── G-code encoder ────────────────────────────────────────────────


def test_gcode_encode_succeeds():
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            GcodeSpec(
                dialect=GcodeDialectSpec(),
                context_json="{}",
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    c = _by_key(completed)["enc"]
    assert c.error is None
    assert c.output is not None


def test_gcode_encode_carries_machine_code():
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            GcodeSpec(
                dialect=GcodeDialectSpec(),
                context_json="{}",
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    assert type(out).__name__ == "EncodeOutput"
    assert out.variant == "MachineCode"
    assert out.text is not None
    assert len(out.text) > 0
    assert out.power_texture is None
    assert out.repr is None


def test_gcode_encode_text_contains_g_code_commands():
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            GcodeSpec(
                dialect=GcodeDialectSpec(),
                context_json="{}",
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    text = encode_result(_by_key(completed)["enc"]).text
    assert text is not None
    assert "G1" in text or "G0" in text


def test_gcode_encode_op_maps_populated():
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            GcodeSpec(
                dialect=GcodeDialectSpec(),
                context_json="{}",
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    om = out.op_to_machine_code
    mo = out.machine_code_to_op
    assert om is not None and mo is not None
    assert len(om) > 0
    for op_idx, (start, count) in enumerate(om):
        for mi in range(start, start + count):
            assert mi < len(mo)
            assert mo[mi] == op_idx


# ── Vertex-array encoder ──────────────────────────────────────────


def test_vertex_encode_succeeds():
    src = _compute_src()
    enc = _encode_node("enc", "src", Encoder(VertexSpec()))
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    assert out.variant == "VertexArrays"
    assert out.repr is not None
    assert out.text is None
    assert out.power_texture is None


def test_texture_encode_succeeds():
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            TextureSpec(
                width_px=32,
                height_px=32,
                px_per_mm=(6.4, 6.4),
                origin_mm=(0.0, 0.0),
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    assert out.variant == "Texture"


def test_texture_encode_dimensions_match_spec():
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            TextureSpec(
                width_px=64,
                height_px=48,
                px_per_mm=(6.4, 6.4),
                origin_mm=(0.0, 0.0),
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    assert out.width_px == 64
    assert out.height_px == 48


def test_texture_encode_buffer_size():
    w, h = 32, 16
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            TextureSpec(
                width_px=w,
                height_px=h,
                px_per_mm=(6.4, 6.4),
                origin_mm=(0.0, 0.0),
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    assert out.power_texture is not None
    assert len(out.power_texture) == w * h


def test_texture_encode_buffer_is_uint8():
    src = _compute_src()
    enc = _encode_node(
        "enc",
        "src",
        Encoder(
            TextureSpec(
                width_px=16,
                height_px=16,
                px_per_mm=(6.4, 6.4),
                origin_mm=(0.0, 0.0),
            )
        ),
    )
    completed, _ = collect_completions([src, enc])
    bytes_ = encode_result(_by_key(completed)["enc"]).power_texture
    assert bytes_ is not None
    for b in bytes_:
        assert 0 <= b <= 255


# ── Topology: Encode on top of Aggregate ───────────────────────────


def test_encode_can_consume_aggregate():
    IDENTITY = [
        [1.0, 0, 0, 0],
        [0, 1.0, 0, 0],
        [0, 0, 1.0, 0],
        [0, 0, 0, 1.0],
    ]
    src = _compute_src("src")
    agg = NodeRequest(
        key="agg",
        generation_id=1,
        stage=StageSpec.Aggregate(
            spec=AggregateSpec(
                wrap_start=[],
                groups=[
                    AggregateGroup(
                        start_markers=[],
                        inputs=[
                            AggregateInput(
                                source_key="src",
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            )
                        ],
                        end_markers=[],
                    )
                ],
                wrap_end=[],
                machine=MachineParams(),
            )
        ),
    )
    enc = _encode_node(
        "enc",
        "agg",
        Encoder(
            GcodeSpec(
                dialect=GcodeDialectSpec(),
                context_json="{}",
            )
        ),
    )
    completed, _ = collect_completions([src, agg, enc])
    c = _by_key(completed)["enc"]
    assert c.error is None
    assert encode_result(c).variant == "MachineCode"


# ── Scene encoder ─────────────────────────────────────────────────


def _scene_spec():
    return SceneSpec(
        np.eye(4, dtype=np.float32).tolist(),
        {},
    )


def test_scene_encode_succeeds():
    src = _compute_src()
    enc = _encode_node("enc", "src", Encoder(_scene_spec()))
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    assert out.variant == "Scene"
    assert out.repr is not None
    assert out.text is None
    assert out.power_texture is None


def test_scene_encode_repr_contains_groups():
    src = _compute_src()
    enc = _encode_node("enc", "src", Encoder(_scene_spec()))
    completed, _ = collect_completions([src, enc])
    out = encode_result(_by_key(completed)["enc"])
    assert out.repr is not None
    assert "groups=" in out.repr
    assert "layers=" in out.repr


# ── Error cases ───────────────────────────────────────────────────


def test_encode_with_missing_source_yields_error():
    enc = _encode_node("enc", "ghost-src", Encoder(VertexSpec()))
    completed, _ = collect_completions([enc])
    c = _by_key(completed)["enc"]
    assert c.error is not None
    assert "ghost-src" in c.error
    assert c.output is None


def test_encode_with_unknown_encoder_type_errors_at_construction():
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src",
            encoder=Encoder(0),  # wrong type
        ),
    )
    src = _compute_src()
    with pytest.raises(TypeError):
        execute_stages([src, enc], lambda n: None, None)
