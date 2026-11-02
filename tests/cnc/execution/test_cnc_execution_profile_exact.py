"""Tests for profile passes over curve-true geometry.

Verifies that a ``ProfileSpec`` over a part built from a ``Geometry``
follows the exact line-and-arc offset:

* arcs are emitted as ``ARC_TO`` moves (parts without curve-true
  geometry take the polygon path and only emit lines),
* the tool stays on the exact offset — a circle of radius R profiled
  with tool radius r cuts on a circle of radius R − r,
* ``cut_direction`` is honoured (the arc rotation flips with the
  direction),
* islands are honoured, and
* the exact attempt falls back to the polygon path when it yields
  nothing machinable. Region-scoped steps always take the polygon
  path (scopes are polygonal); the clearing workplan exercises that
  combination in its own suite.
"""

import math

import pytest
from conftest import collect_completions, compute_result

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.geo import Geometry
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.profile import ProfileSpec, profile_inner
from raygeo.ops.part import Part
from raygeo.pipeline.execute import clear_cache
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _circle_geometry(r, cx=0.0, cy=0.0):
    g = Geometry()
    g.move_to(cx + r, cy, 0)
    g.arc_to(cx - r, cy, -r, 0, False, 0)
    g.arc_to(cx + r, cy, r, 0, False, 0)
    return g


def _rounded_rect_geometry(x0, y0, x1, y1, r):
    g = Geometry()
    g.move_to(x0 + r, y0, 0)
    g.line_to(x1 - r, y0, 0)
    g.arc_to(x1, y0 + r, 0, r, False, 0)
    g.line_to(x1, y1 - r, 0)
    g.arc_to(x1 - r, y1, -r, 0, False, 0)
    g.line_to(x0 + r, y1, 0)
    g.arc_to(x0, y1 - r, 0, -r, False, 0)
    g.line_to(x0, y0 + r, 0)
    g.arc_to(x0 + r, y0, r, 0, False, 0)
    return g


def _stadium_geometry(cx, cy, half_len, r):
    """A stadium (slot shape) drawn clockwise, so it bounds an island."""
    g = Geometry()
    g.move_to(cx - half_len, cy + r, 0)
    g.line_to(cx + half_len, cy + r, 0)
    g.arc_to(cx + half_len, cy - r, 0, -r, True, 0)
    g.line_to(cx - half_len, cy - r, 0)
    g.arc_to(cx - half_len, cy + r, 0, r, True, 0)
    g.close_path()
    return g


def _profile_ops(part, **spec_kwargs):
    spec = ProfileSpec(tool_radius=3.0, **spec_kwargs)
    request = NodeRequest(
        key="k",
        generation_id=1,
        stage=StageSpec.Compute(
            part=part,
            params=ComputePayload(assembler=Assembler(spec)),
            face_id="",
        ),
    )
    clear_cache()
    completed, _ = collect_completions([request])
    clear_cache()
    [node] = completed
    assert node.error is None, node.error
    return compute_result(node).ops.to_dict()["commands"]


@pytest.fixture(autouse=True)
def _clear_cache():
    clear_cache()
    yield
    clear_cache()


def test_inner_profile_emits_arcs_on_a_circle():
    part = Part(geometry=_circle_geometry(10.0), size_mm=(30.0, 30.0))
    cmds = _profile_ops(part, kind="inner")
    kinds = [c["type"] for c in cmds]
    assert "ARC_TO" in kinds, f"no arcs emitted: {sorted(set(kinds))}"
    # The tool centre rides the exact offset: a circle of radius 7.
    checked = 0
    for c in cmds:
        if c["type"] not in ("LINE_TO", "ARC_TO"):
            continue
        x, y = c["end"][0], c["end"][1]
        radius = math.hypot(x, y)
        assert radius == pytest.approx(7.0, abs=1e-6), (
            f"cut at radius {radius}"
        )
        checked += 1
    assert checked > 10, f"only {checked} cutting moves to check"


def test_polygon_only_parts_stay_line_only():
    part = Part.from_polygons(
        [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)],
        [],
        (60.0, 50.0),
    )
    cmds = _profile_ops(part, kind="inner")
    kinds = [c["type"] for c in cmds]
    assert "ARC_TO" not in kinds, "the polygon path should not emit arcs"
    assert kinds.count("LINE_TO") > 10


def test_cut_direction_flips_the_arc_rotation():
    ccw_arcs = [
        c
        for c in _profile_ops(
            Part(geometry=_circle_geometry(10.0), size_mm=(30.0, 30.0)),
            kind="inner",
        )
        if c["type"] == "ARC_TO"
    ]
    cw_arcs = [
        c
        for c in _profile_ops(
            Part(geometry=_circle_geometry(10.0), size_mm=(30.0, 30.0)),
            kind="inner",
            cut_direction="cw",
        )
        if c["type"] == "ARC_TO"
    ]
    assert ccw_arcs and cw_arcs
    # The same wall walked the other way round: rotation flips.
    assert all(not c["clockwise"] for c in ccw_arcs), [
        c["clockwise"] for c in ccw_arcs
    ]
    assert all(c["clockwise"] for c in cw_arcs), [
        c["clockwise"] for c in cw_arcs
    ]


