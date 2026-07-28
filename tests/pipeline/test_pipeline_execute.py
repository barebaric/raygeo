"""
Generic pipeline-execute tests.
"""

from raygeo.pipeline.execute import Pipeline, execute_stages


def test_empty_batch_final_tick_is_one():
    """An empty batch still calls on_batch_progress(1.0, '')."""
    progress: list[tuple[float, str]] = []
    execute_stages([], lambda n: None, lambda f, m: progress.append((f, m)))
    assert progress == [(1.0, "")]


def test_empty_batch_no_completions():
    """An empty batch fires zero on_completed."""
    completed = []
    execute_stages([], lambda n: completed.append(n), lambda f, m: None)
    assert completed == []


def test_empty_batch_no_callback_is_ok():
    """Omitting on_batch_progress for an empty batch works."""
    execute_stages([], lambda n: None, None)


def test_pipeline_construct_default_budget():
    """Pipeline() constructs with the default 256 MiB budget."""
    p = Pipeline()
    assert p.cache_budget_bytes == 2147483648
    assert p.cache_used_bytes == 0


def test_pipeline_construct_custom_budget():
    """Pipeline(budget_bytes=...) sets the budget."""
    p = Pipeline(budget_bytes=1024)
    assert p.cache_budget_bytes == 1024


def test_pipeline_clear_cache_no_error():
    """clear_cache on an empty pipeline is a no-op."""
    p = Pipeline()
    p.clear_cache()
    assert p.cache_used_bytes == 0


def test_pipeline_clear_cache_prefix_no_error():
    """clear_cache_prefix on an empty pipeline is a no-op."""
    p = Pipeline()
    p.clear_cache_prefix("workpiece-1")
    assert p.cache_used_bytes == 0


def test_pipeline_clear_cache_prefix_no_match():
    """clear_cache_prefix with a non-matching prefix is a no-op."""
    p = Pipeline()
    p.clear_cache_prefix("nonexistent")
    assert p.cache_used_bytes == 0
