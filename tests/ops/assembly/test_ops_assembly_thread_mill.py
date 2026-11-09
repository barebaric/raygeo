"""Tests for the thread-mill assembler (helical interpolation)."""

import math

import pytest

from raygeo.ops.assembly.thread_mill import generate_thread_mill
from raygeo.ops.part import Part
from raygeo.ops.state import State


def _state():
    return State(feed_rate=800, plunge_feed=200, spindle_rpm=2000)


def _commands(result):
    return result.ops.to_dict()["commands"]


def _arcs(result):
    return [c for c in _commands(result) if c["type"] == "ARC_TO"]


CENTER = (10.0, 10.0)
NOMINAL, PITCH, TOOL = 6.0, 1.0, 4.0
Z_TOP, Z_BOTTOM = 0.0, -12.0
ORBIT_R = (NOMINAL - TOOL) / 2.0  # 1.0


def _mill(**kwargs):
    part = Part.from_polygons([])
    kwargs.setdefault("center", CENTER)
    kwargs.setdefault("nominal_diameter", NOMINAL)
    kwargs.setdefault("pitch", PITCH)
    kwargs.setdefault("tool_diameter", TOOL)
    kwargs.setdefault("z_top", Z_TOP)
    kwargs.setdefault("z_bottom", Z_BOTTOM)
    kwargs.setdefault("state", _state())
    return part, generate_thread_mill(part, **kwargs)


def test_helix_arcs_orbit_at_the_thread_radius():
    """All helix arc endpoints sit at the orbit radius (the thread
    flank); the lead-in/out arc endpoints sit at the center."""
    _, result = _mill()
    arcs = _arcs(result)
    # Lead-in + 24 half-turns (12 revolutions) + lead-out.
    assert len(arcs) == 2 * 12 + 2
    for arc in arcs[1:-1]:
        x, y, z = arc["end"]
        assert math.hypot(x - CENTER[0], y - CENTER[1]) == pytest.approx(
            ORBIT_R, abs=1e-6
        ), f"helix arc off the orbit radius at {arc['end']}"
    assert arcs[0]["end"][:2] == pytest.approx(
        (CENTER[0] + ORBIT_R, CENTER[1])
    ), "lead-in must end on the orbit entry point"
    assert arcs[-1]["end"][:2] == pytest.approx(CENTER), (
        "lead-out must return to the center"
    )


def test_helix_climbs_one_pitch_per_revolution():
    _, result = _mill()
    arcs = _arcs(result)[1:-1]
    zs = [arc["end"][2] for arc in arcs]
    assert zs[0] == pytest.approx(Z_BOTTOM + PITCH / 2)
    assert zs[-1] == pytest.approx(Z_TOP)
    for z0, z1 in zip(zs, zs[1:]):
        assert z1 - z0 == pytest.approx(PITCH / 2)


def test_handedness_switches_orbit_direction():
    _, rh = _mill(handedness="RH")
    _, lh = _mill(handedness="LH")
    rh_dirs = {c["clockwise"] for c in _arcs(rh)}
    lh_dirs = {c["clockwise"] for c in _arcs(lh)}
    assert len(rh_dirs) == 1 and len(lh_dirs) == 1
    assert rh_dirs != lh_dirs, "handedness must flip the orbit direction"


def test_move_sequence_rapids_plunges_and_retracts():
    _, result = _mill()
    moves = [
        (c["type"], round(c["end"][2], 6))
        for c in _commands(result)
        if c["type"] in ("MOVE_TO", "LINE_TO")
    ]
    assert moves == [
        ("MOVE_TO", 5.0),  # safe_z
        ("LINE_TO", Z_BOTTOM),  # plunge in the pilot bore
        ("MOVE_TO", 5.0),  # final retract
    ]


def test_plunge_runs_at_plunge_feed():
    _, result = _mill()
    seq = [
        (c["type"], c.get("speed"))
        for c in _commands(result)
        if c["type"] in ("SET_FEED_RATE", "LINE_TO")
    ]
    plunge = seq.index(("SET_FEED_RATE", 200))
    assert seq[plunge + 1][0] == "LINE_TO"
    assert ("SET_FEED_RATE", 800) in seq[plunge + 1 :], (
        "cut feed not restored after the plunge"
    )


def test_non_integer_thread_length_is_rejected():
    with pytest.raises(Exception, match="integer multiple"):
        _mill(z_bottom=-12.5)


def test_tool_not_fitting_the_thread_is_rejected():
    with pytest.raises(Exception, match="tool_diameter"):
        _mill(tool_diameter=6.5)
    with pytest.raises(Exception, match="tool_diameter"):
        _mill(tool_diameter=0.0)


def test_invalid_pitch_and_heights_are_rejected():
    with pytest.raises(Exception, match="pitch"):
        _mill(pitch=0.0)
    with pytest.raises(Exception, match="z_top"):
        _mill(z_top=-15.0)


def test_marks_bore_cleared():
    part, _ = _mill()
    # Disk at the major radius: π·3² ≈ 28.27, minus the 64-gon error.
    assert part.cleared.total_area() == pytest.approx(
        math.pi * (NOMINAL / 2.0) ** 2, rel=5e-3
    )