def test_profile_honours_the_island():
    region = _rounded_rect_geometry(0, 0, 100, 70, 10.0)
    region.extend(_stadium_geometry(50, 35, 12.0, 8.0))
    part = Part(geometry=region, size_mm=(120.0, 90.0))
    cmds = _profile_ops(part, kind="inner")

    def clearance(x, y):
        # Distance to the outer wall (min side of the 100x70 rectangle; the
        # rounded corners only bring the wall closer, so this never hides a
        # violation at the corners for this exact pass) and to the island's
        # stadium (centred, half length 12, end radius 8).
        wall = min(x, y, 100.0 - x, 70.0 - y)
        ix = max(0.0, abs(x - 50.0) - 12.0)
        iy = max(0.0, abs(y - 35.0) - 8.0)
        island = math.hypot(ix, iy)
        return wall, island

    checked = 0
    for c in cmds:
        if c["type"] not in ("LINE_TO", "ARC_TO") or c["end"][2] >= 0.0:
            continue
        wall, island = clearance(c["end"][0], c["end"][1])
        assert wall >= 3.0 - 1e-6, f"wall clearance {wall} at {c['end']}"
        assert island >= 3.0 - 1e-6, f"island clearance {island} at {c['end']}"
        checked += 1
    assert checked > 50, f"only {checked} cutting moves to check"


def test_rough_pass_cuts_the_island_ring_instead_of_skipping_it():
    """A rough pass (stock to leave) around an island starts in open
    material, where the engagement check fires on every step. Travel
    skips must not deadlock there: after a few, the pass cuts the ring
    as planned."""
    region = _rounded_rect_geometry(0, 0, 100, 70, 10.0)
    region.extend(_stadium_geometry(50, 35, 12.0, 8.0))
    part = Part(geometry=region, size_mm=(120.0, 90.0))
    cmds = _profile_ops(part, kind="inner", stock_to_leave=0.5)

    ring_cuts = ring_travels = 0
    for c in cmds:
        if "end" not in c:
            continue
        if not (3.0 <= c["end"][0] <= 97.0 and -8.0 <= c["end"][1] <= 78.0):
            continue
        if c["type"] in ("LINE_TO", "ARC_TO") and c["end"][2] < 0.0:
            ring_cuts += 1
        elif c["type"] == "MOVE_TO":
            ring_travels += 1
    assert ring_cuts > 60, (
        f"the rough island ring was skipped, not cut: {ring_cuts} cuts"
    )
    assert ring_travels < ring_cuts // 2, (
        f"the rough island ring is mostly travel: {ring_travels} travels"
    )


def test_descent_runs_at_the_plunge_feed():
    """The descent into material is a feed move at the plunge feed, and
    cutting resumes at the cut feed afterwards."""
    part = Part(geometry=_circle_geometry(10.0), size_mm=(30.0, 30.0))
    result = profile_inner(
        part,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        cut_feed_rate=1000,
        cut_power=0.0,
        plunge_feed_rate=300,
    )
    cmds = result.ops.to_dict()["commands"]
    rates = [c["speed"] for c in cmds if c["type"] == "SET_FEED_RATE"]
    assert 300 in rates, f"plunge feed missing from {rates}"
    assert 1000 in rates, f"cut feed missing from {rates}"
    # The descent is a feed move straight down from the safe height to the
    # target depth, at the plunge feed, before any cutting arc.
    moving = [c for c in cmds if c["type"] in ("MOVE_TO", "LINE_TO", "ARC_TO")]
    descents = [
        (prev, c)
        for prev, c in zip(moving, moving[1:])
        if c["type"] == "LINE_TO"
        and c["end"][2] == -5.0
        and prev["end"][2] == 2.0
    ]
    assert descents, "no vertical feed descent found"
    first_descent_index = cmds.index(descents[0][1])
    before = [
        c["speed"]
        for c in cmds[:first_descent_index]
        if c["type"] == "SET_FEED_RATE"
    ]
    assert before[-1] == 300, f"descent not at plunge feed: {before}"
    after = [
        c["speed"]
        for c in cmds[first_descent_index:]
        if c["type"] == "SET_FEED_RATE"
    ]
    assert 1000 in after, f"cut feed not restored: {after}"


def test_outer_profile_grows_outward():
    part = Part(geometry=_circle_geometry(10.0), size_mm=(30.0, 30.0))
    cmds = _profile_ops(part, kind="outer")
    arcs = [c for c in cmds if c["type"] == "ARC_TO"]
    assert arcs, "no arcs emitted"
    for c in arcs:
        radius = math.hypot(c["end"][0], c["end"][1])
        assert radius == pytest.approx(13.0, abs=1e-6), (
            f"cut at radius {radius}"
        )


def test_pocket_too_small_for_the_tool_machines_nothing():
    """An inset that vanishes (r2 pocket, r3 tool) leaves no machinable
    loops: the pass must come out empty rather than cut into the wall."""
    part = Part(geometry=_circle_geometry(2.0), size_mm=(30.0, 30.0))
    cmds = _profile_ops(part, kind="inner")
    kinds = [c["type"] for c in cmds]
    assert "ARC_TO" not in kinds
    assert "LINE_TO" not in kinds
