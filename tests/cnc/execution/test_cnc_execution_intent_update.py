"""Tests for the Intent mutation API (R3).

Exercises Intent.update (diff-based cache invalidation) and
Intent.invalidate (manual eviction), including transitive dependent
propagation and cache survival for unchanged nodes.
"""

from conftest import make_square_part

from raygeo.cnc.execution.intent import (
    create_intent_from_nodes,
    run_intent,
)
from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    MachineParams,
)
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.pipeline.execute import Pipeline
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

IDENTITY_4X4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _compute_node(
    key: str = "c1",
    token: int = 0,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        version_token=token,
        stage=StageSpec.Compute(
            part=make_square_part(),
            params=ComputePayload(
                assembler=Assembler(ContourSpec()),
            ),
        ),
    )


def _aggregate_node(
    source_key: str = "c1",
    key: str = "agg",
    token: int = 0,
) -> NodeRequest:
    agg_input = AggregateInput(
        source_key=source_key,
        placement_matrix=IDENTITY_4X4,
    )
    group = AggregateGroup(
        start_markers=[],
        inputs=[agg_input],
        end_markers=[],
    )
    spec = AggregateSpec(
        wrap_start=[],
        groups=[group],
        wrap_end=[],
        machine=MachineParams(),
    )
    return NodeRequest(
        key=key,
        generation_id=1,
        version_token=token,
        stage=StageSpec.Aggregate(spec=spec),
    )


def _make_intent(compute_token=0, agg_token=0):
    nodes = [
        _compute_node(token=compute_token),
        _aggregate_node(token=agg_token),
    ]
    return create_intent_from_nodes(nodes)


# ── update: no-change ──────────────────────────────────────────────


def test_update_no_change_cache_survives():
    """update with identical version_tokens keeps cache entries."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    after_first = p.cache_used_bytes
    assert after_first > 0

    fresh = _make_intent(compute_token=1, agg_token=1)
    intent.update(fresh, pipeline=p)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes == after_first, (
        "identical tokens should not invalidate cache"
    )


# ── update: changed token ──────────────────────────────────────────


def test_update_changed_token_evicts_compute():
    """Changing the compute token evicts its cache entry; aggregate
    is also evicted because it depends on compute."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    after_first = p.cache_used_bytes

    fresh = _make_intent(compute_token=2, agg_token=1)
    intent.update(fresh, pipeline=p)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes == after_first, (
        "changed compute should evict compute + aggregate; "
        "re-run repopulates to same size"
    )


def test_update_changed_token_only_aggregate():
    """Changing only the aggregate token evicts only the aggregate
    entry; the compute entry survives (cache hit)."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    after_first = p.cache_used_bytes

    fresh = _make_intent(compute_token=1, agg_token=2)
    intent.update(fresh, pipeline=p)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes == after_first, (
        "aggregate re-run after eviction should repopulate"
    )


# ── update: removed key ────────────────────────────────────────────


def test_update_removed_key_evicts():
    """Removing a compute node from the intent evicts its cache."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes > 0

    fresh = create_intent_from_nodes([_aggregate_node(token=1)])
    intent.update(fresh, pipeline=p)
    assert p.cache_used_bytes == 0, (
        "removed compute node's cache entry should have been evicted"
    )


# ── update: added key ──────────────────────────────────────────────


def test_update_added_key_runs():
    """Adding a new compute node does not crash; the node runs on
    the next run_intent."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes > 0

    fresh = create_intent_from_nodes(
        [
            _compute_node("c1", token=1),
            _compute_node("c2", token=1),
            _aggregate_node(token=1),
        ]
    )
    intent.update(fresh, pipeline=p)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes > 0


# ── invalidate ─────────────────────────────────────────────────────


def test_invalidate_evicts_specified_key():
    """invalidate('c1') evicts c1's cache entry."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes > 0

    fresh = _make_intent(compute_token=1, agg_token=1)
    intent.update(fresh, pipeline=p)
    intent.invalidate(["c1"], pipeline=p)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes > 0


def test_invalidate_propagates_to_dependents():
    """invalidating 'c1' also evicts the aggregate (its dependent)."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    after_first = p.cache_used_bytes

    fresh = _make_intent(compute_token=1, agg_token=1)
    intent.update(fresh, pipeline=p)
    intent.invalidate(["c1"], pipeline=p)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes == after_first, (
        "invalidate(c1) evicts c1 + agg; re-run repopulates both"
    )


def test_invalidate_aggregate_only_keeps_compute():
    """invalidating 'agg' evicts only the aggregate; compute survives."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)

    fresh = _make_intent(compute_token=1, agg_token=1)
    intent.update(fresh, pipeline=p)
    intent.invalidate(["agg"], pipeline=p)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes > 0


# ── run_intent with explicit pipeline ──────────────────────────────


def test_run_intent_uses_pipeline_cache():
    """run_intent with an explicit Pipeline writes to that pipeline's
    cache, not the default."""
    p = Pipeline()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent, pipeline=p)
    assert p.cache_used_bytes > 0

    p2 = Pipeline()
    assert p2.cache_used_bytes == 0, (
        "second pipeline should not share first's cache"
    )


def test_update_works_with_default_pipeline():
    """update without a pipeline argument uses the default cache."""
    from raygeo.pipeline.execute import clear_cache

    clear_cache()
    intent = _make_intent(compute_token=1, agg_token=1)
    run_intent(intent)

    fresh = _make_intent(compute_token=2, agg_token=1)
    intent.update(fresh)
    run_intent(intent)
