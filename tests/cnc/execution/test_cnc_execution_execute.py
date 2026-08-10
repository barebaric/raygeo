"""Tests for the per-batch aggregate progress callback and topology.

`execute_stages` accepts an optional `on_batch_progress(frac, msg)`
callback that reports an aggregate fraction for the whole batch:

```
frac = (completed + sum(in_flight_fracs)) / total
```

These tests verify the math: that an empty batch completes at 1.0,
that a single-node batch jumps cleanly to 1.0, that a multi-node
batch increments proporionally, and that the final tick is always
exactly 1.0.

Topology, throughput, and cache integration tests are also here.
"""

import time

import pytest
from conftest import (
    aggregate_result,
    collect_completions,
    encode_result,
    make_contour_compute,
    make_square_part,
    result_ops,
)

from raygeo.cnc.execution.specs import (
    AggregateGroup,
    AggregateInput,
    AggregateSpec,
    ComputePayload,
    EncodeSpec,
    MachineParams,
)
from raygeo.geo import Geometry
from raygeo.ops import Ops
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.convert import Encoder, GcodeDialectSpec, GcodeSpec
from raygeo.ops.part import Part
from raygeo.ops.transform.overscan import OverscanSpec
from raygeo.pipeline.completed import CompletedNode
from raygeo.pipeline.execute import Pipeline, execute_stages
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

# ── Single-node batch ─────────────────────────────────────────────


def test_single_node_batch_ends_at_one():
    _, batch = collect_completions([make_contour_compute("k1")], on_batch=True)
    assert batch[-1][0] == pytest.approx(1.0)


def test_single_node_batch_frac_clamped_to_one():
    _, batch = collect_completions([make_contour_compute("k1")], on_batch=True)
    for frac, _ in batch:
        assert 0.0 <= frac <= 1.0


# ── Multi-node ────────────────────────────────────────────────────


def test_multi_node_batch_increments():
    nodes = [make_contour_compute(f"k{i}") for i in range(4)]
    _, batch = collect_completions(nodes, on_batch=True)
    fracs = [f for f, _ in batch]
    assert fracs[-1] == pytest.approx(1.0)
    assert max(fracs) >= 0.5
    assert min(fracs) >= 0.0


def test_multi_node_batch_fires_on_completions():
    nodes = [make_contour_compute(f"k{i}") for i in range(8)]
    _, batch = collect_completions(nodes, on_batch=True)
    assert len(batch) >= 8
    assert batch[-1][0] == pytest.approx(1.0)


# ── Status message handling ────────────────────────────────────────


def test_batch_status_message_is_str():
    _, batch = collect_completions([make_contour_compute("k1")], on_batch=True)
    for _, msg in batch:
        assert isinstance(msg, str)


def test_batch_status_message_carries_key_then_completion_marker():
    _, batch = collect_completions([make_contour_compute("k1")], on_batch=True)
    messages = [m for _, m in batch]
    assert any(m.startswith("k1") for m in messages), (
        f"expected node key in messages: {messages}"
    )
    assert "\tk1" in messages, f"expected completion marker: {messages}"


def test_batch_status_message_carries_transformer_detail():
    nodes = [
        NodeRequest(
            key="k1",
            generation_id=1,
            stage=StageSpec.Compute(
                part=make_square_part(),
                params=ComputePayload(
                    assembler=Assembler(ContourSpec()),
                    transformers=[OverscanSpec(distance_mm=0.1)],
                ),
            ),
        )
    ]
    _, batch = collect_completions(nodes, on_batch=True)
    messages = [m for _, m in batch]
    assert "k1\toverscan" in messages, (
        f"expected transformer detail in messages: {messages}"
    )


# ── Cancellation ──────────────────────────────────────────────────


def test_batch_progress_fires_even_on_cancellation():
    collected = []
    batch = []
    try:
        execute_stages(
            [make_contour_compute("k1", on_cancelled=lambda: True)],
            collected.append,
            lambda f, m: batch.append((f, m)),
        )
    except RuntimeError:
        pass
    assert batch[-1][0] == pytest.approx(1.0)


# ── Optional callback is None ─────────────────────────────────────


def test_batch_progress_can_be_none_for_nonempty_batch():
    collected = []
    execute_stages(
        [make_contour_compute("k1")],
        collected.append,
        None,
    )
    assert len(collected) == 1


# ── Cache tests ──────────────────────────────────────────────────


def _make_part() -> Part:
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.line_to(0, 0)
    return Part(geometry=g, size_mm=(10.0, 10.0))


def _contour_request(key: str, spec=None) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=StageSpec.Compute(
            part=_make_part(),
            params=ComputePayload(assembler=Assembler(spec or ContourSpec())),
        ),
    )


def _collect(pipeline, nodes):
    completed = []
    pipeline.execute(nodes, completed.append, None)
    return completed


