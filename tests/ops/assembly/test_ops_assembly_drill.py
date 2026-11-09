"""Tests for the drill assembler (plain, peck, chip-break, countersink)."""

import math

import pytest

from raygeo.ops.assembly.drill import (
    PeckParams,
    generate_countersink,
    generate_drill,
)
from raygeo.ops.part import Part
from raygeo.ops.state import State


def _state(**kwargs):
    return State(feed_rate=1000, plunge_feed=300, spindle_rpm=4000, **kwargs)


def _commands(result):
    return result.ops.to_dict()["commands"]


def _moving(result):
    return [
        (c["type"], round(c["end"][2], 6))
        for c in _commands(result)
        if c["type"] in ("MOVE_TO", "LINE_TO")
    ]


def test_plain_drill_move_sequence():
    """Rapid to safe, rapid to the rapid plane, one plunge at the plunge
    feed, retract to safe."""
    result = generate_drill(
        Part.from_polygons([]),
        center=(10.0, 20.0),
        diameter=5.0,
        target_z=-10.0,
        state=_state(),
    )
    moves = _moving(result)
    tip = -10.0 - (2.5 / math.tan(math.radians(118.0) / 2.0))
    assert moves == [
        ("MOVE_TO", 5.0),  # safe_z
        ("MOVE_TO", 1.0),  # rapid_z
        ("LINE_TO", pytest.approx(tip)),  # tip-compensated depth
        ("MOVE_TO", 5.0),  # retract to safe
    ]


def test_cycle_wrapped_in_drill_cycle_sections():
    result = generate_drill(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-10.0,
    )
    types = [c["type"] for c in _commands(result)]
    assert types.count("OPS_SECTION_START") == 1
    assert types.count("OPS_SECTION_END") == 1
    first = _commands(result)[types.index("OPS_SECTION_START")]
    assert first["section_type"] == "DRILL_CYCLE"


def test_plunge_runs_at_plunge_feed():
    result = generate_drill(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-10.0,
        state=_state(),
    )
    seq = [
        (c["type"], c.get("speed"))
        for c in _commands(result)
        if c["type"] in ("SET_FEED_RATE", "LINE_TO")
    ]
    assert ("SET_FEED_RATE", 300) in seq, "plunge feed not set"
    plunge = seq.index(("SET_FEED_RATE", 300))
    assert seq[plunge + 1][0] == "LINE_TO", "plunge feed not on the plunge"
    assert ("SET_FEED_RATE", 1000) in seq[plunge + 1 :], (
        "cut feed not restored"
    )


def test_dwell_emitted_when_configured():
    result = generate_drill(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-10.0,
        dwell_ms=200.0,
    )
    dwells = [c for c in _commands(result) if c["type"] == "DWELL"]
    assert len(dwells) == 1
    assert dwells[0]["duration_ms"] == pytest.approx(200.0)


def test_peck_full_retract_sequence():
    """12 mm deep hole, 5 mm pecks: three plunges, full retracts to the
    rapid plane between them, re-entry just above the previous bottom."""
    result = generate_drill(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-12.0,
        peck=PeckParams(increment=5.0),
        state=_state(),
    )
    moves = _moving(result)
    tip = -12.0 - (2.5 / math.tan(math.radians(118.0) / 2.0))
    # Per peck: re-entry (from 2nd), plunge, retract to the rapid plane.
    # Final retract to safe_z.
    bottoms = [z for (t, z) in moves if t == "LINE_TO"]
    assert len(bottoms) == 3
    assert bottoms[0] == pytest.approx(-5.0)
    assert bottoms[1] == pytest.approx(-10.0)
    assert bottoms[2] == pytest.approx(tip)
    post_plunge_retracts = [
        z
        for i, (t, z) in enumerate(moves)
        if t == "MOVE_TO" and moves[i - 1][0] == "LINE_TO"
    ]
    assert post_plunge_retracts == [1.0, 1.0, 1.0], moves
    assert moves[-1] == ("MOVE_TO", 5.0), "final retract to safe missing"
    # Re-entry stops 0.5 mm above the previous bottom.
    reentries = [z for (t, z) in moves if t == "MOVE_TO" and z < 0.0]
    assert reentries == [pytest.approx(-4.5), pytest.approx(-9.5)]


