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

from conftest import (
    make_square_part,
    result_ops,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    MachineParams,
)
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

IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]

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
    cacheable: bool = True,
    version_token: int = 0,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=generation_id,
        version_token=version_token,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(
                assembler=Assembler(spec or ContourSpec()),
                transformers=transformers or [],
            ),
        ),
        cacheable=cacheable,
    )


def _agg_node(
    key: str, source_keys: list[str], version_token: int = 0
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        version_token=version_token,
        stage=StageSpec.Aggregate(
            spec=AggregateSpec(
                wrap_start=[],
                groups=[
                    AggregateGroup(
                        start_markers=[],
                        inputs=[
                            AggregateInput(
                                source_key=sk,
                                placement_matrix=IDENTITY,
                                uid="",
                                target_dimensions=(0.0, 0.0),
                            )
                            for sk in source_keys
                        ],
                        end_markers=[],
                    )
                ],
                wrap_end=[],
                machine=MachineParams(),
                transformers=[],
            )
        ),
    )


def _run_pipeline(p: Pipeline, nodes):
    """Run nodes through the given Pipeline, return completions by key."""
    completed: list[CompletedNode] = []
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


# ── Cache entry sizes reflect real output size ─────────────────────


def test_cache_used_bytes_scales_with_ops_size():
    """Different compute outputs consume different cache bytes,
    proving prepare_cache_entry uses real entry sizes.

    An adaptive clearing node with a complex pocket produces more Ops
    commands than a simple square contour, so its cache entry must be
    larger under the new sizing.
    """
    p = Pipeline()

    contour = _contour_node("contour", ContourSpec())
    p.execute([contour], lambda n: None, None)
    contour_bytes = p.cache_used_bytes
    p.clear_cache()

    adaptive = _adaptive_node("adaptive", AdaptiveClearingSpec())
    p.execute([adaptive], lambda n: None, None)
    adaptive_bytes = p.cache_used_bytes

    assert contour_bytes > 0
    assert adaptive_bytes > 0
    assert adaptive_bytes != contour_bytes, (
        "different compute outputs should produce different cache byte counts"
    )


def test_cache_entry_size_not_hardcoded():
    """The cache entry size is no longer hardcoded to 1024 bytes.

    After running a single compute node, cache_used_bytes reflects the
    real Ops data size rather than the old hardcoded constant.
    Verifying by running the same node twice should produce the same
    byte count (cache hit), and that count should not be a simple
    multiple of 1024 for a single entry.
    """
    p = Pipeline()
    out = _run_pipeline(p, [_contour_node("c", ContourSpec())])
    assert out["c"].error is None
    used = p.cache_used_bytes
    assert used > 0
    # With a single entry the byte count should not be 1024
    # (the old hardcoded constant).
    assert used != 1024, (
        "cache entry size should not be the old hardcoded 1024"
    )


# ── Cache eviction enforces byte budget ──────────────────────────


def test_cache_enforces_budget():
    """When cache entries exceed the byte budget, older entries are
    evicted so that used_bytes never exceeds budget_bytes."""
    # Run enough compute nodes whose total would exceed a tight budget.
    budget = 2000
    p = Pipeline(budget_bytes=budget)
    for i in range(5):
        key = f"n{i}"
        _run_pipeline(p, [_contour_node(key, ContourSpec())])
        assert p.cache_used_bytes <= p.cache_budget_bytes, (
            f"after inserting {key}, used_bytes {p.cache_used_bytes} "
            f"exceeded budget {p.cache_budget_bytes}"
        )


