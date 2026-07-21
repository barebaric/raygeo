"""Cache tests for the pipeline.

The pipeline cache is a simple key→value store with LRU eviction.
The cache key is the node's ``key`` string only — no content hash.

There is no automatic invalidation. Callers invalidate by:

  - calling ``clear_cache_prefix(tag)`` or ``clear_cache()`` to drop
    entries explicitly, or
  - bumping the node's epoch via the cache (supersession), which
    causes in-flight results to be discarded on completion.

Changing an assembler's parameters does NOT invalidate the cache —
the caller must evict explicitly. This is by design: hashing geometry
and raster pixels on every lookup is too expensive.

All compute and aggregate nodes cache uniformly. The tests below
exercise both external invalidation and the cache-hit / cache-miss
behavior.
"""

from typing import List

from conftest import (
    make_square_part,
    result_ops,
)

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.adaptive import AdaptiveClearingSpec
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.part import Part
from raygeo.ops.transform.overscan import OverscanSpec
from raygeo.ops.transform.smooth import SmoothSpec
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import Pipeline
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

# ── Helpers ────────────────────────────────────────────────────────


def _adaptive_part():
    """A small Part with a rectangular pocket for adaptive clearing."""
    boundary = [
        (-20.0, -20.0),
        (20.0, -20.0),
        (20.0, 20.0),
        (-20.0, 20.0),
    ]
    seed = [[(-5.0, -5.0), (5.0, -5.0), (5.0, 5.0), (-5.0, 5.0)]]
    return Part.from_polygons(boundary, initial=seed)


def _adaptive_node(
    key: str,
    spec: AdaptiveClearingSpec,
    part: Part | None = None,
    transformers=None,
    generation_id: int = 1,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=generation_id,
        stage=StageSpec.Compute(
            part=part or _adaptive_part(),
            params=ComputePayload(
                assembler=Assembler(spec),
                transformers=transformers or [],
            ),
        ),
    )


def _contour_node(
    key: str,
    spec: ContourSpec | None = None,
    transformers=None,
    generation_id: int = 1,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=generation_id,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(
                assembler=Assembler(spec or ContourSpec()),
                transformers=transformers or [],
            ),
        ),
    )


def _run_pipeline(p: Pipeline, nodes):
    """Run nodes through the given Pipeline, return completions by key."""
    completed: List[CompletedNode] = []
    p.execute(nodes, completed.append, None)
    return {c.key: c for c in completed}


# ── External invalidation ──────────────────────────────────────────


def test_clear_cache_drops_all_entries():
    """After clear_cache(), cache_used_bytes returns to 0 and
    re-running produces fresh completions."""
    p = Pipeline()
    _run_pipeline(p, [_adaptive_node("a1", AdaptiveClearingSpec())])
    assert p.cache_used_bytes > 0, "first run should populate cache"
    p.clear_cache()
    assert p.cache_used_bytes == 0


def test_clear_cache_prefix_drops_only_matching_entries():
    """clear_cache_prefix('a-') drops only whose tag starts with 'a-'."""
    p = Pipeline()
    _run_pipeline(
        p,
        [
            _adaptive_node("a-1", AdaptiveClearingSpec()),
            _adaptive_node("a-2", AdaptiveClearingSpec()),
            _adaptive_node("b-1", AdaptiveClearingSpec()),
        ],
    )
    before = p.cache_used_bytes
    assert before > 0
    p.clear_cache_prefix("a-")
    after = p.cache_used_bytes
    assert after < before, "prefix clear should drop at least one entry"
    assert after > 0, "non-matching entries should remain"


def test_clear_cache_prefix_with_empty_string_clears_everything():
    """An empty prefix matches every tag (``str.startswith("")`` is
    always true), so clear_cache_prefix("") is equivalent to
    clear_cache()."""
    p = Pipeline()
    _run_pipeline(p, [_adaptive_node("a1", AdaptiveClearingSpec())])
    assert p.cache_used_bytes > 0
    p.clear_cache_prefix("")
    assert p.cache_used_bytes == 0


