"""A single intent tree whose leaves run every assembler we have.

The tree has the shape:

    leaf-0 ─┐
    leaf-1 ─┤
    ...     ├─► agg ──► enc (gcode)
    leaf-N ─┘

Each leaf uses a *different* assembler. The aggregate concatenates
their ops, and the encode node turns the result into G-code. This
exercises the dispatcher's ability to:

- drive every Assembler subclass through ``Box<dyn Assembler>``;
- fan-in many compute leaves into one aggregate;
- run an encoder over the merged ops.

Heavy assemblers (adaptive clearing, profile) are intentionally
excluded here so the suite stays fast; they have their own
dedicated files.
"""

import numpy as np
from conftest import (
    collect_completions,
    encode_result,
    make_square_part,
    result_ops,
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
from raygeo.ops.assembly.frame import FrameSpec
from raygeo.ops.assembly.helix import HelixSpec
from raygeo.ops.assembly.material_test_grid import MaterialTestGridSpec
from raygeo.ops.assembly.ramp import RampSpec
from raygeo.ops.assembly.raster import RasterSpec
from raygeo.ops.assembly.shrinkwrap import ShrinkwrapSpec
from raygeo.ops.assembly.slot import SlotSpec
from raygeo.ops.assembly.spiral import SpiralSpec
from raygeo.ops.assembly.toroid import ToroidSpec
from raygeo.ops.convert import Encoder, GcodeDialectSpec, GcodeSpec
from raygeo.ops.part import Part
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _filled_part(fill: int = 255, size_mm=(10.0, 10.0), ppm=(10.0, 10.0)):
    """Build a small filled-image Part for raster-like assemblers.

    A fresh Part is returned on every call because the pipeline
    converter *steals* the underlying core Part out of the Python
    wrapper on first dispatch — reusing the same instance leaves an
    empty Part behind for the next compute node.
    """
    part = Part(size_mm=size_mm, pixels_per_mm=ppm)
    w = int(size_mm[0] * ppm[0])
    h = int(size_mm[1] * ppm[1])
    part.image = np.full((h, w), fill, dtype=np.uint8)
    return part


def _polygons_part():
    """A Part with a single rectangular stock polygon for adaptive
    assemblers that need real geometry.

    A fresh Part is returned on every call (see ``_filled_part``).
    """
    boundary = [
        (-20.0, -20.0),
        (20.0, -20.0),
        (20.0, 20.0),
        (-20.0, 20.0),
    ]
    return Part.from_polygons(boundary)


def _leaf(key: str, assembler: Assembler, part: Part) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=part,
            params=ComputePayload(assembler=assembler),
        ),
    )


def _aggregate(key: str, source_keys: list[str]) -> NodeRequest:
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


def _encode(key: str, source_key: str) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=EncodeSpec(
            source_key=source_key,
            encoder=Encoder(
                GcodeSpec(dialect=GcodeDialectSpec(), context_json="{}")
            ),
        ),
    )


def _build_leaves() -> list[NodeRequest]:
    # Each leaf gets its OWN fresh Part instance: the converter
    # empties the Python-side Part on dispatch, so an instance may
    # only be used by a single compute node.
    return [
        _leaf("contour", Assembler(ContourSpec()), make_square_part()),
        _leaf("frame", Assembler(FrameSpec()), make_square_part()),
        _leaf(
            "helix",
            Assembler(
                HelixSpec(
                    center=(0, 0),
                    start_radius=2.0,
                    z_start=1.0,
                    z_end=-3.0,
                    pitch=1.0,
                )
            ),
            _polygons_part(),
        ),
        _leaf(
            "ramp",
            Assembler(
                RampSpec(start=(0, 0), end=(10, 0), z_start=1.0, z_end=-2.0)
            ),
            _polygons_part(),
        ),
        _leaf(
            "spiral",
            Assembler(
                SpiralSpec(
                    center=(0, 0),
                    z=-1.0,
                    start_radius=0.5,
                    end_radius=3.0,
                    revolutions=2,
                    direction="CW",
                    angular_step=0.5,
                    start_angle=0.0,
                )
            ),
            _polygons_part(),
        ),
        _leaf(
            "slot",
            Assembler(
                SlotSpec(
                    carrier=[(0, 0), (20, 0)], tool_radius=1.5, target_z=-2.0
                )
            ),
            _polygons_part(),
        ),
        _leaf(
            "toroid",
            Assembler(
                ToroidSpec(
                    carrier=[(0, 0), (40, 0)],
                    tool_radius=2.0,
                    step_over=1.0,
                    target_z=-3.0,
                )
            ),
            _polygons_part(),
        ),
        _leaf(
            "material_test_grid",
            Assembler(
                MaterialTestGridSpec(size_mm=(50.0, 50.0), cols=2, rows=2)
            ),
            _polygons_part(),
        ),
        _leaf(
            "shrinkwrap",
            Assembler(ShrinkwrapSpec(gravity=0.0, cut_side="outer")),
            _filled_part(),
        ),
        _leaf(
            "raster",
            Assembler(RasterSpec(mode="mask_scan", line_interval_mm=1.0)),
            _filled_part(),
        ),
    ]


