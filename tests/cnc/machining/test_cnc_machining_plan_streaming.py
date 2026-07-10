"""Tests for the on_progress streaming callback on Workplan.execute."""

from raygeo.cnc.machining.plan import Workplan
from raygeo.cnc.machining.wavefront import build_wavefront_workplan


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_workplan_streams_step_events():
    boundary = _rect(-20, -20, 40, 40)
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        safe_margin=1.0,
        angular_step=0.1,
        area_tolerance=1.0,
        precision=0.0,
    )
    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)

    events = []
    wp.execute(on_progress=lambda e: events.append(e))

    kinds = [e["kind"] for e in events]
    assert "step_start" in kinds, f"expected step_start, got {kinds}"
    assert "ops" in kinds, f"expected ops, got {kinds}"
    assert "step_end" in kinds, f"expected step_end, got {kinds}"
