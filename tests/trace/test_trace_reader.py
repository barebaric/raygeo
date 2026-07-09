"""Tests for the new span/event trace file reader (step 3)."""

from raygeo.cnc.machining.plan import Workplan
from raygeo.trace import TraceFile


def _rect(x0, y0, w, h):
    return [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]


def test_trace_reader_basic(tmp_path):
    """Build a two-step workplan, execute with trace, verify reader output."""
    boundary = _rect(-5, -5, 40, 40)

    steps = [
        {
            "kind": "HelixPlunge",
            "center": (10.0, 10.0),
            "helix_r": 4.0,
            "z_start": 0.0,
            "z_end": -5.0,
            "pitch": 1.0,
            "direction": "CW",
            "angular_step": 0.5,
        },
        {
            "kind": "HelixPlunge",
            "center": (25.0, 25.0),
            "helix_r": 4.0,
            "z_start": 0.0,
            "z_end": -5.0,
            "pitch": 1.0,
            "direction": "CW",
            "angular_step": 0.5,
        },
    ]

    wp = Workplan(boundary, safe_z=2.0)
    wp.extend(steps)

    tp = str(tmp_path / "trace.bin")
    wp.execute(trace=tp)

    trace = TraceFile(tp)

    # ── Header ─────────────────────────────────────────────────
    assert trace.ver == 3

    # ── Root span ──────────────────────────────────────────────
    assert trace.root is not None
    assert trace.root.source == "workplan"
    assert trace.root.label == "Workplan"

    # ── Sources include the helix assembler ──────────────────────
    assert "helix" in trace.sources
    assert "workplan" in trace.sources

    # ── Events ──────────────────────────────────────────────────
    assert len(trace.events) > 0
    move_events = [e for e in trace.events if e.kind == "move"]
    assert len(move_events) > 0

    # ── Children of root ────────────────────────────────────────
    # root should have at least two step spans and a link span
    assert len(trace.root.children) >= 2

    step_spans = [c for c in trace.root.children if c.source == "helix"]
    assert len(step_spans) >= 1
    # There should be a link span between the two steps

    # ── Span order: step spans should have events that include moves ──
    for sp in step_spans:
        assert len(sp.events) > 0
        has_move = any(e.kind == "move" for e in sp.events)
        assert has_move, f"span {sp} has no move events"

    # ── toolpath ────────────────────────────────────────────────
    tp_result = trace.toolpath()
    assert len(tp_result) > 0
    for x, y, mk in tp_result:
        assert isinstance(x, float)
        assert isinstance(y, float)
        assert isinstance(mk, str)

    # ── Root attrs ──────────────────────────────────────────────
    attrs = trace.root.attrs
    assert isinstance(attrs, dict)
    assert "safe_z" in attrs
    assert isinstance(attrs["safe_z"], float)
    assert attrs["safe_z"] == 2.0
    assert "steps" in attrs
    assert isinstance(attrs["steps"], list)
    assert len(attrs["steps"]) >= 1
    for s in attrs["steps"]:
        assert isinstance(s, str)

    # ── __len__ / __getitem__ ────────────────────────────────────
    assert len(trace) == len(trace.events)
    assert trace[0] is not None
    assert trace[0].kind in ("init", "move", "resume", "exit")

    # ── Span __repr__ ────────────────────────────────────────────
    assert repr(trace.root).startswith("Span(")
    assert "workplan" in repr(trace.root)