def test_peck_chip_break_retracts_a_small_amount():
    # 90° point: the tip reaches 2.5 mm deeper than target_z, so a
    # -8 target drills to -10.5: four 3 mm pecks.
    result = generate_drill(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-8.0,
        tip_angle=90.0,
        peck=PeckParams(increment=3.0, mode="CHIP_BREAK", retract_amount=0.4),
    )
    moves = _moving(result)
    # Chip-break: after each plunge, retract only 0.4 mm and rapid back
    # to the previous bottom; never all the way out between pecks.
    break_retracts = [
        z
        for i, (t, z) in enumerate(moves)
        if t == "MOVE_TO"
        and i > 0
        and moves[i - 1][0] == "LINE_TO"
        and z < 1.0
    ]
    assert break_retracts == [
        pytest.approx(-2.6),
        pytest.approx(-5.6),
        pytest.approx(-8.6),
        pytest.approx(-10.1),
    ], moves
    # Ends with the full retract to safe.
    assert moves[-1] == ("MOVE_TO", 5.0)


def test_dwell_on_every_peck():
    result = generate_drill(
        Part.from_polygons([]),
        center=(0.0, 0.0),
        diameter=5.0,
        target_z=-6.0,
        tip_angle=90.0,  # tip to -8.5: three 3 mm pecks
        peck=PeckParams(increment=3.0),
        dwell_ms=100.0,
    )
    dwells = [c for c in _commands(result) if c["type"] == "DWELL"]
    assert len(dwells) == 3


def test_countersink_depth_from_diameter_and_angle():
    result = generate_countersink(
        Part.from_polygons([]),
        center=(5.0, 5.0),
        tip_angle=90.0,
        target_diameter=8.0,
    )
    moves = _moving(result)
    depth = 4.0 / math.tan(math.radians(45.0))  # 4 mm for a 90° cone
    plunge = [z for (t, z) in moves if t == "LINE_TO"]
    assert plunge == [pytest.approx(-depth)]
    # The countersink disk is cut at the chamfer diameter.
    part = Part.from_polygons([])
    generate_countersink(
        part, center=(0.0, 0.0), tip_angle=90.0, target_diameter=10.0
    )
    assert part.cleared.total_area() == pytest.approx(math.pi * 25.0, rel=5e-3)


def test_invalid_parameters_are_rejected():
    part = Part.from_polygons([])
    with pytest.raises(Exception, match="diameter"):
        generate_drill(part, center=(0, 0), diameter=0.0, target_z=-5.0)
    with pytest.raises(Exception, match="target_z"):
        generate_drill(part, center=(0, 0), diameter=5.0, target_z=1.0)
    with pytest.raises(Exception, match="rapid_z"):
        generate_drill(
            part, center=(0, 0), diameter=5.0, target_z=-5.0, rapid_z=-2.0
        )
    with pytest.raises(Exception, match="tip_angle"):
        generate_drill(
            part, center=(0, 0), diameter=5.0, target_z=-5.0, tip_angle=200.0
        )
    with pytest.raises(Exception, match="target_diameter"):
        generate_countersink(
            part, center=(0, 0), tip_angle=90.0, target_diameter=0.0
        )


def test_result_poses_and_cleared_area():
    part = Part.from_polygons([])
    result = generate_drill(
        part,
        center=(3.0, 4.0),
        diameter=6.0,
        target_z=-6.0,
    )
    assert result.start.pos == pytest.approx((3.0, 4.0, 5.0))
    assert result.end.pos == pytest.approx((3.0, 4.0, 5.0))
    assert part.cleared.total_area() > 0.0
