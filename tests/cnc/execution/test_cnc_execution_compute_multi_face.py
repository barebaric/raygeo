"""Tests for multi-face iteration in the Compute stage (Step 3).

Verifies that ``AssemblerCompute::run()``:

* Iterates every face of a multi-face part when ``face_id`` is empty,
  collecting per-face operations and emitting a non-fatal
  ``AssemblyWarning`` (kind ``FACE_FAILED``) for a face whose assembly
  fails, while still producing ops for the faces that succeed.
* Processes only the requested face when ``face_id`` is set.
* Emits no warnings when every face assembles successfully.

A ``ProfileSpec`` (outer) is used because it returns ``Err(String)``
with a ``DegenerateGeometry`` message for a face that has no extractable
boundary — a clean, controllable per-face failure.
"""

import pytest
from conftest import collect_completions, compute_result

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.geo import Geometry
from raygeo.ops.assembly import Assembler, AssemblyWarningKind
from raygeo.ops.assembly.profile import ProfileSpec
from raygeo.ops.part import Part
from raygeo.pipeline.execute import clear_cache
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec


def _square_part_with_empty_second_face() -> Part:
    """Face ``""`` is a valid 10x10 square; face ``"1"`` is empty."""
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.line_to(0, 0)
    p = Part(geometry=g, size_mm=(20.0, 20.0))
    p.add_face("1", Geometry())
    return p


def _two_valid_faces_part() -> Part:
    """Two disjoint squares as separate faces via auto-detection."""
    g = Geometry()
    a = Geometry()
    a.move_to(0, 0)
    a.line_to(10, 0)
    a.line_to(10, 10)
    a.line_to(0, 10)
    a.line_to(0, 0)
    b = Geometry()
    b.move_to(30, 30)
    b.line_to(34, 30)
    b.line_to(34, 34)
    b.line_to(30, 34)
    b.line_to(30, 30)
    g.extend(a)
    g.extend(b)
    return Part.from_geometry_multi_face(g, size_mm=(40.0, 40.0))


def _profile_outer_compute(
    part: Part, face_id: str = "", key: str = "c"
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=part,
            params=ComputePayload(
                assembler=Assembler(ProfileSpec(kind="outer", tool_radius=1.0))
            ),
            face_id=face_id,
        ),
    )


@pytest.fixture(autouse=True)
def _clear_cache():
    clear_cache()
    yield
    clear_cache()


# ── Multi-face iteration with per-face recovery ─────────────────


def test_failed_face_emits_warning_successful_face_still_machined():
    p = _square_part_with_empty_second_face()
    completed, _ = collect_completions([_profile_outer_compute(p)])
    assert len(completed) == 1
    node = completed[0]

    # No fatal error — the pipeline recovered and produced output.
    assert node.error is None
    assert node.output is not None

    out = compute_result(node)
    warnings = out.warnings
    assert len(warnings) == 1
    [w] = warnings
    assert w.kind == AssemblyWarningKind.FACE_FAILED
    assert w.face_id == "1"
    assert w.region is None
    assert "boundary" in w.detail.lower() or "degenerate" in w.detail.lower()

    # The valid face "" still produced ops.
    assert len(out.ops) > 0


def test_successful_faces_produce_no_warnings():
    p = _two_valid_faces_part()
    assert set(p.face_ids) == {"", "1"}

    completed, _ = collect_completions([_profile_outer_compute(p)])
    assert len(completed) == 1
    node = completed[0]
    assert node.error is None
    out = compute_result(node)
    assert out.warnings == []
    assert len(out.ops) > 0


def test_empty_face_id_iterates_all_faces():
    p = _square_part_with_empty_second_face()
    assert set(p.face_ids) == {"", "1"}

    completed, _ = collect_completions([_profile_outer_compute(p)])
    node = completed[0]
    out = compute_result(node)
    # Exactly one face ("1") failed; the other ("") succeeded.
    warning_face_ids = {w.face_id for w in out.warnings}
    assert warning_face_ids == {"1"}


# ── Explicit face_id processes only that face ───────────────────


def test_explicit_face_id_matches_failed_face():
    p = _square_part_with_empty_second_face()
    # Request only the empty face, which fails. When *every* attempted
    # face fails (here: the single requested one), the compute surfaces
    # a hard error rather than a soft warning — so the pipeline's
    # failure-cascade contract (a fully-failed compute does not spawn
    # dependents) still holds (test_pipeline_failure_propagation).
    completed, _ = collect_completions(
        [_profile_outer_compute(p, face_id="1")]
    )
    assert len(completed) == 1
    node = completed[0]
    assert node.error is not None
    assert node.output is None
    detail = node.error.lower()
    assert "boundary" in detail or "degenerate" in detail


def test_empty_face_id_does_not_select_only_default_face():
    # An *empty* face_id means "iterate all faces", not "only the
    # default face": the failing face "1" still gets processed and
    # warns even though the default face "" is present and valid.
    p = _square_part_with_empty_second_face()
    completed, _ = collect_completions([_profile_outer_compute(p, face_id="")])
    assert len(completed) == 1
    node = completed[0]
    assert node.error is None
    out = compute_result(node)
    assert {w.face_id for w in out.warnings} == {"1"}
