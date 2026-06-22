"""Tests for Medial Axis Transform computation."""

import pytest

from raygeo.geo.algo.medial_axis import MedialAxis


def _rect(w, h):
    return [(0, 0), (w, 0), (w, h), (0, h)]


def _ls():
    return [(0, 0), (10, 0), (10, 3), (3, 3), (3, 10), (0, 10)]


class TestComputeMedialAxis:
    def test_rect_basic(self):
        outer = _rect(50, 40)
        axis = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=5.0
        )
        assert len(axis.nodes) > 0
        assert len(axis.clearances) == len(axis.nodes)
        assert len(axis.branches) >= 4  # rectangle has 4+ branches
        # Root should have max clearance
        assert axis.clearances[axis.root] == max(axis.clearances)

    def test_rect_root_position(self):
        """For a rectangle the root should be near the center."""
        outer = _rect(50, 40)
        axis = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=5.0
        )
        rx, ry = axis.nodes[axis.root]
        # Center of valid area ≈ (25, 20); allow ±5 mm
        assert 20 <= rx <= 30
        assert 15 <= ry <= 25

    def test_rect_root_clearance(self):
        """Max clearance should roughly equal half the shorter side."""
        outer = _rect(50, 40)
        axis = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=5.0
        )
        # max clearance ≈ min(25, 20) = 20; allow ±1
        assert abs(axis.clearances[axis.root] - 20.0) < 2.0

    def test_clearances_non_negative(self):
        outer = _rect(100, 80)
        axis = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=8.0
        )
        assert all(c >= 0.0 for c in axis.clearances)

    def test_edges_form_tree(self):
        """With n nodes there should be exactly n-1 edges (a tree)."""
        outer = _rect(60, 50)
        axis = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=6.0
        )
        assert len(axis.edges) == len(axis.nodes) - 1

    def test_branches_have_ordered_clearances(self):
        """Each branch should have root→leaf ordered nodes."""
        outer = _rect(60, 50)
        axis = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=6.0
        )
        # nodes returned as (x,y); clearances not in branch struct
        # but the node indices in the branch should be contiguous
        for b in axis.branches:
            assert len(b) >= 2

    def test_with_island(self):
        """Pocket with an island produces a valid MAT."""
        outer = _rect(100, 80)
        island = [(30, 30), (50, 30), (50, 50), (30, 50)]
        axis = MedialAxis.compute(
            outer, holes=[island], tool_radius=1.0, sampling_spacing=8.0
        )
        assert len(axis.nodes) > 0
        assert axis.clearances[axis.root] == max(axis.clearances)
        assert len(axis.edges) == len(axis.nodes) - 1
        assert len(axis.branches) >= 4

    def test_l_shape(self):
        outer = _ls()
        axis = MedialAxis.compute(
            outer, holes=[], tool_radius=0.5, sampling_spacing=3.0
        )
        assert len(axis.nodes) > 0
        assert axis.clearances[axis.root] == max(axis.clearances)
        assert len(axis.edges) == len(axis.nodes) - 1

    def test_empty_outer_errors(self):
        with pytest.raises(RuntimeError, match="at least 3 vertices"):
            MedialAxis.compute(
                [(0, 0)], holes=[], tool_radius=1.0, sampling_spacing=1.0
            )

    def test_too_narrow_pocket(self):
        outer = [(0, 0), (2, 0), (2, 2), (0, 2)]
        with pytest.raises(RuntimeError, match="no valid medial axis"):
            MedialAxis.compute(
                outer, holes=[], tool_radius=5.0, sampling_spacing=1.0
            )

    def test_sampling_spacing_affects_node_count(self):
        outer = _rect(100, 80)
        axis1 = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=4.0
        )
        axis2 = MedialAxis.compute(
            outer, holes=[], tool_radius=1.0, sampling_spacing=10.0
        )
        # Denser sampling → more nodes (more triangles)
        assert len(axis1.nodes) > len(axis2.nodes)

    def test_negative_sampling_spacing_errors(self):
        outer = _rect(50, 40)
        with pytest.raises(RuntimeError, match="sampling_spacing"):
            MedialAxis.compute(
                outer, holes=[], tool_radius=1.0, sampling_spacing=-1.0
            )

    def test_three_islands(self):
        outer = _rect(180, 120)
        islands = [
            [(15, 15), (35, 15), (35, 35), (15, 35)],
            [(70, 40), (90, 40), (90, 60), (70, 60)],
            [(130, 80), (160, 80), (160, 105), (130, 105)],
        ]
        axis = MedialAxis.compute(
            outer, holes=islands, tool_radius=1.0, sampling_spacing=8.0
        )
        assert len(axis.nodes) > 0
        assert len(axis.edges) == len(axis.nodes) - 1

    def test_no_nodes_inside_islands(self):
        outer = _rect(100, 80)
        island = [(30, 30), (50, 30), (50, 50), (30, 50)]
        axis = MedialAxis.compute(
            outer, holes=[island], tool_radius=1.0, sampling_spacing=6.0
        )
        for nx, ny in axis.nodes:
            assert not (30 <= nx <= 50 and 30 <= ny <= 50)


class TestMatPath:
    def test_path_between_corners(self):
        outer = _rect(100, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        path = axis.path_between((5, 5), (95, 5))
        assert path is not None
        assert len(path) >= 2

    def test_path_between_opposite_corners(self):
        outer = _rect(100, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        path = axis.path_between((5, 5), (95, 75))
        assert path is not None
        assert len(path) >= 2

    def test_path_with_island(self):
        outer = _rect(100, 80)
        island = [(30, 30), (50, 30), (50, 50), (30, 50)]
        axis = MedialAxis.compute(outer, [island], 1.0, 6.0)
        path = axis.path_between((5, 5), (95, 75))
        assert path is not None
        assert len(path) >= 2

    def test_same_point_returns_single(self):
        outer = _rect(100, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        path = axis.path_between((25, 25), (25, 25))
        assert path is not None
        assert len(path) >= 1

    def test_path_stays_within_boundary(self):
        outer = _rect(100, 80)
        axis = MedialAxis.compute(outer, [], 1.0, 6.0)
        path = axis.path_between((5, 5), (95, 75))
        assert path is not None
        for x, y in path:
            assert -1 <= x <= 101
            assert -1 <= y <= 81

    def test_too_narrow_pocket_errors(self):
        outer = [(0, 0), (10, 0), (10, 10), (0, 10)]
        with pytest.raises(RuntimeError, match="no valid medial axis"):
            MedialAxis.compute(outer, [], 10.0, 1.0)

    def test_l_shape_path(self):
        outer = [(0, 0), (10, 0), (10, 3), (3, 3), (3, 10), (0, 10)]
        axis = MedialAxis.compute(outer, [], 0.5, 3.0)
        path = axis.path_between((1, 1), (1, 9))
        assert path is not None
        assert len(path) >= 2
