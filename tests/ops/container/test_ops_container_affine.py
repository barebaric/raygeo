import math

import numpy as np
import pytest

from raygeo.ops import Ops
from raygeo.ops.axis import Axis
from raygeo.ops.types import CommandCategory, CommandType


def test_translate_3d():
    ops = Ops()
    ops.move_to(10, 20, 30)
    ops.line_to(30, 40, 50)
    ops.translate(5, 10, -20)
    assert ops.endpoint(0) == pytest.approx((15, 30, 10))
    assert ops.endpoint(1) == pytest.approx((35, 50, 30))
    assert ops.last_move_to == pytest.approx((15, 30, 10))


def test_scale_3d():
    ops = Ops()
    ops.move_to(10, 20, 5)
    ops.arc_to(22, 22, 5, 7, z=-10)
    ops.scale(2, 3, 4)  # Non-uniform scale

    assert ops.endpoint(0) == pytest.approx((20, 60, 20))
    assert ops.command_type(1) == CommandType.LINE_TO
    final_idx = ops.len() - 1
    final_point = ops.endpoint(final_idx)
    expected_final_point = (22 * 2, 22 * 3, -10 * 4)
    assert final_point == pytest.approx(expected_final_point)
    assert ops.last_move_to == pytest.approx((20, 60, 20))


def test_rotate_preserves_z():
    ops = Ops()
    ops.move_to(10, 10, -5)
    ops.rotate(90, 0, 0)
    x, y, z = ops.endpoint(0)
    assert z == -5
    assert x == pytest.approx(-10)
    assert y == pytest.approx(10)