def _by_key(completed):
    return {c.key: c for c in completed}


# ── End-to-end topology ────────────────────────────────────────────


def test_multi_assembler_intent_completes_all_leaves():
    leaves = _build_leaves()
    leaf_keys = [leaf.key for leaf in leaves]
    agg = _aggregate("agg", leaf_keys)
    enc = _encode("enc", "agg")
    completed, _ = collect_completions(leaves + [agg, enc])
    by_key = _by_key(completed)
    for k in leaf_keys + ["agg", "enc"]:
        assert k in by_key, f"missing completion for {k}"
        assert by_key[k].error is None, f"{k} failed: {by_key[k].error}"


def test_each_leaf_produces_ops():
    leaves = _build_leaves()
    completed, _ = collect_completions(leaves)
    by_key = _by_key(completed)
    for leaf in leaves:
        out = by_key[leaf.key].output
        assert out is not None, f"{leaf.key} produced no output"
        assert len(out.ops) > 0, f"{leaf.key} produced empty ops"


def test_aggregate_concatenates_all_leaf_ops():
    leaves = _build_leaves()
    leaf_keys = [leaf.key for leaf in leaves]
    agg = _aggregate("agg", leaf_keys)
    completed, _ = collect_completions(leaves + [agg])
    by_key = _by_key(completed)

    leaf_total = sum(len(result_ops(by_key[k])) for k in leaf_keys)
    agg_ops = result_ops(by_key["agg"])
    assert len(agg_ops) == leaf_total


def test_encode_node_produces_machine_code():
    leaves = _build_leaves()
    leaf_keys = [leaf.key for leaf in leaves]
    agg = _aggregate("agg", leaf_keys)
    enc = _encode("enc", "agg")
    completed, _ = collect_completions(leaves + [agg, enc])
    by_key = _by_key(completed)
    out = encode_result(by_key["enc"])
    assert out.variant == "MachineCode"
    assert out.text is not None
    assert len(out.text) > 0


def test_is_scalable_flag_per_leaf():
    """Vector assemblers return True; raster/shrinkwrap return False."""
    leaves = _build_leaves()
    completed, _ = collect_completions(leaves)
    by_key = _by_key(completed)
    non_scalable = {"raster", "shrinkwrap"}
    for leaf in leaves:
        out = by_key[leaf.key].output
        assert out is not None, f"{leaf.key} produced no output"
        if leaf.key in non_scalable:
            assert out.is_scalable is False, (
                f"{leaf.key} should NOT be scalable"
            )
        else:
            assert out.is_scalable is True, f"{leaf.key} should be scalable"


def test_source_dimensions_echoed_for_square_parts():
    """Square geometry leaves carry their original size_mm."""
    leaves = _build_leaves()
    completed, _ = collect_completions(leaves)
    by_key = _by_key(completed)
    square_leaves = {"contour", "frame"}
    for key in square_leaves:
        out = by_key[key].output
        assert out is not None
        assert out.source_dimensions == (10.0, 10.0)
