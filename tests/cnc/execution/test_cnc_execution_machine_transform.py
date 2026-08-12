"""Tests for the MachineTransform pipeline stage.

Verifies that the world→machine matrix and WCS offset are applied in
the correct order: the WCS offset must be subtracted AFTER the w2m
transform (in machine space), not before (in world space).

Regression test for a bug where ``translate_layers`` subtracted the
WCS offset from world-space ops before the w2m transform was applied,
producing wrong G-code whenever the w2m matrix was not identity (e.g.
axis reversal or non-bottom-left origin).
"""

import pytest
from conftest import collect_completions, make_square_part

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    MachineParams,
    MachineTransformSpec,
    Marker,
)
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]

# Sign-flip matrix (axis reversal).
SIGN_FLIP_XY = [
    [-1.0, 0.0, 0.0, 0.0],
    [0.0, -1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]

# Top-right origin with reversal: world (x,y) → machine (x-100, y-100).
TR_REVERSED = [
    [1.0, 0.0, 0.0, -100.0],
    [0.0, 1.0, 0.0, -100.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _contour_node(key: str) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()),
            ),
        ),
    )


def _agg_node(key: str, source_keys: list[str]) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Aggregate(
            spec=AggregateSpec(
                wrap_start=[],
                groups=[
                    AggregateGroup(
                        start_markers=[],
                        inputs=[
                            AggregateInput(
                                source_key=sk,
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            )
                            for sk in source_keys
                        ],
                        end_markers=[],
                    )
                ],
                wrap_end=[],
                machine=MachineParams(),
            )
        ),
    )


def _transform_node(
    key: str,
    source_key: str,
    w2m=None,
    default_wcs=None,
    layer_wcs=None,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=MachineTransformSpec(
            source_key=source_key,
            linearize_curves=False,
            world_to_machine=w2m or IDENTITY,
            default_wcs_offset=default_wcs or [0.0, 0.0, 0.0],
            layer_wcs_offsets=layer_wcs or [],
            reverse_z=False,
            rotary_mappings=[],
        ),
    )


def _run_transform(nodes) -> dict:
    completed, _ = collect_completions(nodes)
    return {c.key: c for c in completed}


def _first_point(node: CompletedNode) -> tuple[float, float]:
    """Extract the first MOVE_TO point from the transformed ops."""
    assert node.output is not None
    ops = node.output.ops
    d = ops.to_dict()
    for cmd in d.get("commands", []):
        if cmd.get("type") == "MOVE_TO":
            end = cmd["end"]
            return (end[0], end[1])
    pytest.fail("No MOVE_TO found in transformed ops")


# ── No WCS offset: w2m is applied correctly ───────────────────────


