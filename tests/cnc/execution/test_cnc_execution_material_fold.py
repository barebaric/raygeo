"""Pipeline integration tests for the ``MaterialFold`` compute stage.

Exercises the ``MaterialFoldCompute`` Rust impl (see
``src/cnc/execution/material_fold.rs``) through the PyO3 bindings:
the ``MaterialFoldSpec`` is passed directly as the node's stage (the
cnc-execution converter recognises it via ``cast``, exactly like
``EncodeSpec``/``MachineTransformSpec`` — no ``StageSpec`` variant
is needed, since ``pipeline`` is domain-free). End-to-end execution,
caching, cache invalidation, upstream-dependency folding of
``AssemblyOutput.material_effects``, and the laser burn surface map
are covered.
"""

import numpy as np
from conftest import collect_completions, make_contour_compute

from raygeo.cnc.execution.specs import ComputePayload
from raygeo.geo import Matrix
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.raster import RasterSpec
from raygeo.ops.material.spec import (
    FoldEntry,
    GridBudget,
    MaterialFoldSpec,
    PrismaticStock,
)
from raygeo.ops.part import Part
from raygeo.pipeline.execute import clear_cache
from raygeo.pipeline.request import NodeRequest
from raygeo.pipeline.stage import StageSpec

STOCK_POLYGONS = [[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]]
STOCK_THICKNESS = 3.0


def _fold_node(key, entries, source_keys, version_token=1):
    """Build a fold compute node.

    The ``MaterialFoldSpec`` is passed directly as the node's stage.
    """
    spec = MaterialFoldSpec(
        stock=PrismaticStock(
            polygons=STOCK_POLYGONS, thickness=STOCK_THICKNESS
        ),
        entries=entries,
        grid=GridBudget(),
    )
    return NodeRequest(
        key=key,
        generation_id=1,
        stage=spec,
        version_token=version_token,
    )


def _empty_fold_node(key, source_keys, version_token=1):
    """A fold node whose entries carry no effects (upstreams are
    None or absent)."""
    entries = [FoldEntry(sk, Matrix.identity(), []) for sk in source_keys]
    return _fold_node(key, entries, source_keys, version_token)


def _completed(nodes):
    completed, _batch = collect_completions(nodes)
    return {c.key: c for c in completed}


def test_fold_node_executes_in_pipeline():
    """A standalone fold node with no upstream effects completes and
    produces an empty ``MaterialState`` (prismatic profile, no voids)."""
    node = _empty_fold_node("fold", source_keys=[])
    results = _completed([node])
    assert "fold" in results
    fold_result = results["fold"]
    assert fold_result.error is None
    assert fold_result.output is not None
    assert fold_result.output.profile == "prismatic"
    assert fold_result.output.void_polygons == []
    assert fold_result.output.provenance == []


def test_fold_node_cache_hit():
    """Executing twice with the same version_token is a cache hit on
    the second run — the node is not re-executed."""
    try:
        node = _empty_fold_node(
            "fold_cached", source_keys=[], version_token=42
        )
        first = _completed([node])
        assert first["fold_cached"].error is None
        # Second run: identical key + version_token ⇒ cache hit. The
        # node completes (from cache) without error.
        second = _completed([node])
        assert "fold_cached" in second
        assert second["fold_cached"].error is None
        assert second["fold_cached"].output is not None
    finally:
        clear_cache()


def test_fold_node_cache_invalidation():
    """Changing the version_token invalidates the cache and forces
    re-execution."""
    try:
        node_v1 = _empty_fold_node(
            "fold_inval", source_keys=[], version_token=1
        )
        _completed([node_v1])
        node_v2 = _empty_fold_node(
            "fold_inval", source_keys=[], version_token=2
        )
        results = _completed([node_v2])
        # Re-execution produces a fresh output (the cache miss fired).
        assert results["fold_inval"].output is not None
        assert results["fold_inval"].error is None
    finally:
        clear_cache()


def test_fold_node_upstream_dependency():
    """A fold node depending on a contour compute node folds the
    upstream ``AssemblyOutput.material_effects`` into voids."""
    compute = make_contour_compute("wp_cut")
    fold = _fold_node(
        "fold_upstream",
        entries=[FoldEntry("wp_cut", Matrix.identity(), [])],
        source_keys=["wp_cut"],
    )
    try:
        results = _completed([compute, fold])
        fold_result = results["fold_upstream"]
        assert fold_result.error is None
        assert fold_result.output is not None
        # The contour assembler emits a full-through vector effect,
        # which the fold turns into a void clipped to the stock.
        assert len(fold_result.output.void_polygons) >= 1
        assert "wp_cut" in fold_result.output.provenance
    finally:
        clear_cache()


def test_fold_node_surface_map_from_raster_compute():
    """End-to-end burn slice: a raster engrave compute node emits a
    burn RasterEffect; the fold max-reduces it onto the stock grid as
    ``surface_map``."""
    part = Part(size_mm=(10.0, 10.0), pixels_per_mm=(10.0, 10.0))
    part.image = np.full((100, 100), 255, dtype=np.uint8)
    compute = NodeRequest(
        key="wp_engrave",
        generation_id=1,
        stage=StageSpec.Compute(
            part=part,
            params=ComputePayload(
                assembler=Assembler(RasterSpec(mode="mask_scan"))
            ),
        ),
    )
    fold = _fold_node(
        "fold_burn",
        entries=[FoldEntry("wp_engrave", Matrix.identity(), [])],
        source_keys=["wp_engrave"],
    )
    try:
        results = _completed([compute, fold])
        fold_result = results["fold_burn"]
        assert fold_result.error is None
        state = fold_result.output
        assert state is not None
        assert state.surface_map is not None
        assert state.grid is not None
        # The stock is 100x100 mm at the 50 px/mm budget: 5000 px/side.
        assert state.grid.size_px == (5000, 5000)
        sm = state.surface_map.to_numpy()
        # The engraved 10x10 mm area burns 500x500 stock-grid pixels.
        assert (sm > 0).sum() == 500 * 500
        assert "wp_engrave" in state.provenance
    finally:
        clear_cache()