def test_cache_eviction_reduces_usage():
    """Inserting entries beyond the budget causes eviction and reduces
    used_bytes compared to running with no budget limit."""
    # Run with a generous budget (no eviction).
    generous = Pipeline()
    for i in range(3):
        _run_pipeline(generous, [_contour_node(f"g{i}", ContourSpec())])
    generous_bytes = generous.cache_used_bytes

    # Run the same keys with a tight budget (half the footprint, so
    # eviction is guaranteed regardless of per-node struct size).
    budget = max(1, generous_bytes // 2)
    tight = Pipeline(budget_bytes=budget)
    for i in range(3):
        _run_pipeline(tight, [_contour_node(f"t{i}", ContourSpec())])
    tight_bytes = tight.cache_used_bytes

    assert budget < generous_bytes, (
        "tight budget must be smaller than the unconstrained footprint"
    )
    assert tight_bytes <= budget, (
        "cache usage must stay within the tight budget after eviction"
    )
    assert generous_bytes > tight_bytes, (
        "tight budget should evict entries, resulting in less cache usage"
    )


# ── Per-node cacheability ──────────────────────────────────────────


def test_non_cacheable_node_does_not_populate_cache():
    """A node with cacheable=False runs but is never stored in the
    cache."""
    p = Pipeline()
    out = _run_pipeline(
        p, [_contour_node("c", ContourSpec(), cacheable=False)]
    )
    assert out["c"].error is None
    assert out["c"].output is not None
    assert p.cache_used_bytes == 0, (
        "non-cacheable node must not add a cache entry"
    )


def test_non_cacheable_node_reruns_every_time():
    """Re-running a non-cacheable node executes it again instead of
    hitting the cache."""
    p = Pipeline()
    first = _run_pipeline(
        p, [_contour_node("c", ContourSpec(), cacheable=False)]
    )
    assert first["c"].error is None
    assert p.cache_used_bytes == 0
    second = _run_pipeline(
        p, [_contour_node("c", ContourSpec(), cacheable=False)]
    )
    assert second["c"].error is None
    assert p.cache_used_bytes == 0, (
        "non-cacheable node must never grow the cache"
    )


def test_non_cacheable_compute_feeds_cached_aggregate():
    """A non-cacheable compute can still feed a downstream aggregate;
    only its own cache entry is skipped."""
    p = Pipeline()
    nodes = [
        _contour_node("c", ContourSpec(), cacheable=False),
        _agg_node("agg", ["c"]),
    ]
    out = _run_pipeline(p, nodes)
    assert out["c"].error is None
    assert out["agg"].error is None
    assert out["agg"].output is not None
    assert p.cache_used_bytes > 0, (
        "the aggregate's own entry should still be cached"
    )


def test_mixed_cacheability_hits_only_cacheable_entries():
    """With one cacheable and one non-cacheable node of the same key,
    the second run re-executes the non-cacheable node while the
    cacheable entry is reused."""
    p = Pipeline()
    first = _run_pipeline(
        p, [_contour_node("c", ContourSpec(), cacheable=False)]
    )
    assert first["c"].error is None
    assert p.cache_used_bytes == 0
    second = _run_pipeline(
        p, [_contour_node("c", ContourSpec(), cacheable=True)]
    )
    assert second["c"].error is None
    assert p.cache_used_bytes > 0, (
        "cacheable node should populate the cache on the second run"
    )


# ── Non-cacheable node skipping ────────────────────────────────────


def test_non_cacheable_node_skipped_when_token_unchanged():
    """A non-cacheable node whose version token is unchanged since the
    last run is skipped instead of re-executed: its completion carries
    no output."""
    p = Pipeline()
    node = _contour_node("c", ContourSpec(), cacheable=False)
    first = _run_pipeline(p, [node])
    assert first["c"].error is None
    assert first["c"].output is not None

    second = _run_pipeline(p, [node])
    assert second["c"].error is None
    assert second["c"].output is None, (
        "unchanged non-cacheable node should be skipped"
    )
    assert p.cache_used_bytes == 0


def test_non_cacheable_node_reruns_when_token_changes():
    """Changing the version token forces a non-cacheable node to run
    again (and updates the fingerprint)."""
    p = Pipeline()
    spec = ContourSpec()
    first = _run_pipeline(
        p, [_contour_node("c", spec, cacheable=False, version_token=1)]
    )
    assert first["c"].output is not None

    second = _run_pipeline(
        p, [_contour_node("c", spec, cacheable=False, version_token=2)]
    )
    assert second["c"].output is not None, (
        "token change should force a re-run"
    )

    third = _run_pipeline(
        p, [_contour_node("c", spec, cacheable=False, version_token=2)]
    )
    assert third["c"].output is None, (
        "repeated token should be skipped again"
    )


def test_skipped_non_cacheable_feeds_cached_downstream():
    """When a non-cacheable node is skipped, its cacheable dependent
    still cache-hits and produces an output."""
    p = Pipeline()
    nodes = [
        _contour_node("c", ContourSpec(), cacheable=False),
        _agg_node("agg", ["c"]),
    ]
    first = _run_pipeline(p, nodes)
    assert first["c"].output is not None
    assert first["agg"].output is not None
    assert p.cache_used_bytes > 0

    second = _run_pipeline(p, nodes)
    assert second["c"].error is None
    assert second["c"].output is None, (
        "unchanged non-cacheable node should be skipped"
    )
    assert second["agg"].error is None
    assert second["agg"].output is not None, (
        "aggregate should cache-hit even though its dep was skipped"
    )


def test_evicted_downstream_forces_non_cacheable_rerun():
    """If a dependent's cache entry is gone, the non-cacheable node
    must run again even when its token is unchanged."""
    p = Pipeline()
    nodes = [
        _contour_node("c", ContourSpec(), cacheable=False),
        _agg_node("agg", ["c"]),
    ]
    first = _run_pipeline(p, nodes)
    assert first["c"].output is not None

    p.clear_cache_prefix("agg")
    second = _run_pipeline(p, nodes)
    assert second["c"].error is None
    assert second["c"].output is not None, (
        "missing dependent entry must force the node to run"
    )
    assert second["agg"].error is None
    assert second["agg"].output is not None