def test_identity_transform_preserves_coords():
    """With identity w2m and no WCS, coords are unchanged."""
    nodes = [
        _contour_node("src"),
        _agg_node("agg", ["src"]),
        _transform_node("xform", "agg"),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    assert pt == pytest.approx((0.0, 0.0), abs=1e-6)


def test_sign_flip_negates_coords():
    """Sign-flip w2m negates both axes. World (0,0) → (0,0);
    world (10,0) → (-10, 0)."""
    nodes = [
        _contour_node("src"),
        _agg_node("agg", ["src"]),
        _transform_node("xform", "agg", w2m=SIGN_FLIP_XY),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    assert pt == pytest.approx((0.0, 0.0), abs=1e-6)


def test_tr_reversed_shifts_coords():
    """TR+reversed w2m maps world (0,0) to machine (-100,-100)."""
    nodes = [
        _contour_node("src"),
        _agg_node("agg", ["src"]),
        _transform_node("xform", "agg", w2m=TR_REVERSED),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    assert pt == pytest.approx((-100.0, -100.0), abs=1e-6)


# ── WCS offset with identity w2m (always worked) ──────────────────


def test_wcs_offset_subtracted_with_identity_w2m():
    """With identity w2m, WCS offset is subtracted from machine coords.
    World (0,0) → machine (0,0) → gcode (0,0) - (10,20) = (-10,-20)."""
    nodes = [
        _contour_node("src"),
        _agg_node("agg", ["src"]),
        _transform_node(
            "xform",
            "agg",
            default_wcs=[10.0, 20.0, 0.0],
        ),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    assert pt == pytest.approx((-10.0, -20.0), abs=1e-6)


# ── WCS offset with non-identity w2m (the regression) ─────────────


def test_wcs_offset_with_sign_flip():
    """WCS offset must be subtracted AFTER w2m, not before.

    Sign-flip w2m: world (0,0) → machine (0,0).
    WCS = (10, 20) in machine space.
    Correct gcode = (0,0) - (10,20) = (-10, -20).

    The old bug subtracted WCS before w2m:
    (0-10, 0-20) → sign_flip → (10, 20), which was wrong.
    """
    nodes = [
        _contour_node("src"),
        _agg_node("agg", ["src"]),
        _transform_node(
            "xform",
            "agg",
            w2m=SIGN_FLIP_XY,
            default_wcs=[10.0, 20.0, 0.0],
        ),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    assert pt == pytest.approx((-10.0, -20.0), abs=1e-6)


def test_wcs_offset_with_tr_reversed():
    """WCS offset with TR+reversed w2m.

    w2m: world (0,0) → machine (-100,-100).
    WCS = (10, 20) in machine space.
    Correct gcode = (-100,-100) - (10,20) = (-110, -120).
    """
    nodes = [
        _contour_node("src"),
        _agg_node("agg", ["src"]),
        _transform_node(
            "xform",
            "agg",
            w2m=TR_REVERSED,
            default_wcs=[10.0, 20.0, 0.0],
        ),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    assert pt == pytest.approx((-110.0, -120.0), abs=1e-6)


# ── Per-layer WCS offsets ─────────────────────────────────────────


def test_per_layer_wcs_with_sign_flip():
    """Per-layer WCS offsets must also be subtracted after w2m.

    This is the primary regression test: the old code called
    translate_layers BEFORE applying w2m, subtracting WCS offsets from
    world-space coordinates. With a sign-flip w2m, this produced
    wrong results.
    """
    nodes = [
        _contour_node("src"),
        NodeRequest(
            key="agg",
            generation_id=1,
            stage=StageSpec.Aggregate(
                spec=AggregateSpec(
                    wrap_start=[],
                    groups=[
                        AggregateGroup(
                            start_markers=[
                                Marker.LayerStart(uid="layer-1", _tag=True)
                            ],
                            inputs=[
                                AggregateInput(
                                    source_key="src",
                                    placement_matrix=IDENTITY,
                                    uid="",
                                    target_dimensions=(0.0, 0.0),
                                )
                            ],
                            end_markers=[
                                Marker.LayerEnd(uid="layer-1", _tag=True)
                            ],
                        )
                    ],
                    wrap_end=[],
                    machine=MachineParams(),
                )
            ),
        ),
        _transform_node(
            "xform",
            "agg",
            w2m=SIGN_FLIP_XY,
            default_wcs=[0.0, 0.0, 0.0],
            layer_wcs=[
                ("layer-1", [10.0, 20.0, 0.0]),
            ],
        ),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    # Correct: sign_flip(0,0) - (10,20) = (0,0)-(10,20) = (-10,-20)
    assert pt == pytest.approx((-10.0, -20.0), abs=1e-6)


def test_per_layer_wcs_different_offsets():
    """Multiple layers with different WCS offsets all get the correct
    post-w2m subtraction."""
    nodes = [
        _contour_node("src"),
        NodeRequest(
            key="agg",
            generation_id=1,
            stage=StageSpec.Aggregate(
                spec=AggregateSpec(
                    wrap_start=[],
                    groups=[
                        AggregateGroup(
                            start_markers=[
                                Marker.LayerStart(uid="L1", _tag=True)
                            ],
                            inputs=[
                                AggregateInput(
                                    source_key="src",
                                    placement_matrix=IDENTITY,
                                    uid="",
                                    target_dimensions=(0.0, 0.0),
                                )
                            ],
                            end_markers=[Marker.LayerEnd(uid="L1", _tag=True)],
                        )
                    ],
                    wrap_end=[],
                    machine=MachineParams(),
                )
            ),
        ),
        _transform_node(
            "xform",
            "agg",
            w2m=SIGN_FLIP_XY,
            default_wcs=[0.0, 0.0, 0.0],
            layer_wcs=[
                ("L1", [-5.0, 15.0, 0.0]),
            ],
        ),
    ]
    results = _run_transform(nodes)
    pt = _first_point(results["xform"])
    # Correct: sign_flip(0,0) - (-5,15) = (0,0)-(-5,15) = (5,-15)
    assert pt == pytest.approx((5.0, -15.0), abs=1e-6)
