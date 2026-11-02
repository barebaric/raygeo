"""Tests for the region-splitting path in
``AdaptiveClearingSpec::assemble``.

These tests exercise the new region-processing logic that splits a
pocket into disconnected wide sub-regions via ``find_regions`` and
clears each one independently, recovering per-region failures as
``AssemblyWarning { kind = RegionFailed }`` instead of aborting the
whole face.

They drive the compute-stage pipeline path (``Assembler`` +
``AssemblerCompute``), the only entry point that goes through
``AdaptiveClearingSpec::assemble`` rather than the ``adaptive_clearing``
Python helper that calls the inner Rust function directly.
"""

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.ops.assembly import Assembler, AssemblyWarningKind
from raygeo.ops.assembly.adaptive import AdaptiveClearingSpec
from raygeo.ops.feature.region import find_regions
from raygeo.ops.part import Part
from raygeo.pipeline.execute import clear_cache, execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _dumbbell(lobe=40.0, neck=6.0, span=60.0):
    """Outer loop of two ``lobe``x``lobe`` squares joined by a
    ``neck``-wide corridor of length ``span`` along the x-axis.

    The corridor runs along ``y=0`` and connects the inner walls of
    the two lobes; the returned list is a single closed CCW loop
    suitable for ``Part.from_polygons`` and ``find_regions``.
    """
    half = lobe / 2.0
    lx = -span / 2.0
    rx = span / 2.0
    ny = neck / 2.0
    sy = -neck / 2.0
    return [
        (lx - half, -half),
        (rx + half, -half),
        (rx + half, sy),
        (lx - half, sy),
        (lx, sy),
        (lx, ny),
        (rx, ny),
        (rx + half, ny),
        (rx + half, half),
        (lx - half, half),
        (lx - half, ny),
        (rx, ny),
        (rx, sy),
        (lx - half, sy),
    ]


def _square(cx, cy, w, h=None):
    h = h if h is not None else w
    return [
        (cx - w / 2.0, cy - h / 2.0),
        (cx + w / 2.0, cy - h / 2.0),
        (cx + w / 2.0, cy + h / 2.0),
        (cx - w / 2.0, cy + h / 2.0),
    ]


