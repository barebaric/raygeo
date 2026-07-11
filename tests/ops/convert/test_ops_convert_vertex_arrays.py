import numpy as np

from raygeo.ops import Ops
from raygeo.ops.types import CommandType


class TestToVertexArrays:
    def test_empty_ops(self):
        ops = Ops()
        pv, pc, tv, zv = ops.to_vertex_arrays()
        assert pv.shape == (0, 3)
        assert pc.shape == (0, 4)
        assert tv.shape == (0, 3)
        assert zv.shape == (0, 3)

    def test_simple_cut_and_travel(self):
        ops = Ops()
        ops.move_to(0.0, 0.0, 0.0)
        ops.move_to(10.0, 0.0, 0.0)
        ops.set_power(1.0)
        ops.line_to(10.0, 10.0, 0.0)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert tv.shape == (2, 3)
        np.testing.assert_array_equal(tv[0], [0.0, 0.0, 0.0])
        np.testing.assert_array_equal(tv[1], [10.0, 0.0, 0.0])

        assert pv.shape == (2, 3)
        assert pc.shape == (2, 4)
        np.testing.assert_array_equal(pv[0], [10.0, 0.0, 0.0])
        np.testing.assert_array_equal(pv[1], [10.0, 10.0, 0.0])
        np.testing.assert_array_equal(pc[0], [1.0, 1.0, 1.0, 1.0])

    def test_zero_power_move(self):
        ops = Ops()
        ops.move_to(0.0, 0.0, 0.0)
        ops.set_power(0.0)
        ops.line_to(5.0, 5.0, 0.0)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert pv.shape == (0, 3)
        assert tv.shape == (0, 3)
        assert zv.shape == (2, 3)
        np.testing.assert_array_equal(zv[0], [0.0, 0.0, 0.0])
        np.testing.assert_array_equal(zv[1], [5.0, 5.0, 0.0])

    def test_arc_linearization(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.move_to(0.0, 10.0, 0.0)
        ops.arc_to(10.0, 0.0, 0.0, -10.0, True)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert pv.shape[0] >= 4
        assert pv.shape[0] % 2 == 0
        assert pc.shape[0] == pv.shape[0]
        np.testing.assert_array_almost_equal(pv[0], [0.0, 10.0, 0.0])
        np.testing.assert_array_almost_equal(pv[-1], [10.0, 0.0, 0.0])
        expected_color = [127 / 255, 127 / 255, 127 / 255, 1.0]
        np.testing.assert_array_almost_equal(pc[0], expected_color)

    def test_bezier_powered(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.move_to(0.0, 0.0, 0.0)
        ops.bezier_to((3.0, 5.0, 0.0), (7.0, 5.0, 0.0), (10.0, 0.0, 0.0))

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert pv.shape[0] >= 4
        assert pv.shape[0] % 2 == 0
        assert pc.shape[0] == pv.shape[0]
        np.testing.assert_array_almost_equal(pv[0], [0.0, 0.0, 0.0])
        np.testing.assert_array_almost_equal(pv[-1], [10.0, 0.0, 0.0])

    def test_bezier_zero_power(self):
        ops = Ops()
        ops.move_to(0.0, 0.0, 0.0)
        ops.set_power(0.0)
        ops.bezier_to((3.0, 5.0, 0.0), (7.0, 5.0, 0.0), (10.0, 0.0, 0.0))

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert pv.shape == (0, 3)
        assert zv.shape[0] >= 4
        assert zv.shape[0] % 2 == 0
        np.testing.assert_array_almost_equal(zv[0], [0.0, 0.0, 0.0])
        np.testing.assert_array_almost_equal(zv[-1], [10.0, 0.0, 0.0])

    def test_scanline_zero_power(self):
        ops = Ops()
        ops.move_to(0.0, 0.0, 0.0)
        power_values = bytearray([0, 128, 255, 0, 255])
        ops.scan_to(5.0, 0.0, 0.0, power_values)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert pv.shape == (0, 3)
        assert zv.shape == (4, 3)
        np.testing.assert_array_almost_equal(zv[0], [0.0, 0.0, 0.0])
        np.testing.assert_array_almost_equal(zv[1], [1.0, 0.0, 0.0])
        np.testing.assert_array_almost_equal(zv[2], [3.0, 0.0, 0.0])
        np.testing.assert_array_almost_equal(zv[3], [4.0, 0.0, 0.0])

    def test_complex_path(self):
        ops = Ops()
        ops.move_to(0.0, 0.0, 0.0)
        ops.move_to(10.0, 0.0, 0.0)
        ops.set_power(1.0)
        ops.line_to(20.0, 0.0, 0.0)
        ops.set_power(0.5)
        ops.line_to(20.0, 10.0, 0.0)
        ops.set_power(0.0)
        ops.line_to(10.0, 10.0, 0.0)
        ops.move_to(0.0, 10.0, 0.0)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert tv.shape == (4, 3)
        assert pv.shape == (4, 3)
        assert pc.shape == (4, 4)
        assert zv.shape == (2, 3)
        np.testing.assert_array_equal(pc[0], [1.0, 1.0, 1.0, 1.0])
        np.testing.assert_array_almost_equal(
            pc[2], [127 / 255, 127 / 255, 127 / 255, 1.0]
        )

    def test_3d_coordinates_preserved(self):
        ops = Ops()
        ops.move_to(0.0, 0.0, 5.0)
        ops.set_power(1.0)
        ops.line_to(10.0, 0.0, 5.0)
        ops.line_to(10.0, 10.0, 0.0)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert pv.shape == (4, 3)
        assert pv[0][2] == 5.0
        assert pv[1][2] == 5.0
        assert pv[2][2] == 5.0
        assert pv[3][2] == 0.0

    def test_state_commands_preserved(self):
        ops = Ops()
        ops.set_power(0.5)
        ops.move_to(0.0, 0.0, 0.0)
        ops.line_to(10.0, 0.0, 0.0)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        power_indices = ops.indices_of(CommandType.SET_POWER)
        assert len(power_indices) == 1

    def test_multi_workpiece(self):
        ops = Ops()
        for wp_idx in range(3):
            for c_idx in range(5):
                cx = wp_idx * 100.0 + c_idx * 10.0
                ops.move_to(cx, 0.0, 0.0)
                ops.set_power(1.0)
                ops.line_to(cx + 5.0, 5.0, 0.0)

        pv, pc, tv, zv = ops.to_vertex_arrays()

        assert pv.shape == (30, 3)
        assert tv.shape == (28, 3)
