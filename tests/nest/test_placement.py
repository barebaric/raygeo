"""Tests for the Rust placement module (raygeo.nest.placement)."""

import math

import numpy as np

from raygeo.nest.collision import any_overlap, is_contained
from raygeo.nest.placement import (
    calculate_fitness,
    filter_candidates_multi_resolution,
    find_valid_position,
    find_valid_position_nfp,
    find_valid_position_scored,
    generate_bottom_left_candidates,
    generate_grid_candidates,
    generate_perimeter_candidates,
    place_parts,
)
from raygeo.nest.spatial_grid import SpatialGrid


def _grid():
    return SpatialGrid(100.0)


def _square(x1, y1, x2, y2):
    return [(x1, y1), (x2, y1), (x2, y2), (x1, y2)]


class TestIsContained:
    def test_empty_inner(self):
        outer = _square(0.0, 0.0, 100.0, 100.0)
        assert is_contained([], outer, 10000000) is False

    def test_simple_contained(self):
        inner = [_square(10.0, 10.0, 20.0, 20.0)]
        outer = _square(0.0, 0.0, 100.0, 100.0)
        assert is_contained(inner, outer, 10000000) is True

    def test_not_contained(self):
        inner = [_square(-10.0, -10.0, 10.0, 10.0)]
        outer = _square(0.0, 0.0, 100.0, 100.0)
        assert is_contained(inner, outer, 10000000) is False

    def test_partially_outside(self):
        inner = [_square(50.0, 50.0, 150.0, 150.0)]
        outer = _square(0.0, 0.0, 100.0, 100.0)
        assert is_contained(inner, outer, 10000000) is False


class TestAnyOverlap:
    def test_no_overlap(self):
        placed = [_square(50.0, 50.0, 60.0, 60.0)]
        candidate = _square(0.0, 0.0, 10.0, 10.0)
        assert any_overlap(candidate, placed, 0.0) is False

    def test_overlap(self):
        placed = [_square(5.0, 5.0, 15.0, 15.0)]
        candidate = _square(0.0, 0.0, 10.0, 10.0)
        assert any_overlap(candidate, placed, 0.0) is True

    def test_empty_candidate(self):
        placed = [_square(0.0, 0.0, 10.0, 10.0)]
        assert any_overlap([], placed, 0.0) is False

    def test_empty_placed(self):
        candidate = _square(0.0, 0.0, 10.0, 10.0)
        assert any_overlap(candidate, [], 0.0) is False


class TestGenerateBottomLeftCandidates:
    def test_basic(self):
        ifp_bounds = (0.0, 0.0, 100.0, 100.0)
        part_bounds = (0.0, 0.0, 10.0, 10.0)
        candidates = generate_bottom_left_candidates(
            ifp_bounds, part_bounds, 5.0
        )
        assert len(candidates) > 0
        for x, y in candidates:
            assert x >= 0.0
            assert y >= 0.0
            assert x + 10.0 <= 100.0 + 1e-6
            assert y + 10.0 <= 100.0 + 1e-6

    def test_spacing(self):
        ifp_bounds = (0.0, 0.0, 50.0, 50.0)
        part_bounds = (0.0, 0.0, 10.0, 10.0)
        candidates = generate_bottom_left_candidates(
            ifp_bounds, part_bounds, 15.0
        )
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
        placed = [[_square(0.0, 0.0, 10.0, 10.0)]]
        part_bounds = (0.0, 0.0, 5.0, 5.0)
        candidates = generate_perimeter_candidates(placed, part_bounds, 5.0)
        assert len(candidates) == 8

    def test_no_placed(self):
        candidates = generate_perimeter_candidates(
            [], (0.0, 0.0, 5.0, 5.0), 5.0
        )
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
        assert (
            filter_candidates_multi_resolution([], (0.0, 0.0, 1.0, 1.0), 1.0)
            == []
        )

    def test_zero_dist(self):
        candidates = [(0.0, 0.0), (1.0, 1.0)]
        result = filter_candidates_multi_resolution(
            candidates, (0.0, 0.0, 10.0, 10.0), 0.0
        )
        assert len(result) == 2


