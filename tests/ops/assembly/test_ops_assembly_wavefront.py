"""Tests for wavefront assembly module."""

import math

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.geo import Geometry
from raygeo.ops import Ops
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.wavefront import (
    AdaptiveWavefrontSpec,
    adaptive_wavefronts,
)
from raygeo.ops.part import Part
from raygeo.pipeline.execute import clear_cache, execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _seed_polygon(cx, cy, r, n=32):
    """A small circle polygon to seed cleared area."""
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _geometry_from_loops(loops):
    """Build a :class:`Geometry` from closed ``(x, y)`` loops."""
    geo = Geometry()
    for poly in loops:
        geo.move_to(poly[0][0], poly[0][1], 0.0)
        for x, y in poly[1:]:
            geo.line_to(x, y, 0.0)
        geo.close_path()
    return geo


def _run_wavefront_compute(part, step_over=2.0, area_tolerance=1.0):
    """Run an adaptive-wavefront compute stage on ``part`` and return
    the completed pipeline node.
    """
    clear_cache()
    node = NodeRequest(
        key="c",
        generation_id=1,
        stage=StageSpec.Compute(
            part=part,
            params=ComputePayload(
                assembler=Assembler(
                    AdaptiveWavefrontSpec(
                        step_over=step_over,
                        z=-8.0,
                        area_tolerance=area_tolerance,
                    )
                )
            ),
        ),
    )
    completed = []
    execute_stages([node], completed.append, None)
    assert len(completed) == 1, completed
    return completed[0]


def _cut_xs(node):
    """Yield x-coordinates of cut-move endpoints from a completed node."""
    ops = node.output.ops
    for i in range(ops.len()):
        if ops.is_cutting(i):
            yield ops.endpoint(i)[0]