def test_generation_id_change_alone_does_not_invalidate():
    """The cache is keyed by node key, not generation_id.

    Re-running the same key with a different generation_id still
    hits cache. generation_id is for shadow-node tracking, not cache
    invalidation.
    """
    p = Pipeline()
    first = _run_pipeline(
        p, [_adaptive_node("a", AdaptiveClearingSpec(), generation_id=1)]
    )
    assert first["a"].error is None
    cached_bytes = p.cache_used_bytes
    assert cached_bytes > 0
    second = _run_pipeline(
        p, [_adaptive_node("a", AdaptiveClearingSpec(), generation_id=2)]
    )
    assert second["a"].error is None
    assert p.cache_used_bytes == cached_bytes, (
        "cache size grew — generation_id change should have hit cache"
    )


def test_pipeline_instances_have_independent_caches():
    """Cache populated via Pipeline A is invisible to Pipeline B."""
    a = Pipeline()
    b = Pipeline()
    _run_pipeline(a, [_adaptive_node("a1", AdaptiveClearingSpec())])
    assert a.cache_used_bytes > 0
    assert b.cache_used_bytes == 0


# ── Cache key = node key only ──────────────────────────────────────


def _adaptive_spec(tool_radius=3.0, step_over=1.5, target_z=-5.0):
    return AdaptiveClearingSpec(
        tool_radius=tool_radius,
        step_over=step_over,
        target_z=target_z,
    )


def test_cache_hit_on_identical_spec():
    """Running identical adaptive compute twice keeps the cache size
    stable (cache hit, entry overwritten in place)."""
    p = Pipeline()
    spec = _adaptive_spec()
    first = _run_pipeline(p, [_adaptive_node("a", spec)])
    assert first["a"].error is None
    after_first = p.cache_used_bytes
    assert after_first > 0
    second = _run_pipeline(p, [_adaptive_node("a", spec)])
    assert second["a"].error is None
    assert p.cache_used_bytes == after_first, (
        "second run should have hit cache, not added a new entry"
    )


def test_parameter_change_does_not_auto_invalidate():
    """Changing tool_radius does NOT cause a cache miss — the cache
    key is the node key, not a content hash. The caller must evict
    explicitly (via clear_cache_prefix) to force recomputation.
    """
    p = Pipeline()
    spec_a = _adaptive_spec(tool_radius=3.0)
    _run_pipeline(p, [_adaptive_node("a", spec_a)])
    after_a = p.cache_used_bytes
    spec_b = _adaptive_spec(tool_radius=3.5)
    _run_pipeline(p, [_adaptive_node("a", spec_b)])
    assert p.cache_used_bytes == after_a, (
        "parameter change should NOT auto-invalidate under explicit "
        "invalidation model; cache size should be stable"
    )


def test_eviction_then_rerun_recomputes():
    """After clear_cache_prefix, re-running with different params
    recomputes and the cache grows again."""
    p = Pipeline()
    spec_a = _adaptive_spec(tool_radius=3.0)
    _run_pipeline(p, [_adaptive_node("a", spec_a)])
    assert p.cache_used_bytes > 0

    p.clear_cache_prefix("a")
    assert p.cache_used_bytes == 0

    spec_b = _adaptive_spec(tool_radius=3.5)
    _run_pipeline(p, [_adaptive_node("a", spec_b)])
    assert p.cache_used_bytes > 0, (
        "after eviction, re-run should repopulate cache"
    )


def test_stale_cache_returns_old_result_without_eviction():
    """Without eviction, a cache hit returns the previously stored
    (stale) result even if the assembler params changed. This
    documents the explicit-invalidation contract: the caller is
    responsible for evicting when content changes.
    """
    p = Pipeline()
    ops_a = result_ops(
        _run_pipeline(p, [_adaptive_node("a", _adaptive_spec(target_z=-5.0))])[
            "a"
        ]
    ).to_dict()

    ops_b = result_ops(
        _run_pipeline(p, [_adaptive_node("a", _adaptive_spec(target_z=-3.0))])[
            "a"
        ]
    ).to_dict()

    assert ops_a == ops_b, (
        "without eviction, second run should return cached (stale) result"
    )


