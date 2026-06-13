"""Tests for the Rust collision module (raygeo.nest.collision)."""

import numpy as np

from raygeo.nest.collision import (
    any_overlap,
    any_overlap_hierarchical,
    any_overlap_hierarchical_grid,
    is_contained,
)
from raygeo.nest.spatial_grid import SpatialGrid

PN = np.array


def _p(*args):
    """Convert a list of (x,y) pairs or a single polygon into N×2 ndarray."""
    return PN(args)


class TestIsContained:
    def test_empty_inner(self):
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained([], outer) is False

    def test_simple_contained(self):
        inner = [[(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)]]
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained(inner, outer) is True

    def test_not_contained(self):
        inner = [[(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)]]
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained(inner, outer) is False

    def test_partially_outside(self):
        inner = [[(50.0, 50.0), (150.0, 50.0), (150.0, 150.0), (50.0, 150.0)]]
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained(inner, outer) is False


class TestAnyOverlap:
    def test_no_overlap(self):
        placed = [[(50.0, 50.0), (60.0, 50.0), (60.0, 60.0), (50.0, 60.0)]]
        candidate = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        assert any_overlap(candidate, placed, 0.0) is False

    def test_overlap(self):
        placed = [[(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)]]
        candidate = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        assert any_overlap(candidate, placed, 0.0) is True

    def test_empty_candidate(self):
        placed = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]]
        assert any_overlap([], placed, 0.0) is False

    def test_empty_placed(self):
        candidate = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        assert any_overlap(candidate, [], 0.0) is False


class TestAnyOverlapHierarchical:
    """Test the 3-tier hierarchical overlap check."""

    def test_no_overlap_bbox_reject(self):
        cand_polys = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        cand_hulls = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        placed = [
            [PN([(50.0, 50.0), (60.0, 50.0), (60.0, 60.0), (50.0, 60.0)])]
        ]
        placed_hulls = [
            [PN([(50.0, 50.0), (60.0, 50.0), (60.0, 60.0), (50.0, 60.0)])]
        ]
        assert (
            any_overlap_hierarchical(
                cand_polys, cand_hulls, placed, placed_hulls, 0.0
            )
            is False
        )

    def test_overlap_detected(self):
        cand_polys = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        cand_hulls = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        placed = [[PN([(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)])]]
        placed_hulls = [
            [PN([(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)])]
        ]
        assert (
            any_overlap_hierarchical(
                cand_polys, cand_hulls, placed, placed_hulls, 0.0
            )
            is True
        )

    def test_hull_precheck_skips_non_overlapping_concave(self):
        """Concave polygons whose hulls don't intersect should be skipped
        without reaching the detailed check."""
        cand_polys = [
            PN(
                [
                    (0.0, 0.0),
                    (10.0, 0.0),
                    (5.0, 5.0),
                    (10.0, 10.0),
                    (0.0, 10.0),
                ]
            )
        ]
        cand_hulls = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        placed = [
            [
                PN(
                    [
                        (20.0, 20.0),
                        (30.0, 20.0),
                        (25.0, 25.0),
                        (30.0, 30.0),
                        (20.0, 30.0),
                    ]
                )
            ]
        ]
        placed_hulls = [
            [PN([(20.0, 20.0), (30.0, 20.0), (30.0, 30.0), (20.0, 30.0)])]
        ]
        assert (
            any_overlap_hierarchical(
                cand_polys, cand_hulls, placed, placed_hulls, 0.0
            )
            is False
        )

    def test_empty_candidate_polys(self):
        assert (
            any_overlap_hierarchical(
                [],
                [],
                [[PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)])]],
                [],
                0.0,
            )
            is False
        )


class TestAnyOverlapHierarchicalGrid:
    def test_no_overlap_with_grid(self):
        cand_polys = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        cand_hulls = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        placed = [
            [PN([(50.0, 50.0), (60.0, 50.0), (60.0, 60.0), (50.0, 60.0)])]
        ]
        placed_hulls = [
            [PN([(50.0, 50.0), (60.0, 50.0), (60.0, 60.0), (50.0, 60.0)])]
        ]
        grid = SpatialGrid(50.0)
        grid.insert(0, [50.0, 50.0, 60.0, 60.0])
        assert (
            any_overlap_hierarchical_grid(
                cand_polys,
                cand_hulls,
                placed,
                placed_hulls,
                grid,
                (0.0, 0.0, 10.0, 10.0),
                0.0,
            )
            is False
        )

    def test_overlap_with_grid(self):
        cand_polys = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        cand_hulls = [PN([(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)])]
        placed = [[PN([(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)])]]
        placed_hulls = [
            [PN([(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)])]
        ]
        grid = SpatialGrid(50.0)
        grid.insert(0, [5.0, 5.0, 15.0, 15.0])
        assert (
            any_overlap_hierarchical_grid(
                cand_polys,
                cand_hulls,
                placed,
                placed_hulls,
                grid,
                (0.0, 0.0, 15.0, 15.0),
                0.0,
            )
            is True
        )
