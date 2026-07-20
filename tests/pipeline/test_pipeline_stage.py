"""
Generic pipeline-stage tests.

Tests the Python-visible wire surface types and the execute_stages
dispatch machinery without referencing any domain assembler/encoder.
"""

from raygeo.pipeline.cache import CacheKey
from raygeo.pipeline.execute import execute_stages
from raygeo.pipeline.stage import StageSpec


def test_register_submodules():
    import raygeo.pipeline as pl

    assert hasattr(pl, "stage")
    assert hasattr(pl, "request")
    assert hasattr(pl, "completed")
    assert hasattr(pl, "execute")
    assert hasattr(pl, "cache")


def test_stage_spec_variants_exist():
    assert hasattr(StageSpec, "Compute")
    assert hasattr(StageSpec, "Aggregate")


def test_cache_key_roundtrip():
    key = CacheKey("test-tag", 42)
    assert key.tag == "test-tag"
    assert key.payload_hash == 42


# ── execute_stages — empty batch ───────────────────────────────────


def test_empty_batch_emits_final_progress():
    batch: list[tuple[float, str]] = []

    def on_batch(frac: float, msg: str) -> None:
        batch.append((frac, msg))

    execute_stages([], lambda n: None, on_batch)
    assert batch == [(1.0, "")]


def test_empty_batch_no_completions():
    completed: list = []
    execute_stages([], lambda n: completed.append(n), None)
    assert completed == []


# ── execute_stages — error handling ────────────────────────────────


def test_batch_progress_required_ok():
    execute_stages([], lambda n: None, None)


def test_nested_execute_empty():
    inner: list = []

    def outer_cb(n):
        execute_stages([], lambda n2: inner.append(n2))

    execute_stages([], outer_cb)
    assert len(inner) == 0
