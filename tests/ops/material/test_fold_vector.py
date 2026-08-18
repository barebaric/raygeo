"""Vector-effect folding tests for raygeo.ops.material."""

import pytest

from raygeo.geo import Matrix
from raygeo.geo.shape.polygon import get_polygon_area
from raygeo.ops.material import (
    FoldEntry,
    MaterialFoldSpec,
    PrismaticStock,
    VectorEffect,
    fold_effects,
)


def _rect(x, y, w, h):
    return [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]


STOCK = [[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]]
THICKNESS = 3.0


def _fold(entries, stock=STOCK, thickness=THICKNESS, **kwargs):
    spec = MaterialFoldSpec(
        stock=PrismaticStock(polygons=stock, thickness=thickness),
        entries=entries,
        **kwargs,
    )
    return fold_effects(spec)


def _void_area(state):
    return sum(get_polygon_area(p) for p in state.void_polygons)


def test_through_cut_with_none_z_is_void():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [VectorEffect([_rect(10, 10, 30, 30)])],
            )
        ]
    )
    assert len(state.void_polygons) == 1
    assert _void_area(state) == pytest.approx(900.0)


def test_through_cut_with_bottom_z_is_void():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [VectorEffect([_rect(10, 10, 30, 30)], z_to=-3.0)],
            )
        ]
    )
    assert len(state.void_polygons) == 1


def test_partial_depth_is_not_void():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [VectorEffect([_rect(10, 10, 30, 30)], z_to=-1.0)],
            )
        ]
    )
    assert state.void_polygons == []


def test_effect_clipped_to_stock():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [VectorEffect([_rect(90, 50, 30, 30)])],
            )
        ]
    )
    assert _void_area(state) == pytest.approx(300.0)


def test_effect_fully_outside_stock_is_no_void():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [VectorEffect([_rect(200, 200, 30, 30)])],
            )
        ]
    )
    assert state.void_polygons == []


def test_overlapping_voids_union():
    e1 = FoldEntry(
        "w1",
        Matrix.translation(10.0, 10.0),
        [VectorEffect([_rect(0, 0, 30, 30)])],
    )
    e2 = FoldEntry(
        "w2",
        Matrix.translation(30.0, 30.0),
        [VectorEffect([_rect(0, 0, 30, 30)])],
    )
    state = _fold([e2, e1])
    assert len(state.void_polygons) == 1
    assert _void_area(state) == pytest.approx(1700.0)


def test_disjoint_voids_stay_separate():
    e1 = FoldEntry(
        "w1", Matrix.identity(), [VectorEffect([_rect(10, 10, 20, 20)])]
    )
    e2 = FoldEntry(
        "w2", Matrix.identity(), [VectorEffect([_rect(60, 60, 20, 20)])]
    )
    state = _fold([e1, e2])
    assert len(state.void_polygons) == 2


def test_provenance_sorted_and_deduplicated():
    e1 = FoldEntry(
        "w2", Matrix.identity(), [VectorEffect([_rect(0, 0, 5, 5)])]
    )
    e2 = FoldEntry(
        "w1", Matrix.identity(), [VectorEffect([_rect(50, 50, 5, 5)])]
    )
    state = _fold([e2, e1])
    assert state.provenance == ["w1", "w2"]


def test_entry_without_effects_not_in_provenance():
    e1 = FoldEntry("w1", Matrix.identity(), [])
    state = _fold([e1])
    assert state.provenance == []


def test_placement_translates_effect():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.translation(40.0, 50.0),
                [VectorEffect([_rect(0, 0, 10, 10)])],
            )
        ]
    )
    assert len(state.void_polygons) == 1
    xs = sorted(p[0] for p in state.void_polygons[0])
    ys = sorted(p[1] for p in state.void_polygons[0])
    assert xs[0] == pytest.approx(40.0)
    assert ys[0] == pytest.approx(50.0)
    assert _void_area(state) == pytest.approx(100.0)


def test_placement_rotation_maps_vertices():
    effect_square = _rect(40, 40, 10, 10)
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.rotation(90.0, center=(50.0, 50.0)),
                [VectorEffect([effect_square])],
            )
        ]
    )
    assert len(state.void_polygons) == 1
    assert _void_area(state) == pytest.approx(100.0)
    xs = sorted({round(p[0], 6) for p in state.void_polygons[0]})
    ys = sorted({round(p[1], 6) for p in state.void_polygons[0]})
    assert xs == [50.0, 60.0]
    assert ys == [40.0, 50.0]


def test_z_from_below_surface_escalates():
    state = _fold(
        [
            FoldEntry(
                "w1",
                Matrix.identity(),
                [
                    VectorEffect(
                        [_rect(10, 10, 30, 30)], z_from=-1.0, z_to=None
                    )
                ],
            )
        ]
    )
    assert state.escalation == "top_open_violation"
    assert state.void_polygons == []


def test_escalated_fold_still_completes():
    ok = FoldEntry(
        "ok", Matrix.identity(), [VectorEffect([_rect(10, 10, 20, 20)])]
    )
    bad = FoldEntry(
        "bad",
        Matrix.identity(),
        [VectorEffect([_rect(50, 50, 20, 20)], z_from=-2.0, z_to=None)],
    )
    state = _fold([ok, bad])
    assert state.escalation == "top_open_violation"
    assert _void_area(state) == pytest.approx(400.0)
