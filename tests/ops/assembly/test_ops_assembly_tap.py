"""Tests for the rigid-tapping assembler."""

import math

import pytest

from raygeo.ops.assembly.tap import generate_tap
from raygeo.ops.part import Part
from raygeo.ops.state import State


def _state(**kwargs):
    return State(feed_rate=1000, plunge_feed=300, spindle_rpm=500, **kwargs)


def _tap(part=None, **kwargs):
    kwargs.setdefault("center", (0.0, 0.0))
    kwargs.setdefault("nominal_diameter", 6.0)
    kwargs.setdefault("pitch", 1.0)
    kwargs.setdefault("target_z", -12.0)
    kwargs.setdefault("state", _state())
    return generate_tap(part or Part.from_polygons([]), **kwargs)


def _commands(result):
    return result.ops.to_dict()["commands"]


def _moving(result):
    return [
        (c["type"], round(c["end"][2], 6))
        for c in _commands(result)
        if c["type"] in ("MOVE_TO", "LINE_TO")
    ]


def _directions(result):
    return [
        c["direction"]
        for c in _commands(result)
        if c["type"] == "SET_SPINDLE_DIRECTION"
    ]


def _pitches(result):
    return [
        c["pitch"] for c in _commands(result) if c["type"] == "SET_SYNC_PITCH"
    ]


def test_rigid_tap_move_sequence():
    """Rapid to safe, rapid to the rapid plane, synchronized descent,
    synchronized retract to the rapid plane, rapid out to safe."""
    result = _tap()
    assert _moving(result) == [
        ("MOVE_TO", 5.0),  # safe_z
        ("MOVE_TO", 1.0),  # rapid_z
        ("LINE_TO", -12.0),  # synchronized tap down
        ("LINE_TO", 1.0),  # synchronized retract
        ("MOVE_TO", 5.0),  # rapid out to safe
    ]


def test_cycle_wrapped_in_tapping_cycle_sections():
    result = _tap()
    types = [c["type"] for c in _commands(result)]
    assert types.count("OPS_SECTION_START") == 1
    assert types.count("OPS_SECTION_END") == 1
    first = _commands(result)[types.index("OPS_SECTION_START")]
    assert first["section_type"] == "TAPPING_CYCLE"


def test_spindle_reverses_at_the_bottom_rh():
    """Right-hand tap: clockwise down, counter-clockwise up, clockwise
    restored afterwards (G84)."""
    assert _directions(_tap()) == ["Cw", "Ccw", "Cw"]


def test_handedness_flips_directions_lh():
    """Left-hand tap: counter-clockwise down, clockwise up (G74)."""
    assert _directions(_tap(handedness="LH")) == ["Ccw", "Cw", "Ccw"]


def test_sync_pitch_set_and_cleared():
    """The sync pitch is active during the cycle and switched off
    before the rapid out."""
    assert _pitches(_tap()) == [pytest.approx(1.0), pytest.approx(0.0)]


def test_reversal_is_ordered_around_the_retract():
    """The retract move runs under the reversed spindle: the direction
    command precedes the retract LINE_TO and the restore follows it."""
    seq = [
        (c["type"], c.get("direction"))
        for c in _commands(_tap())
        if c["type"] in ("SET_SPINDLE_DIRECTION", "LINE_TO")
    ]
    down = seq.index(("LINE_TO", None))
    assert seq[down + 1] == ("SET_SPINDLE_DIRECTION", "Ccw")
    assert seq[down + 2][0] == "LINE_TO", "retract not a synchronized feed"
    assert seq[down + 3] == ("SET_SPINDLE_DIRECTION", "Cw")


def test_requires_spindle_rpm():
    with pytest.raises(Exception, match="spindle"):
        _tap(state=State(feed_rate=1000))
    with pytest.raises(Exception, match="above 0"):
        _tap(state=State(feed_rate=1000, spindle_rpm=0))


def test_peck_full_retract_sequence():
    """12 mm thread, 5 mm pecks: three synchronized passes, full
    synchronized retracts to the rapid plane between them, re-entry
    just above the previous bottom."""
    result = _tap(peck=5.0)
    moves = _moving(result)
    bottoms = [z for (t, z) in moves if t == "LINE_TO"]
    # Per peck: descent, retract to the rapid plane; final retract
    # after the last pass.
    assert bottoms == [
        pytest.approx(-5.0),
        pytest.approx(1.0),
        pytest.approx(-10.0),
        pytest.approx(1.0),
        pytest.approx(-12.0),
        pytest.approx(1.0),
    ]
    # Rapid re-entry stops 0.5 mm above the previous bottom.
    reentries = [z for (t, z) in moves if t == "MOVE_TO" and z < 0.0]
    assert reentries == [pytest.approx(-4.5), pytest.approx(-9.5)]
    assert moves[-1] == ("MOVE_TO", 5.0), "final retract to safe missing"


def test_dwell_emitted_when_configured():
    result = _tap(dwell_ms=50.0)
    dwells = [c for c in _commands(result) if c["type"] == "DWELL"]
    assert len(dwells) == 1
    assert dwells[0]["duration_ms"] == pytest.approx(50.0)
    # The dwell sits between the descent and the spindle reversal.
    types = [c["type"] for c in _commands(result)]
    assert types.index("DWELL") < types.index("OPS_SECTION_END")
    assert types[types.index("DWELL") + 1] == "SET_SPINDLE_DIRECTION"


def test_invalid_parameters_are_rejected():
    part = Part.from_polygons([])
    with pytest.raises(Exception, match="nominal_diameter"):
        _tap(part, nominal_diameter=0.0)
    with pytest.raises(Exception, match="pitch"):
        _tap(part, pitch=0.0)
    with pytest.raises(Exception, match="target_z"):
        _tap(part, target_z=1.0)
    with pytest.raises(Exception, match="rapid_z"):
        _tap(part, rapid_z=-2.0)
    with pytest.raises(Exception, match="safe_z"):
        _tap(part, safe_z=0.5)
    with pytest.raises(Exception, match="peck increment"):
        _tap(part, peck=0.0)


def test_result_poses_and_cleared_area():
    part = Part.from_polygons([])
    result = _tap(part, center=(3.0, 4.0))
    assert result.start.pos == pytest.approx((3.0, 4.0, 5.0))
    assert result.end.pos == pytest.approx((3.0, 4.0, 5.0))
    # The thread disk is cut at the nominal diameter.
    assert part.cleared.total_area() == pytest.approx(
        math.pi * 3.0**2, rel=5e-3
    )
