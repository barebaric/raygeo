"""Tests for build_wavefront_workplan in cnc/machining/wavefront."""

from raygeo.cnc.machining.wavefront import build_wavefront_workplan


def _rect(x0, y0, w, h):
    """CCW rectangle starting at (x0, y0)."""
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_wavefront_workplan_structure():
    """Wide rect -> [FlatSpiral, Wavefront]."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        area_tolerance=1.0,
    )
    kinds = [s["kind"] for s in steps]
    assert kinds == ["FlatSpiral", "Wavefront"], f"got {kinds}"


def test_wavefront_workplan_never_emits_helix():
    """The wavefront builder produces no helical plunge (decoupling)."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
    )
    assert all(s["kind"] != "HelixPlunge" for s in steps)


def test_wavefront_workplan_flat_spiral_radius():
    """FlatSpiral end_radius == legacy spiral_max_r (r_max - r - margin).

    For a 40x40 rect r_max ~= 20, so with tool_radius=3 and the default
    safe_margin=1 the seed disk reaches ~16. This is the behaviour
    preservation guarantee: the wavefront seed is identical to the old
    helix+spiral path.
    """
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        safe_margin=1.0,
    )
    spiral = next(s for s in steps if s["kind"] == "FlatSpiral")
    assert abs(spiral["end_radius"] - 16.0) < 0.2, (
        f"expected end_radius ~= 16, got {spiral['end_radius']}"
    )
    assert spiral["start_radius"] < spiral["end_radius"]
    assert spiral["revolutions"] > 0.0


def test_wavefront_workplan_wavefront_fields():
    """The Wavefront step mirrors the assembler options."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        area_tolerance=0.5,
        precision=0.2,
    )
    wf = next(s for s in steps if s["kind"] == "Wavefront")
    assert abs(wf["tool_radius"] - 3.0) < 1e-9
    assert abs(wf["step_over"] - 2.0) < 1e-9
    assert abs(wf["z"] - (-5.0)) < 1e-9
    assert abs(wf["area_tolerance"] - 0.5) < 1e-9
    assert abs(wf["precision"] - 0.2) < 1e-9
    assert len(wf["pocket_boundary"]) == 4


def test_wavefront_workplan_step_over_zero():
    """step_over=0 -> no FlatSpiral, only the Wavefront step."""
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=0.0,
        target_z=-5.0,
    )
    kinds = [s["kind"] for s in steps]
    assert "FlatSpiral" not in kinds
    assert kinds == ["Wavefront"]


def test_wavefront_workplan_with_islands():
    """Islands are forwarded to the Wavefront step."""
    boundary = _rect(0, 0, 60, 60)
    island = _rect(25, 25, 10, 10)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        islands=[island],
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
    )
    wf = next(s for s in steps if s["kind"] == "Wavefront")
    assert len(wf["islands"]) == 1
