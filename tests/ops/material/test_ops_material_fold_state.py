"""Fold state and validation tests for raygeo.ops.material."""

import pytest

from raygeo.geo import Matrix
from raygeo.ops.material import VectorEffect, VolumeEffect
from raygeo.ops.material.fold import fold_effects
from raygeo.ops.material.spec import (
    FoldEntry,
    MaterialFoldSpec,
    PrismaticStock,
)

STOCK = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]


def _rect(x, y, w, h):
    return [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]


def _entries():
    return [
        FoldEntry(
            "w2", Matrix.identity(), [VectorEffect([_rect(5, 5, 2, 2)])]
        ),
        FoldEntry(
            "w1", Matrix.identity(), [VectorEffect([_rect(1, 1, 2, 2)])]
        ),
    ]


def test_fold_is_deterministic():
    spec = MaterialFoldSpec(
        stock=PrismaticStock(polygons=STOCK, thickness=3.0),
        entries=_entries(),
    )
    a = fold_effects(spec)
    b = fold_effects(spec)
    assert a.void_polygons == b.void_polygons
    assert a.provenance == b.provenance
    assert a.escalation == b.escalation


def test_state_snapshot_independent_of_input_lists():
    entries = _entries()
    stock = PrismaticStock(polygons=list(STOCK), thickness=3.0)
    spec = MaterialFoldSpec(stock=stock, entries=entries)
    first = fold_effects(spec)
    entries.clear()
    second = fold_effects(spec)
    assert first.void_polygons == second.void_polygons


def test_depth_field_none_in_phase0():
    spec = MaterialFoldSpec(
        stock=PrismaticStock(polygons=STOCK, thickness=3.0),
        entries=_entries(),
    )
    state = fold_effects(spec)
    assert state.depth_field is None


def test_profile_is_prismatic():
    spec = MaterialFoldSpec(
        stock=PrismaticStock(polygons=STOCK, thickness=3.0),
        entries=_entries(),
    )
    assert fold_effects(spec).profile == "prismatic"


def test_empty_stock_rejected():
    with pytest.raises(ValueError):
        fold_effects(
            MaterialFoldSpec(
                stock=PrismaticStock(polygons=[], thickness=3.0),
                entries=[],
            )
        )


def test_zero_thickness_rejected():
    with pytest.raises(ValueError):
        fold_effects(
            MaterialFoldSpec(
                stock=PrismaticStock(polygons=STOCK, thickness=0.0),
                entries=[],
            )
        )


def test_negative_thickness_rejected():
    with pytest.raises(ValueError):
        fold_effects(
            MaterialFoldSpec(
                stock=PrismaticStock(polygons=STOCK, thickness=-1.0),
                entries=[],
            )
        )


def test_volume_effect_escalates_without_crashing():
    state = fold_effects(
        MaterialFoldSpec(
            stock=PrismaticStock(polygons=STOCK, thickness=3.0),
            entries=[
                FoldEntry(
                    "w1",
                    Matrix.identity(),
                    [
                        VolumeEffect(
                            [(0, 0, 0), (1, 0, 0), (0, 1, 0)],
                            [(0, 1, 2)],
                        )
                    ],
                )
            ],
        )
    )
    assert state.escalation == "solid_profile_required"
    assert state.void_polygons == []


def test_mixed_effects_first_escalation_wins():
    state = fold_effects(
        MaterialFoldSpec(
            stock=PrismaticStock(polygons=STOCK, thickness=3.0),
            entries=[
                FoldEntry(
                    "vol",
                    Matrix.identity(),
                    [
                        VolumeEffect(
                            [(0, 0, 0), (1, 0, 0), (0, 1, 0)],
                            [(0, 1, 2)],
                        )
                    ],
                ),
                FoldEntry(
                    "groove",
                    Matrix.identity(),
                    [VectorEffect([_rect(1, 1, 2, 2)], z_from=-1.0)],
                ),
            ],
        )
    )
    assert state.escalation == "solid_profile_required"
    assert state.provenance == ["groove", "vol"]


def test_repr():
    spec = MaterialFoldSpec(
        stock=PrismaticStock(polygons=STOCK, thickness=3.0),
        entries=_entries(),
    )
    state = fold_effects(spec)
    assert "MaterialState" in repr(state)