def test_cache_hit_preserves_cleared_fragments():
    """On a cache hit, face.cleared.fragments() should be restored
    to the same state as the original run."""
    p = Pipeline()
    part_a = _adaptive_part()
    part_b = _adaptive_part()

    nr_a = NodeRequest(
        key="a",
        generation_id=1,
        stage=StageSpec.Compute(
            part=part_a,
            params=ComputePayload(assembler=Assembler(_adaptive_spec())),
        ),
    )
    completed_a: list[CompletedNode] = []
    p.execute([nr_a], completed_a.append, None)
    face_a = part_a.face("")
    assert face_a is not None
    frags_after_first = face_a.cleared.fragments()

    nr_b = NodeRequest(
        key="a",
        generation_id=1,
        stage=StageSpec.Compute(
            part=part_b,
            params=ComputePayload(assembler=Assembler(_adaptive_spec())),
        ),
    )
    completed_b: list[CompletedNode] = []
    p.execute([nr_b], completed_b.append, None)
    face_b = part_b.face("")
    assert face_b is not None
    frags_after_second = face_b.cleared.fragments()

    assert len(frags_after_second) == len(frags_after_first), (
        "cache hit should restore the same number of cleared fragments"
    )


# ── Transformer caching ────────────────────────────────────────────


def test_transformer_change_does_not_auto_invalidate():
    """Adding or changing a transformer does NOT auto-invalidate the
    cache. The cache key is the node key only.
    """
    p = Pipeline()
    spec = _adaptive_spec()
    _run_pipeline(p, [_adaptive_node("a", spec)])
    after_first = p.cache_used_bytes
    second = _run_pipeline(
        p,
        [_adaptive_node("a", spec, transformers=[SmoothSpec(50, 30.0)])],
    )
    assert second["a"].error is None
    assert p.cache_used_bytes == after_first, (
        "transformer change should NOT auto-invalidate"
    )


def test_transformer_change_after_eviction_recomputes():
    """After clear_cache_prefix, adding a transformer causes a fresh
    compute (cache miss)."""
    p = Pipeline()
    spec = _adaptive_spec()
    _run_pipeline(p, [_adaptive_node("a", spec)])
    p.clear_cache_prefix("a")
    assert p.cache_used_bytes == 0
    second = _run_pipeline(
        p,
        [_adaptive_node("a", spec, transformers=[SmoothSpec(50, 30.0)])],
    )
    assert second["a"].error is None
    assert p.cache_used_bytes > 0


def test_different_transformer_types_on_different_keys():
    """Two compute nodes with different transformer types on different
    keys both cache independently."""
    p = Pipeline()
    spec = _adaptive_spec()
    _run_pipeline(
        p,
        [
            _adaptive_node("k1", spec, transformers=[SmoothSpec(50, 30.0)]),
            _adaptive_node("k2", spec, transformers=[OverscanSpec(2.0)]),
        ],
    )
    assert p.cache_used_bytes > 0


# ── All assemblers cache ───────────────────────────────────────────


def test_contour_compute_caches():
    """Contour (non-adaptive) compute nodes also cache. Under the
    old model only AdaptiveClearingSpec opted in; under the new model
    all compute nodes cache uniformly."""
    p = Pipeline()
    _run_pipeline(p, [_contour_node("c")])
    assert p.cache_used_bytes > 0, "contour compute should be cached"


def test_contour_cache_hit_on_second_run():
    """Second run of the same contour key hits cache (no growth)."""
    p = Pipeline()
    _run_pipeline(p, [_contour_node("c")])
    after_first = p.cache_used_bytes
    _run_pipeline(p, [_contour_node("c")])
    assert p.cache_used_bytes == after_first


def test_compute_cache_with_no_transformers_still_works():
    """Smoke test: transformers=[] (the default) works correctly."""
    p = Pipeline()
    out = _run_pipeline(p, [_contour_node("c", transformers=[])])
    assert out["c"].error is None
    assert out["c"].output is not None