def test_transform_uniform():
    """Tests applying a uniform transformation (rotation + translation)."""
    ops = Ops()
    ops.move_to(10, 0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=False)  # 90 deg arc

    # Rotate 90 degrees around origin and translate by (100, 0)
    angle_rad = math.radians(90)
    cos_a, sin_a = math.cos(angle_rad), math.sin(angle_rad)
    matrix = np.array(
        [
            [cos_a, -sin_a, 0, 100],
            [sin_a, cos_a, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ]
    )

    ops.transform(matrix)

    # Original (10,0) -> Rotated (0,10) -> Translated (100, 10)
    assert ops.endpoint(0) == pytest.approx((100, 10, 0))
    # Original (0,10) -> Rotated (-10,0) -> Translated (90, 0)
    assert ops.endpoint(1) == pytest.approx((90, 0, 0))

    # Arc should NOT be linearized
    assert ops.command_type(1) == CommandType.ARC_TO

    # Original offset (-10, 0) should be rotated to (0, -10)
    _, _, cw = ops.arc_params(1)
    assert ops.inspect(1).center_offset == pytest.approx((0, -10))


def test_transform_uniform_reflection_flips_cw():
    """A uniform transform with negative determinant flips the CW flag."""
    ops = Ops()
    ops.move_to(10, 0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=False)

    # Mirror in X: det = -1
    matrix = np.array(
        [
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    )
    ops.transform(matrix)

    assert ops.command_type(1) == CommandType.ARC_TO
    i, j, cw = ops.arc_params(1)
    assert cw is True
    assert i == pytest.approx(10)
    assert j == pytest.approx(0)


def test_transform_uniform_reflection_flips_ccw_to_cw():
    """Mirroring a CW arc should make it CCW."""
    ops = Ops()
    ops.move_to(10, 0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=True)

    # Mirror in Y: det = -1
    matrix = np.array(
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    )
    ops.transform(matrix)

    assert ops.command_type(1) == CommandType.ARC_TO
    _, _, cw = ops.arc_params(1)
    assert cw is False


def test_transform_uniform_rotation_preserves_cw():
    """A rotation (positive det) should preserve the CW flag."""
    ops = Ops()
    ops.move_to(10, 0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=False)

    angle_rad = math.radians(45)
    cos_a, sin_a = math.cos(angle_rad), math.sin(angle_rad)
    matrix = np.array(
        [
            [cos_a, -sin_a, 0, 0],
            [sin_a, cos_a, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ]
    )
    ops.transform(matrix)

    assert ops.command_type(1) == CommandType.ARC_TO
    _, _, cw = ops.arc_params(1)
    assert cw is False


def test_transform_non_uniform():
    """Tests that a non-uniform scale linearizes arcs."""
    ops = Ops()
    ops.move_to(10, 0)
    ops.arc_to(0, 10, i=-10, j=0, clockwise=False)  # 90 deg arc

    # Scale X by 2, Y by 3
    matrix = np.diag([2.0, 3.0, 1.0, 1.0])
    ops.transform(matrix)

    # Original move_to (10,0) -> (20, 0)
    assert ops.endpoint(0) == pytest.approx((20, 0, 0))

    # Arc must be linearized into LineToCommands
    assert all(
        ops.command_type(i) == CommandType.LINE_TO for i in range(1, ops.len())
    )
    assert ops.len() > 2

    # Final point should be original arc end (0,10) scaled -> (0, 30)
    assert ops.endpoint(ops.len() - 1) == pytest.approx((0, 30, 0))


def test_transform_uniform_bezier():
    """Uniform transform preserves BezierToCommand and transforms controls."""
    ops = Ops()
    ops.move_to(0, 0)
    ops.bezier_to(control1=(10, 0, 0), control2=(10, 10, 0), end=(0, 10, 0))

    angle_rad = math.radians(90)
    cos_a, sin_a = math.cos(angle_rad), math.sin(angle_rad)
    matrix = np.array(
        [
            [cos_a, -sin_a, 0, 5],
            [sin_a, cos_a, 0, 10],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ]
    )
    ops.transform(matrix)

    assert ops.command_type(1) == CommandType.BEZIER_TO
    info = ops.inspect(1)
    assert info.end == pytest.approx((-5, 10, 0))
    assert info.control1 == pytest.approx((5, 20, 0))
    assert info.control2 == pytest.approx((-5, 20, 0))


def test_transform_non_uniform_bezier():
    """Non-uniform transform preserves BezierToCommand (affine invariance)."""
    ops = Ops()
    ops.move_to(0, 0)
    ops.bezier_to(control1=(10, 0, 0), control2=(10, 10, 0), end=(0, 10, 0))

    matrix = np.diag([2.0, 3.0, 1.0, 1.0])
    ops.transform(matrix)

    assert ops.command_type(1) == CommandType.BEZIER_TO
    info = ops.inspect(1)
    assert info.end == pytest.approx((0, 30, 0))
    assert info.control1 == pytest.approx((20, 0, 0))
    assert info.control2 == pytest.approx((20, 30, 0))


def test_transform_bezier_linearize_matches():
    """Transform then linearize should match linearize then transform.

    Bezier curves are affine-invariant: transforming control points
    then evaluating produces the same curve as evaluating then
    transforming.  We verify by comparing bounding boxes rather than
    point-by-point (different subdivision counts are expected when
    the linearization tolerance is applied in different scales).
    """
    ops = Ops()
    ops.move_to(0, 0)
    ops.bezier_to(control1=(10, 0, 0), control2=(10, 10, 0), end=(0, 10, 0))

    ops_linearized_first = Ops()
    ops_linearized_first.move_to(0, 0)
    ops_linearized_first.bezier_to(
        control1=(10, 0, 0), control2=(10, 10, 0), end=(0, 10, 0)
    )
    ops_linearized_first.linearize_curves()

    matrix = np.array(
        [
            [2, 0, 0, 5],
            [0, 2, 0, 3],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ]
    )

    ops.transform(matrix)
    ops.linearize_curves()

    ops_linearized_first.transform(matrix)

    def _endpoints(o):
        return np.array(
            [
                o.endpoint(i)
                for i in range(o.len())
                if o.category(i) == CommandCategory.MOVING
            ]
        )

    pts_a = _endpoints(ops)
    pts_b = _endpoints(ops_linearized_first)

    assert pts_a[0] == pytest.approx(pts_b[0], abs=1e-9)
    assert pts_a[-1] == pytest.approx(pts_b[-1], abs=1e-9)

    for i in range(3):
        assert pts_a[:, i].min() == pytest.approx(pts_b[:, i].min(), abs=0.1)
        assert pts_a[:, i].max() == pytest.approx(pts_b[:, i].max(), abs=0.1)


def test_transform_preserves_extra_axes():
    """Ops.transform() must not modify extra_axes on any command."""
    ops = Ops()
    ops.move_to(10, 20, 0, extra={Axis.A: 45.0, Axis.Y: 0.0})
    ops.line_to(30, 40, 0, extra={Axis.A: 90.0, Axis.Y: 10.0})

    angle_rad = math.radians(90)
    cos_a, sin_a = math.cos(angle_rad), math.sin(angle_rad)
    matrix = np.array(
        [
            [cos_a, -sin_a, 0, 5],
            [sin_a, cos_a, 0, 10],
            [0, 0, 1, 0],
            [0, 0, 0, 1],
        ]
    )
    ops.transform(matrix)

    assert ops.inspect(0).extra_axes == {Axis.A: 45.0, Axis.Y: 0.0}
    assert ops.inspect(1).extra_axes == {Axis.A: 90.0, Axis.Y: 10.0}

    assert ops.endpoint(0) != pytest.approx((10, 20, 0))
    assert ops.endpoint(1) != pytest.approx((30, 40, 0))


def test_transform_preserves_extra_axes_with_identity():
    """Even with identity matrix, extra_axes must survive untouched."""
    ops = Ops()
    ops.move_to(5, 5, 0, extra={Axis.B: 180.0})
    ops.line_to(15, 25, 0, extra={Axis.B: 270.0})

    matrix = np.identity(4)
    ops.transform(matrix)

    assert ops.inspect(0).extra_axes == {Axis.B: 180.0}
    assert ops.inspect(1).extra_axes == {Axis.B: 270.0}
    assert ops.endpoint(0) == pytest.approx((5, 5, 0))
    assert ops.endpoint(1) == pytest.approx((15, 25, 0))


def test_translate_with_scanline():
    """Tests that translate() correctly transforms ScanLinePowerCommand."""
    ops = Ops()
    ops.move_to(10, 20, 30)
    ops.scan_to(40, 50, 60, bytearray([1, 2, 3]))
    ops.translate(5, -10, 15)

    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.SCAN_LINE

    # Check if both start_point (from move_to) and end are translated
    assert ops.endpoint(0) == pytest.approx((15, 10, 45))
    assert ops.endpoint(1) == pytest.approx((45, 40, 75))


def test_transform_moving_endpoint():
    ops = Ops()
    ops.move_to(10.0, 20.0, 5.0)
    ops.line_to(30.0, 40.0, 5.0)

    def flip_xy(end, extra):
        end[0] = -end[0]
        end[1] = -end[1]

    ops.transform_moving(flip_xy)
    assert ops.endpoint(0) == (-10.0, -20.0, 5.0)
    assert ops.endpoint(1) == (-30.0, -40.0, 5.0)


def test_transform_moving_extra_axes():
    ops = Ops()
    ops.move_to(0.0, 0.0, extra={Axis.A: 10.0})
    ops.line_to(10.0, 10.0, extra={Axis.A: 20.0})

    def scale_a(end, extra):
        if Axis.A in extra:
            extra[Axis.A] = extra[Axis.A] * 2

    ops.transform_moving(scale_a)
    assert ops.inspect(0).extra_axes == {Axis.A: 20.0}
    assert ops.inspect(1).extra_axes == {Axis.A: 40.0}


def test_transform_moving_arc_center():
    ops = Ops()
    ops.arc_to(5.0, 0.0, 2.0, 3.0)

    def scale_center(pt):
        pt[0] = pt[0] * 2
        pt[1] = pt[1] * 2

    ops.transform_moving(lambda end, extra: None, scale_center)
    assert ops.command_type(0) == CommandType.ARC_TO
    info = ops.inspect(0)
    assert info.center_offset == (4.0, 6.0)


def test_transform_moving_bezier_controls():
    ops = Ops()
    ops.move_to(0.0, 0.0, 0.0)
    ops.bezier_to((1.0, 2.0, 0.0), (3.0, 4.0, 0.0), (5.0, 6.0, 0.0))

    def flip_y(pt):
        pt[1] = -pt[1]

    ops.transform_moving(lambda end, extra: None, flip_y)
    assert ops.command_type(1) == CommandType.BEZIER_TO
    info = ops.inspect(1)
    assert info.control1 == (1.0, -2.0, 0.0)
    assert info.control2 == (3.0, -4.0, 0.0)
    assert info.end == (5.0, 6.0, 0.0)


def test_transform_moving_no_aux():
    ops = Ops()
    ops.move_to(1.0, 2.0)

    def reset(end, extra):
        end[:] = [0.0, 0.0, 0.0]

    ops.transform_moving(reset)
    assert ops.endpoint(0) == (0.0, 0.0, 0.0)


def test_transform_moving_skips_markers():
    ops = Ops()
    ops.layer_start("lyr1")
    ops.move_to(5.0, 5.0)
    ops.layer_end("lyr1")

    def reset(end, extra):
        end[:] = [0.0, 0.0, 0.0]

    ops.transform_moving(reset)
    assert ops.endpoint(1) == (0.0, 0.0, 0.0)