class TestFindValidPosition:
    def test_simple_valid(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        pos = find_valid_position(ifp, part, [], [], [], _grid(), (0.0, 0.0))
        assert pos is not None
        x, y = pos
        assert x >= 0.0
        assert y >= 0.0

    def test_no_ifp(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        result = find_valid_position([], part, [], [], [], _grid(), (0.0, 0.0))
        assert result is None

    def test_no_part(self):
        ifp = [_square(0.0, 0.0, 10.0, 10.0)]
        result = find_valid_position(ifp, [], [], [], [], _grid(), (0.0, 0.0))
        assert result is None

    def test_no_overlap_with_placed(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        placed = [[_square(50.0, 50.0, 60.0, 60.0)]]
        pos = find_valid_position(
            ifp, part, [], placed, [], _grid(), (0.0, 0.0)
        )
        assert pos is not None
        x, y = pos
        assert not (50.0 <= x <= 60.0 and 50.0 <= y <= 60.0)

    def test_with_hulls(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        hulls = [_square(0.0, 0.0, 10.0, 10.0)]  # same as polygon
        pos = find_valid_position(
            ifp, part, hulls, [], [], _grid(), (0.0, 0.0)
        )
        assert pos is not None

    def test_with_world_offset(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        pos = find_valid_position(ifp, part, [], [], [], _grid(), (50.0, 50.0))
        assert pos is not None
        x, y = pos
        assert x >= 50.0
        assert y >= 50.0


class TestFindValidPositionScored:
    def test_simple_valid(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        pos = find_valid_position_scored(
            ifp, part, [], [], [], _grid(), (0.0, 0.0)
        )
        assert pos is not None
        x, y = pos
        assert x >= 0.0
        assert y >= 0.0

    def test_empty_ifp(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        assert (
            find_valid_position_scored(
                [], part, [], [], [], _grid(), (0.0, 0.0)
            )
            is None
        )

    def test_empty_part(self):
        ifp = [_square(0.0, 0.0, 10.0, 10.0)]
        assert (
            find_valid_position_scored(
                ifp, [], [], [], [], _grid(), (0.0, 0.0)
            )
            is None
        )

    def test_with_placed_parts(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        placed = [[_square(50.0, 50.0, 60.0, 60.0)]]
        pos = find_valid_position_scored(
            ifp, part, [], placed, [], _grid(), (0.0, 0.0)
        )
        assert pos is not None

    def test_with_hulls(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        pos = find_valid_position_scored(
            ifp, part, part, [], [], _grid(), (0.0, 0.0)
        )
        assert pos is not None

    def test_custom_curve_tolerance(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        pos = find_valid_position_scored(
            ifp,
            part,
            [],
            [],
            [],
            _grid(),
            (0.0, 0.0),
            curve_tolerance=2.0,
        )
        assert pos is not None


class TestFindValidPositionNfp:
    def test_simple_valid(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        placed = [[_square(50.0, 50.0, 60.0, 60.0)]]
        pos = find_valid_position_nfp(
            ifp, part, [], placed, [], _grid(), (0.0, 0.0)
        )
        assert pos is not None

    def test_empty_ifp(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        placed = [[_square(50.0, 50.0, 60.0, 60.0)]]
        assert (
            find_valid_position_nfp(
                [], part, [], placed, [], _grid(), (0.0, 0.0)
            )
            is None
        )

    def test_empty_placed_returns_none(self):
        """NFP method requires at least one placed part."""
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        assert (
            find_valid_position_nfp(ifp, part, [], [], [], _grid(), (0.0, 0.0))
            is None
        )

    def test_with_hulls(self):
        ifp = [_square(0.0, 0.0, 100.0, 100.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        placed = [[_square(50.0, 50.0, 60.0, 60.0)]]
        pos = find_valid_position_nfp(
            ifp, part, part, placed, placed, _grid(), (0.0, 0.0)
        )
        assert pos is not None

    def test_disjoint_placed(self):
        """Multiple placed parts far from each other."""
        ifp = [_square(0.0, 0.0, 200.0, 200.0)]
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        placed = [
            [_square(20.0, 20.0, 30.0, 30.0)],
            [_square(100.0, 100.0, 110.0, 110.0)],
        ]
        pos = find_valid_position_nfp(
            ifp, part, [], placed, [], _grid(), (0.0, 0.0)
        )
        assert pos is not None


class TestCalculateFitness:
    def test_single_placement(self):
        groups = [
            [np.array(_square(0.0, 0.0, 10.0, 10.0), dtype=float)],
        ]
        fitness = calculate_fitness(groups, [0.0], [0], num_parts=1)
        assert isinstance(fitness, float)
        assert math.isfinite(fitness)
        assert fitness > 0.0

    def test_multiple_placements_same_sheet(self):
        groups = [
            [np.array(_square(0.0, 0.0, 10.0, 10.0), dtype=float)],
            [np.array(_square(10.0, 0.0, 20.0, 10.0), dtype=float)],
        ]
        fitness = calculate_fitness(groups, [0.0, 0.0], [0, 0], num_parts=2)
        assert isinstance(fitness, float)
        assert fitness > 0.0

    def test_multiple_sheets(self):
        groups = [
            [np.array(_square(0.0, 0.0, 10.0, 10.0), dtype=float)],
            [np.array(_square(0.0, 10.0, 10.0, 20.0), dtype=float)],
        ]
        fitness = calculate_fitness(groups, [0.0, 0.0], [0, 1], num_parts=2)
        assert isinstance(fitness, float)
        assert fitness > 0.0

    def test_unplaced_parts(self):
        groups = [
            [np.array(_square(0.0, 0.0, 10.0, 10.0), dtype=float)],
        ]
        fitness = calculate_fitness(groups, [0.0], [0], num_parts=3)
        assert isinstance(fitness, float)
        assert fitness > 0.0

    def test_empty_placements_returns_inf(self):
        fitness = calculate_fitness([], [], [], num_parts=0)
        assert fitness == float("inf")

    def test_multi_polygon_part(self):
        """A part with a hole: outer rect minus inner rect."""
        outer = np.array(_square(0.0, 0.0, 10.0, 10.0), dtype=float)
        inner = np.array(_square(2.0, 2.0, 8.0, 8.0), dtype=float)
        groups = [[outer, inner]]
        fitness = calculate_fitness(groups, [0.0], [0], num_parts=1)
        assert isinstance(fitness, float)
        assert math.isfinite(fitness)
        assert fitness > 0.0


class TestPlaceParts:
    def test_single_part_one_sheet(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part], [[]], [sheet], [(0.0, 0.0)], [0.0], [False], [False]
        )
        assert len(results) == 1
        r = results[0]
        assert len(r["placements"]) == 1
        assert r["sheet_index"] == 0
        assert r["unused_part_indices"] == []
        assert "fitness" in r
        assert math.isfinite(r["fitness"])

    def test_multiple_parts_one_sheet(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part, part],
            [[], []],
            [sheet],
            [(0.0, 0.0)],
            [0.0, 0.0],
            [False, False],
            [False, False],
        )
        assert len(results) == 1
        r = results[0]
        assert len(r["placements"]) == 2

    def test_with_hulls(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        hull = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part],
            [hull],
            [sheet],
            [(0.0, 0.0)],
            [0.0],
            [False],
            [False],
        )
        assert len(results) == 1
        assert len(results[0]["placements"]) == 1

    def test_with_rotation(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part],
            [[]],
            [sheet],
            [(0.0, 0.0)],
            [90.0],
            [False],
            [False],
        )
        assert len(results) == 1
        assert len(results[0]["placements"]) == 1

    def test_with_flip_h(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part],
            [[]],
            [sheet],
            [(0.0, 0.0)],
            [0.0],
            [True],
            [False],
        )
        assert len(results) == 1
        assert len(results[0]["placements"]) == 1

    def test_with_flip_v(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part],
            [[]],
            [sheet],
            [(0.0, 0.0)],
            [0.0],
            [False],
            [True],
        )
        assert len(results) == 1
        assert len(results[0]["placements"]) == 1

    def test_multiple_sheets(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet1 = _square(0.0, 0.0, 50.0, 50.0)
        sheet2 = _square(0.0, 0.0, 50.0, 50.0)
        results = place_parts(
            [part, part],
            [[], []],
            [sheet1, sheet2],
            [(0.0, 0.0), (100.0, 0.0)],
            [0.0, 0.0],
            [False, False],
            [False, False],
        )
        assert len(results) == 2
        assert results[0]["sheet_index"] == 0
        assert results[1]["sheet_index"] == 1

    def test_sheet_world_offset(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part],
            [[]],
            [sheet],
            [(50.0, 50.0)],
            [0.0],
            [False],
            [False],
        )
        assert len(results) == 1
        p = results[0]["placements"][0]
        x, y = p["position"]
        assert x >= 50.0
        assert y >= 50.0

    def test_more_parts_than_fit(self):
        many_parts = [[_square(0.0, 0.0, 10.0, 10.0)] for _ in range(200)]
        sheet = _square(0.0, 0.0, 50.0, 50.0)
        results = place_parts(
            many_parts,
            [[] for _ in range(200)],
            [sheet],
            [(0.0, 0.0)],
            [0.0] * 200,
            [False] * 200,
            [False] * 200,
        )
        assert len(results) == 1
        r = results[0]
        assert len(r["unused_part_indices"]) > 0
        assert len(r["placements"]) + len(r["unused_part_indices"]) == 200

    def test_empty_parts(self):
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [],
            [],
            [sheet],
            [(0.0, 0.0)],
            [],
            [],
            [],
        )
        assert len(results) == 0

    def test_custom_curve_tolerance(self):
        part = [_square(0.0, 0.0, 10.0, 10.0)]
        sheet = _square(0.0, 0.0, 100.0, 100.0)
        results = place_parts(
            [part],
            [[]],
            [sheet],
            [(0.0, 0.0)],
            [0.0],
            [False],
            [False],
            curve_tolerance=2.0,
        )
        assert len(results) == 1
        assert len(results[0]["placements"]) == 1
