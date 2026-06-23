"""Tests for raygeo.geo.algo.ordering module."""

from raygeo.geo.algo.ordering import order_nearest_neighbor


class TestOrderNearestNeighbor:
    """Tests for order_nearest_neighbor function."""

    def test_empty_input(self):
        """Empty list returns empty list."""
        assert order_nearest_neighbor([]) == []

    def test_single_path_returns_zero(self):
        """Single path returns [0]."""
        assert order_nearest_neighbor([[(0.0, 0.0), (10.0, 0.0)]]) == [0]

    def test_starts_with_longest(self):
        """Should start with the longest path (most vertices)."""
        arcs = [
            [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)],
            [(0.0, 0.0), (10.0, 0.0)],
            [(0.0, 5.0), (10.0, 5.0), (20.0, 5.0), (30.0, 5.0)],
        ]
        order = order_nearest_neighbor(arcs)
        assert order[0] == 2

    def test_ties_pick_last(self):
        """When lengths are equal, max_by picks the last."""
        arcs = [
            [(0.0, 0.0), (10.0, 0.0)],
            [(0.0, 5.0), (10.0, 5.0)],
            [(0.0, 10.0), (10.0, 10.0)],
        ]
        order = order_nearest_neighbor(arcs)
        assert order[0] == 2

    def test_nn_orders_by_proximity(self):
        """Order by nearest-first-end to last-end."""
        arcs = [
            [(0.0, 100.0), (10.0, 100.0)],
            [(50.0, 50.0), (60.0, 50.0)],
            [(100.0, 0.0), (110.0, 0.0)],
        ]
        order = order_nearest_neighbor(arcs)
        assert order[0] == 2
        assert order[1] == 1
        assert order[2] == 0

    def test_skips_short_paths(self):
        """Paths with < 2 vertices should be skipped."""
        arcs = [
            [(0.0, 0.0)],
            [(0.0, 0.0), (10.0, 0.0)],
            [(0.0, 5.0), (10.0, 5.0)],
        ]
        order = order_nearest_neighbor(arcs)
        assert 0 not in order
        assert len(order) == 2

    def test_all_paths_visited(self):
        """Every valid path should appear exactly once."""
        arcs = [
            [(float(i * 10), 0.0), (float(i * 10 + 10), 0.0)] for i in range(5)
        ]
        order = order_nearest_neighbor(arcs)
        assert len(order) == 5
        assert sorted(order) == [0, 1, 2, 3, 4]

    def test_result_indices_unique(self):
        """No duplicate indices in the result."""
        arcs = [
            [(0.0, 100.0), (10.0, 100.0)],
            [(50.0, 50.0), (60.0, 50.0)],
            [(100.0, 0.0), (110.0, 0.0)],
            [(20.0, 30.0), (30.0, 30.0)],
            [(80.0, 20.0), (90.0, 20.0)],
        ]
        order = order_nearest_neighbor(arcs)
        assert len(order) == len(set(order))
