import pytest

from raygeo.geo import Geometry
from raygeo.ops import Ops
from raygeo.ops.types import CommandType


def test_from_geometry():
    geo_obj = Geometry()
    geo_obj.move_to(10, 10, 0)
    geo_obj.line_to(20, 20, 0)
    geo_obj.arc_to(30, 10, -10, 0, clockwise=False, z=0)

    ops = Ops.from_geometry(geo_obj)

    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == (10, 10, 0)
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.endpoint(1) == (20, 20, 0)
    assert ops.command_type(2) == CommandType.ARC_TO
    assert ops.endpoint(2) == (30, 10, 0)
    i, j, cw = ops.arc_params(2)
    assert (i, j) == (-10, 0)
    assert cw is False
    assert ops.last_move_to == geo_obj.last_move_to


def test_from_geometry_with_bezier():
    geo_obj = Geometry()
    geo_obj.move_to(10, 10, 0)
    geo_obj.line_to(20, 20, 0)
    geo_obj.arc_to_as_bezier(30, 10, -10, 0, clockwise=False, z=0)

    ops = Ops.from_geometry(geo_obj)

    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.endpoint(0) == pytest.approx((10, 10, 0))
    assert ops.command_type(1) == CommandType.LINE_TO
    assert ops.endpoint(1) == pytest.approx((20, 20, 0))
    assert all(
        ops.command_type(i) == CommandType.BEZIER_TO
        for i in range(2, ops.len())
    )
    assert ops.endpoint(ops.len() - 1) == pytest.approx((30, 10, 0))
    assert ops.last_move_to == geo_obj.last_move_to


def test_to_geometry():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 10)
    ops.arc_to(20, 0, 5, 0, False)
    ops.set_power(1.0)
    ops.bezier_to((15, 15, 0), (25, 15, 0), (30, 0, 0))
    geo = ops.to_geometry()
    assert isinstance(geo, Geometry)


def test_to_geometry_empty():
    ops = Ops()
    geo = ops.to_geometry()
    assert isinstance(geo, Geometry)