# ── Pipeline independence ─────────────────────────────────────────


def test_pipeline_instances_have_independent_caches():
    p1 = Pipeline()
    p2 = Pipeline()
    completed1 = _collect(p1, [_contour_request("k1")])
    completed2 = _collect(p2, [_contour_request("k1")])
    assert len(completed1) == 1
    assert len(completed2) == 1
    assert p1.cache_used_bytes > 0
    assert p2.cache_used_bytes > 0
    p1.clear_cache()
    assert p1.cache_used_bytes == 0
    assert p2.cache_used_bytes > 0


def test_default_execute_stages_still_works():
    completed = []
    execute_stages([_contour_request("bare-1")], completed.append, None)
    assert len(completed) == 1
    assert completed[0].error is None
    assert isinstance(completed[0].output.ops, Ops)


# ── clear_cache between runs is observable ────────────────────────


def test_clear_cache_is_idempotent():
    p = Pipeline()
    _collect(p, [_contour_request("k1")])
    p.clear_cache()
    p.clear_cache()
    assert p.cache_used_bytes == 0


def test_clear_cache_prefix_with_empty_string_no_op():
    p = Pipeline()
    p.clear_cache_prefix("")
    assert p.cache_used_bytes == 0


# ── Dependency-map tests ─────────────────────────────────────────


IDENTITY = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
]


def _agg(
    key: str,
    source_keys,
) -> NodeRequest:
    return NodeRequest(
        key=key,
        generation_id=1,
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
            )
        ),
    )


def _by_key(completed):
    return {c.key: c for c in completed}


# ── Single-source ─────────────────────────────────────────────────


def test_single_source_topology_completes_both():
    src = make_contour_compute("src")
    agg = _agg("agg", ["src"])
    completed, _ = collect_completions([src, agg])
    by_key = _by_key(completed)
    assert by_key["src"].output is not None
    assert by_key["agg"].output is not None


def test_single_source_aggregate_consumes_compute_ops():
    src = make_contour_compute("src")
    agg = _agg("agg", ["src"])
    completed, _ = collect_completions([src, agg])
    out = aggregate_result(_by_key(completed)["agg"])
    assert len(out.ops) == 6


# ── Multi-source ──────────────────────────────────────────────────


def test_multi_source_topology_completes_all():
    a = make_contour_compute("a")
    b = make_contour_compute("b")
    agg = _agg("agg", ["a", "b"])
    completed, _ = collect_completions([a, b, agg])
    keys = {c.key for c in completed}
    assert keys == {"a", "b", "agg"}
    assert all(c.error is None for c in completed)


def test_multi_source_aggregate_concatenates_ops():
    a = make_contour_compute("a")
    b = make_contour_compute("b")
    agg = _agg("agg", ["a", "b"])
    completed, _ = collect_completions([a, b, agg])
    a_ops = result_ops(_by_key(completed)["a"]).to_dict()
    b_ops = result_ops(_by_key(completed)["b"]).to_dict()
    agg_ops = result_ops(_by_key(completed)["agg"]).to_dict()
    assert agg_ops["commands"] == a_ops["commands"] + b_ops["commands"]


def test_aggregate_over_three_sources():
    srcs = [make_contour_compute(f"s{i}") for i in range(3)]
    agg = _agg("agg", ["s0", "s1", "s2"])
    completed, _ = collect_completions(srcs + [agg])
    out = aggregate_result(_by_key(completed)["agg"])
    assert len(out.ops) == 18


# ── Chain ─────────────────────────────────────────────────────────


def test_chain_topology_compute_agg_agg():
    src = make_contour_compute("src")
    inner = _agg("inner", ["src"])
    outer = _agg("outer", ["inner"])
    completed, _ = collect_completions([src, inner, outer])
    by_key = _by_key(completed)
    for k in ("src", "inner", "outer"):
        out = by_key[k].output
        assert out is not None
        assert by_key[k].error is None
        assert len(result_ops(by_key[k])) == 6


# ── Encode on top of aggregate ────────────────────────────────────


def test_encode_on_aggregate_dependency():
    src = make_contour_compute("src")
    agg = _agg("agg", ["src"])
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="agg",
            encoder=Encoder(
                GcodeSpec(
                    dialect=GcodeDialectSpec(),
                    context_json="{}",
                )
            ),
        ),
    )
    completed, _ = collect_completions([src, agg, enc])
    by_key = _by_key(completed)
    assert by_key["enc"].error is None
    assert encode_result(by_key["enc"]).variant == "MachineCode"


