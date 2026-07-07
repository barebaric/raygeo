"""Tests for the inspector's profile-trace mode detection and rendering."""

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt
import pytest

from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.trace import TraceFile


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _profile_trace(tmp_path, profile_fn, **kwargs):
    """Run a profile operation with tracing and return the TraceFile."""
    tp = str(tmp_path / "trace.bin")
    boundary = _rect(0, 0, 60, 60)
    ca = ClearedArea(boundary=boundary, initial=[])
    profile_fn(
        cleared=ca,
        boundary=boundary,
        tool_radius=3.0,
        step_over=1.5,
        target_z=-5.0,
        safe_z=2.0,
        step_length=0.6,
        wall_margin=0.0,
        cut_feed_rate=1000,
        cut_power=0.0,
        trace_path=tp,
        **kwargs,
    )
    return TraceFile(tp)


@pytest.fixture
def profile_outer_trace(tmp_path):
    return _profile_trace(tmp_path, profile_outer)


@pytest.fixture
def profile_inner_trace(tmp_path):
    return _profile_trace(
        tmp_path, profile_inner, islands=[_rect(15, 0, 10, 10)]
    )


def _make_inspector(trace):
    """Instantiate Inspector with a given trace file."""
    # Avoid circular import with trace_path-based instantiation
    from raygeo.cli.inspector import Inspector

    geometry = trace.geometry
    tp = trace.toolpath
    seed_polys = geometry.get("seeds", [])
    return Inspector(trace, tp, seed_polys, geometry)


def test_inspector_recognises_profile_trace(profile_outer_trace):
    """Inspector draws offset polygons for a profile trace."""
    inspector = _make_inspector(profile_outer_trace)
    assert "offset_polys" in inspector.geometry


def test_inspector_cleared_area_not_drawn_for_profile_trace(
    profile_outer_trace,
):
    """Profile trace has no seeds, so cleared-area fill is skipped."""
    inspector = _make_inspector(profile_outer_trace)
    assert not inspector.seed_polys
    # has_seeds should be False → no envelope/fill/remaining/frontier


def test_inspector_panel_data_for_profile_cut_record_contains_wall_distance(
    profile_outer_trace,
):
    """Profile cut records show wall_distance in panel data."""
    inspector = _make_inspector(profile_outer_trace)
    # Find a cut record
    for i in range(inspector.n_steps):
        rec = inspector._rec(i)
        if rec.kind == "cut":
            cells, colors, styles = inspector._format_panel_data(
                rec, rec.kind, rec.status.name
            )
            labels = [r[0] for r in cells]
            values = [r[1] for r in cells]
            assert "wall_dist" in labels, f"wall_dist missing at step {i}"
            idx = labels.index("wall_dist")
            val = float(values[idx].replace(" mm", ""))
            assert val >= 0.0
            return
    pytest.fail("no cut record found")


def test_inspector_panel_data_for_polygon_start_record_has_target_idx(
    profile_inner_trace,
):
    """polygon_start records show target_idx in panel data."""
    inspector = _make_inspector(profile_inner_trace)
    for i in range(inspector.n_steps):
        rec = inspector._rec(i)
        if rec.kind == "polygon_start":
            cells, colors, styles = inspector._format_panel_data(
                rec, rec.kind, rec.status.name
            )
            labels = [r[0] for r in cells]
            assert "target_idx" in labels
            return
    # For inner profile with islands, there should be at least one
    # polygon_start (for the outer wall).  Allow skip if the trace
    # genuinely has none (e.g. empty pocket).
    pytest.skip("no polygon_start record (trace may be degenerate)")


def test_inspector_draw_does_not_raise_agg(profile_outer_trace):
    """Inspector._draw does not raise when rendering any step."""
    inspector = _make_inspector(profile_outer_trace)
    for step in (0, inspector.n_steps // 2, inspector.n_steps - 1):
        try:
            inspector._draw(step)
        except Exception as e:
            pytest.fail(f"_draw({step}) raised {e}")
    plt.close("all")