def _run(part, tool_radius=3.0, step_over=1.5):
    """Run an adaptive-clearing compute stage on ``part`` and return
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
                    AdaptiveClearingSpec(
                        tool_radius=tool_radius,
                        step_over=step_over,
                        target_z=-5.0,
                        area_tolerance=1.0,
                    )
                )
            ),
        ),
    )
    completed = []
    execute_stages([node], completed.append, None)
    assert len(completed) == 1, completed
    return completed[0]


def _cut_endpoints(node):
    """Yield ``(x, y, z)`` cut-move endpoints from the completed
    node.  ``Ops.endpoint`` returns a 3-tuple.
    """
    ops = node.output.ops
    assert ops.len() > 0
    for i in range(ops.len()):
        if ops.is_cutting(i):
            yield ops.endpoint(i)


def test_dumbbell_splits_into_two_regions_both_cleared():
    """A dumbbell whose neck is too narrow for the tool is split into
    two regions by ``find_regions``; both lobes clear and no
    ``RegionFailed`` warning is emitted.  Conservative params keep
    the stepper from stalling inside either lobe.
    """
    loop = _dumbbell(lobe=40.0, neck=6.0, span=60.0)
    regions = find_regions(loop, [], 5.0, 1.0)
    assert len(regions) == 2, (
        f"expected the dumbbell to split into 2 regions, got {len(regions)}"
    )

    part = Part.from_polygons(loop, size_mm=(160.0, 50.0))
    node = _run(part, tool_radius=3.0, step_over=1.5)

    assert node.error is None, node.error
    assert node.output is not None
    assert node.output.ops.len() > 0

    region_failures = [
        w
        for w in node.output.warnings
        if w.kind == AssemblyWarningKind.REGION_FAILED
    ]
    assert not region_failures, [w.detail for w in region_failures]

    cutoff = 0.0
    left_cuts = any(x < cutoff for (x, y, z) in _cut_endpoints(node))
    right_cuts = any(x >= cutoff for (x, y, z) in _cut_endpoints(node))
    assert left_cuts, "no cutting moves landed in the left lobe"
    assert right_cuts, "no cutting moves landed in the right lobe"


def test_single_wide_pocket_is_one_region_and_clears():
    """A single wide square pocket has no separating passages, so
    ``find_regions`` returns the whole pocket as one wide region and the
    assembler clears it in a single pass, with no warnings.  Guards
    against the new path silently breaking the legacy single-region
    behaviour.
    """
    boundary = _square(0.0, 0.0, 60.0, 60.0)
    regions = find_regions(boundary, [], 3.0, 1.0)
    assert len(regions) == 1, len(regions)
    assert abs(regions[0][1] - 3600.0) < 1.0, regions[0][1]

    part = Part.from_polygons(boundary, size_mm=(60.0, 60.0))
    node = _run(part, tool_radius=3.0, step_over=1.5)

    assert node.error is None, node.error
    assert node.output is not None
    assert node.output.ops.len() > 0
    region_failures = [
        w
        for w in node.output.warnings
        if w.kind == AssemblyWarningKind.REGION_FAILED
    ]
    assert not region_failures, [w.detail for w in region_failures]


def test_island_inside_one_region_is_attributed_to_that_region():
    """An island whose centroid lies inside a single detected region
    is filtered into that region only.  After clearing under
    conservative params, both lobes still produce cutting moves and
    no ``RegionFailed`` warning is emitted.
    """
    loop = _dumbbell(lobe=40.0, neck=6.0, span=60.0)

    island_w = 8.0
    island_cx = -60.0
    island_cy = 0.0
    island = _square(island_cx, island_cy, island_w)

    regions = find_regions(loop, [island], 5.0, 1.0)
    assert len(regions) == 2, len(regions)

    part = Part.from_polygons(loop, islands=[island], size_mm=(160.0, 50.0))
    face = part.face("")
    assert face is not None
    assert len(face.stock_region.islands) == 1

    node = _run(part, tool_radius=3.0, step_over=1.5)
    assert node.error is None, node.error
    assert node.output is not None
    assert node.output.ops.len() > 0

    region_failures = [
        w
        for w in node.output.warnings
        if w.kind == AssemblyWarningKind.REGION_FAILED
    ]
    assert not region_failures, [w.detail for w in region_failures]

    cutoff = 0.0
    left_cuts = any(x < cutoff for (x, y, z) in _cut_endpoints(node))
    right_cuts = any(x >= cutoff for (x, y, z) in _cut_endpoints(node))
    assert left_cuts, "no cutting moves landed near the islanded lobe"
    assert right_cuts, "no cutting moves landed in the bare lobe"


def test_aggressive_params_clear_the_wide_regions_without_stalling():
    """An aggressively-parametrised clear over the small-lobe dumbbell
    used to stall the stepper inside each wide region: the wavefront
    stopped on a thin uncleared core, no resume strategy found a
    proposal, and the region was recovered as a ``RegionFailed`` warning
    with the lobe centres left standing.  The core resume strategy
    proposes the spine of that uncleared remainder, so the same params
    now clear both wide regions completely — no warning, no stranded
    core.  (With this tool the narrow neck is a barrier: the tool cuts
    the two bands it separates, as designed.)
    """
    loop = _dumbbell(lobe=40.0, neck=6.0, span=60.0)
    part = Part.from_polygons(loop, size_mm=(160.0, 50.0))

    node = _run(part, tool_radius=5.0, step_over=2.0)

    assert node.error is None, node.error
    assert node.output is not None
    assert node.output.ops.len() > 0

    region_failures = [
        w
        for w in node.output.warnings
        if w.kind == AssemblyWarningKind.REGION_FAILED
    ]
    assert not region_failures, [w.detail for w in region_failures]

    cuts = list(_cut_endpoints(node))
    top = [p for p in cuts if p[1] > 3.0]
    bottom = [p for p in cuts if p[1] < -3.0]
    assert top, "the upper wide region was never machined"
    assert bottom, "the lower wide region was never machined"

    # The wavefront must not strand a core: every point a tool-radius
    # inside the upper band is within a tool radius of a cut.  The band
    # spans y in [3, 20], so tool centres legitimately fill y in
    # [8, 15]; probe that whole reachable strip.
    for y0, y1 in ((8.5, 14.5), (-14.5, -8.5)):
        x = -44.0
        while x <= 44.0:
            y = y0
            while y <= y1:
                d = min((p[0] - x) ** 2 + (p[1] - y) ** 2 for p in cuts) ** 0.5
                assert d <= 5.0 - 0.02, (
                    f"point ({x}, {y}) is {d:.3f} from the nearest cut"
                )
                y += 0.5
            x += 0.5