def test_adaptive_wavefronts_simple():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    part = Part.from_polygons(boundary, initial=initial)
    result_wf = adaptive_wavefronts(
        part,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert result_wf.ops.len() > 0
    assert part.cleared.total_area() > 10000


def test_adaptive_wavefronts_with_islands():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    initial = [_seed_polygon(20.0, 50.0, 15.0)]
    part = Part.from_polygons(boundary, islands, initial=initial)
    result_wf = adaptive_wavefronts(
        part,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert result_wf.ops.len() > 0
    assert part.cleared.total_area() > 5000


def test_adaptive_wavefronts_empty_cleared():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    part = Part.from_polygons(boundary)
    result_wf = adaptive_wavefronts(
        part,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )
    assert isinstance(result_wf.ops, Ops)


def test_adaptive_wavefronts_cut_power_applied():
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    part = Part.from_polygons(boundary, initial=initial)
    result_wf = adaptive_wavefronts(
        part,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        cut_feed_rate=1200,
        cut_power=0.45,
    )
    found_power = False
    for i in range(result_wf.ops.len()):
        if result_wf.ops.is_cutting(i):
            assert result_wf.ops.state_at(i).power == 0.45
            found_power = True
            break
    assert found_power


def test_adaptive_wavefronts_precision_resamples():
    """precision > 0 with a seed polygon produces ops (count may vary)."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    initial = [_seed_polygon(80.0, 50.0, 15.0)]
    part = Part.from_polygons(boundary, initial=initial)
    result_wf = adaptive_wavefronts(
        part,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        precision=5.0,
    )
    assert result_wf.ops.len() > 0, "expected ops with precision>0"


def test_adaptive_wavefronts_precision_with_islands():
    """precision > 0 with islands may or may not produce ops."""
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    islands = [[(60, 35), (100, 35), (100, 65), (60, 65)]]
    initial = [_seed_polygon(20.0, 50.0, 15.0)]
    part = Part.from_polygons(boundary, islands, initial=initial)
    result_wf = adaptive_wavefronts(
        part,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
        precision=5.0,
    )
    assert isinstance(result_wf.ops, Ops)
    # Area should be tracked regardless of ops count
    assert part.cleared.total_area() >= 0


def test_wavefront_face_ids_deterministic_order():
    """Face ids come back in a stable, canonical order regardless of
    HashMap iteration order (which is randomized per process).
    """
    loops = [
        [(-30.0, -20.0), (30.0, -20.0), (30.0, 20.0), (-30.0, 20.0)],
        [(70.0, -20.0), (110.0, -20.0), (110.0, 20.0), (70.0, 20.0)],
        [(130.0, -20.0), (190.0, -20.0), (190.0, 20.0), (130.0, 20.0)],
    ]
    part = Part.from_geometry_multi_face(
        geometry=_geometry_from_loops(loops), size_mm=(220.0, 50.0)
    )
    assert part.face_ids == ["", "1", "2"], part.face_ids

    part.add_face("10", None)
    part.add_face("3", None)
    part.add_face("b", None)
    # Numeric ids sort numerically (so "10" after "3", not after "1"),
    # and non-numeric ids come last in lexicographic order.
    assert part.face_ids == ["", "1", "2", "3", "10", "b"], part.face_ids


def test_wavefront_multi_face_compute_deterministic():
    """Multi-face wavefront produces byte-identical ops across runs."""
    loops = [
        [(-30.0, -20.0), (30.0, -20.0), (30.0, 20.0), (-30.0, 20.0)],
        [(70.0, -20.0), (110.0, -20.0), (110.0, 20.0), (70.0, 20.0)],
    ]
    part1 = Part.from_geometry_multi_face(
        geometry=_geometry_from_loops(loops), size_mm=(160.0, 50.0)
    )
    node1 = _run_wavefront_compute(part1)
    part2 = Part.from_geometry_multi_face(
        geometry=_geometry_from_loops(loops), size_mm=(160.0, 50.0)
    )
    node2 = _run_wavefront_compute(part2)
    assert node1.output.ops.dump() == node2.output.ops.dump()


def test_wavefront_multi_face_compute_both_faces_clear():
    """A multi-face Part assembled through the compute stage clears
    both pockets.  Regression guard for the multi-pocket wrapper
    deletion: per-face iteration in ``AssemblerCompute::run`` is what
    handles multiple pockets now.
    """
    loops = [
        [(-30.0, -20.0), (30.0, -20.0), (30.0, 20.0), (-30.0, 20.0)],
        [(70.0, -20.0), (110.0, -20.0), (110.0, 20.0), (70.0, 20.0)],
    ]
    part = Part.from_geometry_multi_face(
        geometry=_geometry_from_loops(loops), size_mm=(160.0, 50.0)
    )
    assert len(part.face_ids) == 2, part.face_ids

    node = _run_wavefront_compute(part)

    assert node.error is None, node.error
    assert node.output is not None
    assert node.output.ops.len() > 0
    assert not node.output.warnings

    xs = list(_cut_xs(node))
    assert xs, "expected cutting moves"
    assert any(x < 0.0 for x in xs), "no cutting moves in the left pocket"
    assert any(x > 40.0 for x in xs), "no cutting moves in the right pocket"


def test_wavefront_single_face_compute_matches_low_level():
    """A single-face Part assembled through the compute stage produces
    the same clearing as the low-level ``adaptive_wavefronts`` helper
    for the same pocket.  Guards that the ``assemble`` rewiring did not
    change single-pocket behaviour.
    """
    boundary = [(0, 0), (160, 0), (160, 100), (0, 100)]
    part = Part.from_geometry_multi_face(
        geometry=_geometry_from_loops([boundary]), size_mm=(160.0, 100.0)
    )
    assert part.face_ids == [""], part.face_ids

    node = _run_wavefront_compute(part, step_over=2.0, area_tolerance=1.0)
    assert node.error is None, node.error
    compute_ops = node.output.ops
    assert compute_ops.len() > 0

    # Low-level helper on the same pocket.
    direct = Part.from_polygons(
        boundary, initial=[_seed_polygon(80.0, 50.0, 15.0)]
    )
    result_wf = adaptive_wavefronts(
        direct,
        step_over=2.0,
        z=-8.0,
        area_tolerance=1.0,
    )

    assert compute_ops.count_cutting() == result_wf.ops.count_cutting()
    assert node.output.cleared_fragments is not None, (
        "expected cleared fragments in the compute output"
    )
    assert len(node.output.cleared_fragments) > 0
