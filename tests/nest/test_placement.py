"""Tests for the Rust placement module (raygeo.nest.placement)."""

from raygeo.nest.collision import any_overlap, is_contained
from raygeo.nest.placement import (
    filter_candidates_multi_resolution,
    find_valid_position,
    generate_bottom_left_candidates,
    generate_grid_candidates,
    generate_perimeter_candidates,
)


class TestIsContained:
    def test_empty_inner(self):
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained([], outer, 10000000) is False

    def test_simple_contained(self):
        inner = [[(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)]]
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained(inner, outer, 10000000) is True

    def test_not_contained(self):
        inner = [[(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)]]
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained(inner, outer, 10000000) is False

    def test_partially_outside(self):
        inner = [[(50.0, 50.0), (150.0, 50.0), (150.0, 150.0), (50.0, 150.0)]]
        outer = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        assert is_contained(inner, outer, 10000000) is False


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


class TestGenerateBottomLeftCandidates:
    def test_basic(self):
        ifp_bounds = (0.0, 0.0, 100.0, 100.0)
        part_bounds = (0.0, 0.0, 10.0, 10.0)
        candidates = generate_bottom_left_candidates(ifp_bounds, part_bounds, 5.0)
        assert len(candidates) > 0
        for x, y in candidates:
            assert x >= 0.0
            assert y >= 0.0
            assert x + 10.0 <= 100.0 + 1e-6
            assert y + 10.0 <= 100.0 + 1e-6

    def test_spacing(self):
        ifp_bounds = (0.0, 0.0, 50.0, 50.0)
        part_bounds = (0.0, 0.0, 10.0, 10.0)
        candidates = generate_bottom_left_candidates(ifp_bounds, part_bounds, 15.0)
        if len(candidates) > 1:
            dx = candidates[1][0] - candidates[0][0]
            dy = candidates[1][1] - candidates[0][1]
            assert dx >= 10.0 or dy >= 10.0


class TestGenerateGridCandidates:
    def test_basic(self):
        ifp_bounds = (0.0, 0.0, 50.0, 50.0)
        part_bounds = (0.0, 0.0, 10.0, 10.0)
        candidates = generate_grid_candidates(ifp_bounds, part_bounds, 10.0)
        assert len(candidates) > 0
        for x, y in candidates:
            assert 0.0 <= x <= 50.0 + 1e-6
            assert 0.0 <= y <= 50.0 + 1e-6


class TestGeneratePerimeterCandidates:
    def test_basic(self):
        placed = [[[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]]
        part_bounds = (0.0, 0.0, 5.0, 5.0)
        candidates = generate_perimeter_candidates(placed, part_bounds, 5.0)
        assert len(candidates) == 8

    def test_no_placed(self):
        candidates = generate_perimeter_candidates([], (0.0, 0.0, 5.0, 5.0), 5.0)
        assert candidates == []


class TestFilterCandidatesMultiResolution:
    def test_basic(self):
        candidates = [(0.0, 0.0), (1.0, 1.0), (10.0, 10.0), (10.1, 10.1)]
        filtered = filter_candidates_multi_resolution(
            candidates, (0.0, 0.0, 20.0, 20.0), 2.0
        )
        assert len(filtered) <= len(candidates)
        assert (0.0, 0.0) in filtered
        assert (10.0, 10.0) in filtered

    def test_all_far_apart(self):
        candidates = [(0.0, 0.0), (100.0, 0.0), (0.0, 100.0)]
        filtered = filter_candidates_multi_resolution(
            candidates, (0.0, 0.0, 100.0, 100.0), 10.0
        )
        assert len(filtered) == 3

    def test_empty(self):
        assert filter_candidates_multi_resolution([], (0.0, 0.0, 1.0, 1.0), 1.0) == []

    def test_zero_dist(self):
        candidates = [(0.0, 0.0), (1.0, 1.0)]
        result = filter_candidates_multi_resolution(
            candidates, (0.0, 0.0, 10.0, 10.0), 0.0
        )
        assert len(result) == 2


class TestFindValidPosition:
    def test_simple_valid(self):
        ifp = [[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]]
        part = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]
        pos = find_valid_position(ifp, part, [], 1.0, 10000000, 1.0)
        assert pos is not None
        x, y = pos
        assert x >= 0.0
        assert y >= 0.0

    def test_no_ifp(self):
        part = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]]
        result = find_valid_position([], part, [], 1.0, 10000000, 1.0)
        assert result is None

    def test_no_part(self):
        ifp = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]]
        result = find_valid_position(ifp, [], [], 1.0, 10000000, 1.0)
        assert result is None

    def test_no_overlap_with_placed(self):
        ifp = [[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]]
        part = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]
        placed = [[(50.0, 50.0), (60.0, 50.0), (60.0, 60.0), (50.0, 60.0)]]
        pos = find_valid_position(ifp, part, placed, 1.0, 10000000, 1.0)
        assert pos is not None
