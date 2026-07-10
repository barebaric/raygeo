"""Tests for the on_progress streaming callback."""

import math

from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.cut.cleared_area import ClearedArea


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _circle(cx, cy, r, n=32):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def test_adaptive_clearing_streams_ops():
    """on_progress receives ops batches during adaptive clearing."""
    boundary = _rect(0, 0, 60, 60)
    seed = [_circle(0, 0, 5)]
    ca = ClearedArea(boundary=boundary, initial=seed)
    received = []
    total = [0]

    def on_progress(event):
        if event["kind"] == "ops":
            received.append(event["ops_count"])
            total[0] = event["ops_total"]

    result = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=3.0,
        step_over=2.0,
        step_length=0.6,
        target_z=-5.0,
        safe_z=2.0,
        on_progress=on_progress,
        batch_size=50,
    )
    assert len(received) > 1, f"expected multiple batches, got {len(received)}"
    assert 0 < total[0] <= result.ops.len()


def test_streaming_does_not_change_result():
    """Streaming produces the same ops count as non-streaming."""
    boundary = _rect(0, 0, 60, 60)
    seed = [_circle(0, 0, 5)]

    ca1 = ClearedArea(boundary=boundary, initial=seed)
    r1 = adaptive_clearing(
        cleared=ca1,
        pocket_boundary=boundary,
        tool_radius=3.0,
    )

    ca2 = ClearedArea(boundary=boundary, initial=seed)
    r2 = adaptive_clearing(
        cleared=ca2,
        pocket_boundary=boundary,
        tool_radius=3.0,
        on_progress=lambda e: None,
        batch_size=50,
    )

    assert r1.ops.len() == r2.ops.len()
