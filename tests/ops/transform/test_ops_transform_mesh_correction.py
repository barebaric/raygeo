import math

import numpy as np
import pytest

from raygeo.ops import Ops


class TestMeshCorrection:
    def test_empty_ops(self):
        ops = Ops()
        ops.mesh_correction(0, 0, 10, 10, np.zeros((2, 2)))
        assert ops.is_empty()

    def test_flat_grid_noop(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(100, 100, 2)
        ops.mesh_correction(0, 0, 10, 10, np.zeros((2, 2)))
        assert ops.endpoint(0) == (0.0, 0.0, 0.0)
        assert ops.endpoint(1) == (100.0, 100.0, 2.0)

    def test_command_count_preserved(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.set_power(0.5)
        ops.line_to(100, 0, 0)
        ops.scan_to(100, 10, 0, power_values=[10, 20, 30])
        n = ops.len()
        ops.mesh_correction(0, 0, 10, 10, np.ones((3, 3)))
        assert ops.len() == n
        assert list(ops.scanline_data(3)) == [10, 20, 30]

    def test_constant_grid_shifts_every_move(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(50, 50, 1)
        grid = np.full((3, 3), 2.0)
        ops.mesh_correction(0, 0, 10, 10, grid)
        assert ops.endpoint(0) == (0.0, 0.0, 2.0)
        assert ops.endpoint(1) == (50.0, 50.0, 3.0)

    def test_bilinear_midpoint(self):
        ops = Ops()
        ops.line_to(50, 50, 0)
        grid = np.array([[0.0, 2.0], [4.0, 6.0]])
        ops.mesh_correction(0, 0, 100, 100, grid)
        x, y, z = ops.endpoint(0)
        assert (x, y) == (50.0, 50.0)
        assert z == pytest.approx(3.0)

    def test_grid_nodes_exact(self):
        ops = Ops()
        for x, y in ((0, 0), (10, 0), (0, 10), (10, 10)):
            ops.line_to(x, y, 0)
        grid = np.array([[1.0, 2.0], [3.0, 4.0]])
        ops.mesh_correction(0, 0, 10, 10, grid)
        assert ops.endpoint(0)[2] == pytest.approx(1.0)
        assert ops.endpoint(1)[2] == pytest.approx(2.0)
        assert ops.endpoint(2)[2] == pytest.approx(3.0)
        assert ops.endpoint(3)[2] == pytest.approx(4.0)

    def test_outside_grid_clamped_to_edge(self):
        ops = Ops()
        ops.line_to(-50, 0, 0)
        ops.line_to(150, 0, 0)
        ops.line_to(50, 500, 0)
        grid = np.array([[1.0, 2.0], [3.0, 4.0]])
        ops.mesh_correction(0, 0, 10, 10, grid)
        assert ops.endpoint(0)[2] == pytest.approx(1.0)
        assert ops.endpoint(1)[2] == pytest.approx(2.0)
        assert ops.endpoint(2)[2] == pytest.approx(4.0)

    def test_z_offset_added(self):
        ops = Ops()
        ops.line_to(0, 0, 2)
        grid = np.full((2, 2), 1.5)
        ops.mesh_correction(0, 0, 10, 10, grid, z_offset=-1.0)
        assert ops.endpoint(0)[2] == pytest.approx(2.5)

    def test_travel_moves_corrected(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(10, 10, 0)
        grid = np.full((2, 2), 1.0)
        ops.mesh_correction(0, 0, 10, 10, grid)
        assert ops.endpoint(0)[2] == pytest.approx(1.0)
        assert ops.endpoint(2)[2] == pytest.approx(1.0)

    def test_close_path_tracks_corrected_move_to(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.close_path()
        grid = np.array([[0.0, 0.0], [0.0, 5.0]])
        ops.mesh_correction(0, 0, 10, 10, grid)
        first = ops.endpoint(0)
        closing = ops.endpoint(2)
        assert closing[0] == pytest.approx(first[0])
        assert closing[1] == pytest.approx(first[1])
        assert closing[2] == pytest.approx(first[2])

    def test_arc_keeps_center_offset(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.arc_to(10, 0, 5, 0, False, 0)
        grid = np.full((2, 2), 1.0)
        ops.mesh_correction(0, 0, 10, 10, grid)
        assert ops.endpoint(1) == (10.0, 0.0, 1.0)

    def test_bezier_controls_corrected(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.bezier_to((2.5, 2.5, 0), (7.5, 7.5, 0), (10, 10, 0))
        grid = np.array([[0.0, 4.0], [8.0, 12.0]])
        ops.mesh_correction(0, 0, 10, 10, grid)
        assert ops.endpoint(1)[2] == pytest.approx(12.0)

    def test_non_finite_xy_left_unchanged(self):
        ops = Ops()
        ops.line_to(math.inf, 0, 1)
        ops.mesh_correction(0, 0, 10, 10, np.full((2, 2), 5.0))
        assert ops.endpoint(0) == (math.inf, 0.0, 1.0)

    def test_single_column_grid(self):
        ops = Ops()
        ops.line_to(0, 0, 0)
        ops.line_to(100, 100, 0)
        grid = np.array([[1.0], [3.0]])
        ops.mesh_correction(0, 0, 10, 10, grid)
        assert ops.endpoint(0)[2] == pytest.approx(1.0)
        assert ops.endpoint(1)[2] == pytest.approx(3.0)

    def test_clone_not_affected(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(100, 100, 0)
        clone = ops.copy()
        clone.mesh_correction(0, 0, 10, 10, np.full((2, 2), 7.0))
        assert ops.endpoint(1)[2] == 0.0
        assert clone.endpoint(1)[2] == pytest.approx(7.0)

    def test_nonpositive_spacing_raises(self):
        ops = Ops()
        ops.line_to(10, 10, 0)
        with pytest.raises(ValueError, match="dx"):
            ops.mesh_correction(0, 0, 0, 10, np.zeros((2, 2)))

    def test_nan_height_raises(self):
        ops = Ops()
        ops.line_to(10, 10, 0)
        grid = np.array([[0.0, float("nan")], [0.0, 0.0]])
        with pytest.raises(ValueError, match="finite"):
            ops.mesh_correction(0, 0, 10, 10, grid)

    def test_zero_row_grid_raises(self):
        ops = Ops()
        ops.line_to(10, 10, 0)
        with pytest.raises(ValueError):
            ops.mesh_correction(0, 0, 10, 10, np.zeros((0, 2)))