def test_encode_on_compute_dependency():
    src = make_contour_compute("src")
    enc = NodeRequest(
        key="enc",
        generation_id=1,
        stage=EncodeSpec(
            source_key="src",
            encoder=Encoder(
                GcodeSpec(
                    dialect=GcodeDialectSpec(),
                    context_json="{}",
                )
            ),
        ),
    )
    completed, _ = collect_completions([src, enc])
    assert encode_result(_by_key(completed)["enc"]).variant == "MachineCode"


# ── External (out-of-batch) source keys ───────────────────────────


def test_external_source_key_yields_missing_dependency_error():
    agg = _agg("agg", ["external-src"])
    completed, _ = collect_completions([agg])
    c = _by_key(completed)["agg"]
    assert c.error is not None
    assert "external-src" in c.error


def test_partial_external_source_in_topology():
    in_batch = make_contour_compute("in-batch")
    agg = _agg("agg", ["in-batch", "out-of-batch"])
    completed, _ = collect_completions([in_batch, agg])
    c = _by_key(completed)["agg"]
    assert c.error is not None
    assert "out-of-batch" in c.error


# ── Diamond topology ───────────────────────────────────────────────


def test_diamond_topology():
    a = make_contour_compute("a")
    left = _agg("left", ["a"])
    right = _agg("right", ["a"])
    agg = _agg("agg", ["left", "right"])
    completed, _ = collect_completions([a, left, right, agg])
    keys = {c.key for c in completed}
    assert keys == {"a", "left", "right", "agg"}
    assert all(c.error is None for c in completed)
    out = aggregate_result(_by_key(completed)["agg"])
    assert len(out.ops) == 12


# ── Single-source identity properties ─────────────────────────────


def test_identity_placement_preserves_ops():
    src = make_contour_compute("src")
    agg = _agg("agg", ["src"])
    completed, _ = collect_completions([src, agg])
    src_ops = result_ops(_by_key(completed)["src"]).to_dict()["commands"]
    agg_ops = result_ops(_by_key(completed)["agg"]).to_dict()["commands"]
    assert src_ops == agg_ops


# ── GIL-throughput tests ─────────────────────────────────────────


MIN_THROUGHPUT_NODES_PER_SEC = 1000


def _build_batch(n: int) -> list[NodeRequest]:
    spec = ContourSpec()
    assembler = Assembler(spec)
    nodes = []
    for i in range(n):
        nodes.append(
            NodeRequest(
                key=f"leaf-{i}",
                generation_id=1,
                stage=StageSpec.Compute(
                    part=make_square_part(),
                    params=ComputePayload(assembler=assembler),
                ),
            )
        )
    return nodes


def test_2000_node_batch_completes_all():
    nodes = _build_batch(2000)
    completed: list[CompletedNode] = []
    execute_stages(nodes, completed.append, None)
    assert len(completed) == 2000
    assert all(c.error is None for c in completed)
    keys = {c.key for c in completed}
    assert len(keys) == 2000


def test_2000_node_batch_throughput_floor():
    n = 2000
    nodes = _build_batch(n)
    completed: list[CompletedNode] = []
    t0 = time.perf_counter()
    execute_stages(nodes, completed.append, None)
    dt = time.perf_counter() - t0
    throughput = n / dt
    assert throughput >= MIN_THROUGHPUT_NODES_PER_SEC, (
        f"throughput regression: {throughput:.1f} nodes/s "
        f"< {MIN_THROUGHPUT_NODES_PER_SEC} nodes/s "
        f"(n={n}, dt={dt:.3f}s)"
    )


def test_2000_node_batch_with_batch_progress_fires():
    n = 2000
    nodes = _build_batch(n)
    completed: list[CompletedNode] = []
    batch_ticks: list[tuple[float, str]] = []
    execute_stages(
        nodes, completed.append, lambda f, m: batch_ticks.append((f, m))
    )
    assert len(batch_ticks) >= n
    assert batch_ticks[-1][0] == pytest.approx(1.0)


def test_2000_node_batch_under_60_seconds():
    nodes = _build_batch(2000)
    completed: list[CompletedNode] = []
    t0 = time.perf_counter()
    execute_stages(nodes, completed.append, None)
    dt = time.perf_counter() - t0
    assert dt < 60.0, f"wall-clock regression: {dt:.1f}s"


def test_each_completion_carries_correct_key():
    nodes = _build_batch(2000)
    completed: list[CompletedNode] = []
    execute_stages(nodes, completed.append, None)
    expected = {f"leaf-{i}" for i in range(2000)}
    actual = {c.key for c in completed}
    assert actual == expected


def test_each_completion_carries_correct_generation_id():
    nodes = _build_batch(2000)
    completed: list[CompletedNode] = []
    execute_stages(nodes, completed.append, None)
    assert all(c.generation_id == 1 for c in completed)


def test_500_node_subsample_completes():
    nodes = _build_batch(500)
    completed: list[CompletedNode] = []
    execute_stages(nodes, completed.append, None)
    assert len(completed) == 500
