import pytest

from raygeo.geo import Bezier as GeoBezier
from raygeo.geo import Geometry, Move
from raygeo.geo import Line as GeoLine
from raygeo.geo.shape.bezier import get_bezier_samples
from raygeo.sketch.registry import EntityRegistry


@pytest.fixture
def registry():
    return EntityRegistry()


def _resolved_control_points(registry, bid):
    """Absolute control points, falling back to the endpoints when no
    offsets are stored (get_control_points_or_endpoints equivalent)."""
    data = registry.entity_to_dict(bid)
    start_x, start_y = registry.get_point(data["start_idx"])
    end_x, end_y = registry.get_point(data["end_idx"])
    if "cp1_dx" in data:
        cp1_x = start_x + data["cp1_dx"]
        cp1_y = start_y + data["cp1_dy"]
    else:
        cp1_x, cp1_y = start_x, start_y
    if "cp2_dx" in data:
        cp2_x = end_x + data["cp2_dx"]
        cp2_y = end_y + data["cp2_dy"]
    else:
        cp2_x, cp2_y = end_x, end_y
    return cp1_x, cp1_y, cp2_x, cp2_y


def _sample_bbox(registry, bid):
    """Bounding box from the sampled polygon vertices in both
    directions (_get_bbox equivalent)."""
    vertices = registry.to_polygon_vertices(bid, forward=True)
    vertices += registry.to_polygon_vertices(bid, forward=False)
    xs = [x for x, y in vertices]
    ys = [y for x, y in vertices]
    return min(xs), min(ys), max(xs), max(ys)


def test_bezier_serialization_round_trip():
    data = {
        "points": [
            {"id": 1, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 4, "x": 20.0, "y": 0.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 10,
                "type": "bezier",
                "construction": True,
                "start_idx": 1,
                "end_idx": 4,
            },
        ],
        "id_counter": 11,
    }

    registry = EntityRegistry.from_dict(data)
    assert registry.entity_to_dict(10) == {
        "id": 10,
        "type": "bezier",
        "construction": True,
        "start_idx": 1,
        "end_idx": 4,
    }

    restored = EntityRegistry.from_dict(registry.to_dict())
    assert restored.entity_type(10) == "bezier"
    assert restored.entity_point_ids(10) == registry.entity_point_ids(10)
    assert restored.is_construction(10) == registry.is_construction(10)


def test_bezier_get_point_ids(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(20, 0)
    bid = registry.add_bezier(p1, p2)
    assert set(registry.entity_point_ids(bid)) == {p1, p2}


def test_bezier_get_endpoint_ids(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(20, 0)
    bid = registry.add_bezier(p1, p2)
    assert registry.entity_endpoint_ids(bid) == [p1, p2]


def test_bezier_as_line(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(20, 0)
    bid = registry.add_bezier(start, end)

    geo = registry.to_geometry(bid)
    assert isinstance(geo.data[1], GeoLine)

    vertices = registry.to_polygon_vertices(bid, forward=True)
    assert len(vertices) == 1
    assert vertices[0] == pytest.approx((0.0, 0.0))

    vertices_rev = registry.to_polygon_vertices(bid, forward=False)
    assert len(vertices_rev) == 1
    assert vertices_rev[0] == pytest.approx((20.0, 0.0))


def test_bezier_with_control_points(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(20, 0)

    bid = registry.add_bezier(start, end, cp1=(5.0, 10.0), cp2=(-5.0, 10.0))

    geo = registry.to_geometry(bid)
    assert isinstance(geo.data[1], GeoBezier)

    vertices = registry.to_polygon_vertices(bid, forward=True)
    assert len(vertices) == 21
    assert vertices[0] == pytest.approx((0.0, 0.0))
    assert vertices[-1] == pytest.approx((20.0, 0.0))


def test_bezier_get_junction_point_ids(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(20, 0)
    bid = registry.add_bezier(p1, p2)
    assert set(registry.junction_point_ids(bid)) == {p1, p2}


def test_bezier_hit_test_as_line(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(20, 0)
    bid = registry.add_bezier(start, end)
    threshold = 5.0

    assert registry.hit_test(bid, 0, 0, threshold) is True
    assert registry.hit_test(bid, 20, 0, threshold) is True
    assert registry.hit_test(bid, 10, 0, threshold) is True
    assert registry.hit_test(bid, 10, 10, threshold) is False


def test_bezier_hit_test_with_control_points(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(20, 0)

    bid = registry.add_bezier(start, end, cp1=(0.0, 20.0), cp2=(0.0, -20.0))

    threshold = 5.0

    assert registry.hit_test(bid, 0, 0, threshold) is True
    assert registry.hit_test(bid, 20, 0, threshold) is True


def test_bezier_update_constrained_status(registry):
    """Test the entity constrained status recompute. Endpoint
    constrained flags are solver-computed and read-only through the
    bindings, so only the unconstrained baseline is exercisable."""
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(20, 0)
    bid = registry.add_bezier(p1, p2)

    registry.update_constrained_status([])
    assert registry.is_constrained(bid) is False


def test_bezier_to_geometry_as_line(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(20, 0)
    bid = registry.add_bezier(start, end)
    geo = registry.to_geometry(bid)
    assert isinstance(geo, Geometry)
    assert len(geo) == 2
    assert geo.data is not None
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoLine)


def test_bezier_to_geometry_with_control_points(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(20, 0)

    bid = registry.add_bezier(start, end, cp1=(5.0, 10.0), cp2=(-5.0, 10.0))

    geo = registry.to_geometry(bid)
    assert isinstance(geo, Geometry)
    assert len(geo) == 2
    assert geo.data is not None
    assert isinstance(geo.data[0], Move)
    assert isinstance(geo.data[1], GeoBezier)


def test_bezier_get_set_state(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(20, 0)
    bid = registry.add_bezier(p1, p2)

    state = registry.get_state(bid)
    assert state == {
        "construction": False,
        "cp1": None,
        "cp2": None,
    }

    registry.set_construction(bid, True)

    registry.set_state(bid, state)
    assert registry.is_construction(bid) is False


def test_bezier_with_own_control_points(registry):
    start = registry.add_point(0, 0)
    end = registry.add_point(20, 0)

    bid = registry.add_bezier(start, end, cp1=(5.0, 10.0), cp2=(-5.0, 10.0))

    geo = registry.to_geometry(bid)
    assert isinstance(geo.data[1], GeoBezier)

    cp1_x, cp1_y, cp2_x, cp2_y = _resolved_control_points(registry, bid)
    assert cp1_x == pytest.approx(5.0)
    assert cp1_y == pytest.approx(10.0)
    assert cp2_x == pytest.approx(15.0)
    assert cp2_y == pytest.approx(10.0)


def test_bezier_control_points_serialization():
    data = {
        "points": [
            {"id": 1, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 4, "x": 20.0, "y": 0.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 10,
                "type": "bezier",
                "construction": False,
                "start_idx": 1,
                "end_idx": 4,
                "cp1_dx": 5.0,
                "cp1_dy": 10.0,
                "cp2_dx": -3.0,
                "cp2_dy": 7.0,
            },
        ],
        "id_counter": 11,
    }

    registry = EntityRegistry.from_dict(data)
    serialized = registry.entity_to_dict(10)
    assert serialized["cp1_dx"] == 5.0
    assert serialized["cp1_dy"] == 10.0
    assert serialized["cp2_dx"] == -3.0
    assert serialized["cp2_dy"] == 7.0

    restored = EntityRegistry.from_dict(registry.to_dict())
    data_out = restored.entity_to_dict(10)
    assert data_out["cp1_dx"] == pytest.approx(5.0)
    assert data_out["cp1_dy"] == pytest.approx(10.0)
    assert data_out["cp2_dx"] == pytest.approx(-3.0)
    assert data_out["cp2_dy"] == pytest.approx(7.0)


def test_bezier_without_control_points_serialization():
    data = {
        "points": [
            {"id": 1, "x": 0.0, "y": 0.0, "fixed": False},
            {"id": 4, "x": 20.0, "y": 0.0, "fixed": False},
        ],
        "entities": [
            {
                "id": 10,
                "type": "bezier",
                "construction": False,
                "start_idx": 1,
                "end_idx": 4,
            },
        ],
        "id_counter": 11,
    }

    registry = EntityRegistry.from_dict(data)
    serialized = registry.entity_to_dict(10)
    assert "cp1_dx" not in serialized
    assert "cp2_dx" not in serialized

    restored = EntityRegistry.from_dict(registry.to_dict())
    assert restored.get_state(10)["cp1"] is None
    assert restored.get_state(10)["cp2"] is None


def test_bezier_get_control_points_none(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)

    bid = registry.add_bezier(p1, p2, cp1=None, cp2=None)

    data = registry.entity_to_dict(bid)
    assert "cp1_dx" not in data
    assert "cp1_dy" not in data
    assert "cp2_dx" not in data
    assert "cp2_dy" not in data


def test_bezier_get_control_points_with_values(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)

    bid = registry.add_bezier(p1, p2, cp1=(20, 10), cp2=(-30, -15))

    cp1_x, cp1_y, cp2_x, cp2_y = _resolved_control_points(registry, bid)

    assert cp1_x == 20.0
    assert cp1_y == 10.0
    assert cp2_x == 70.0
    assert cp2_y == -15.0


def test_bezier_get_control_points_or_endpoints_with_cps(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)

    bid = registry.add_bezier(p1, p2, cp1=(20, 10), cp2=(-30, -15))

    cp1_x, cp1_y, cp2_x, cp2_y = _resolved_control_points(registry, bid)

    assert cp1_x == 20.0
    assert cp1_y == 10.0
    assert cp2_x == 70.0
    assert cp2_y == -15.0


def test_bezier_get_control_points_or_endpoints_without_cps(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 50)

    bid = registry.add_bezier(p1, p2, cp1=None, cp2=None)

    cp1_x, cp1_y, cp2_x, cp2_y = _resolved_control_points(registry, bid)

    assert cp1_x == 0.0
    assert cp1_y == 0.0
    assert cp2_x == 100.0
    assert cp2_y == 50.0


def test_bezier_sample_bezier_linear(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)

    registry.add_bezier(p1, p2, cp1=None, cp2=None)

    points = get_bezier_samples((0, 0), (0, 0), (100, 0), (100, 0), 5)

    assert len(points) == 6
    assert points[0] == (0.0, 0.0)
    assert points[5] == (100.0, 0.0)

    for i, (x, y) in enumerate(points):
        assert y == pytest.approx(0.0)


def test_bezier_sample_bezier_curved(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)

    registry.add_bezier(p1, p2, cp1=(0, 100), cp2=(100, 100))

    points = get_bezier_samples((0, 0), (0, 100), (100, 100), (100, 0), 10)

    assert len(points) == 11
    assert points[0] == (0.0, 0.0)
    assert points[10] == (100.0, 0.0)

    mid_point = points[5]
    assert mid_point[0] == pytest.approx(50.0)
    assert mid_point[1] == pytest.approx(75.0)


def test_bezier_sample_bezier_quarter_circle_approximation(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)

    registry.add_bezier(p1, p2)

    k = 0.5522847498
    points = get_bezier_samples(
        (0, 100), (k * 100, 100), (100, k * 100), (100, 0), 20
    )

    assert len(points) == 21

    start = points[0]
    assert start[0] == pytest.approx(0.0)
    assert start[1] == pytest.approx(100.0)

    end = points[20]
    assert end[0] == pytest.approx(100.0)
    assert end[1] == pytest.approx(0.0)


def test_bezier_get_bbox_linear(registry):
    p1 = registry.add_point(10, 20)
    p2 = registry.add_point(50, 60)

    bid = registry.add_bezier(p1, p2, cp1=None, cp2=None)

    bbox = _sample_bbox(registry, bid)

    assert bbox == (10.0, 20.0, 50.0, 60.0)


def test_bezier_get_bbox_curved(registry):
    p1 = registry.add_point(0, 0)
    p2 = registry.add_point(100, 0)

    bid = registry.add_bezier(p1, p2, cp1=(0, 100), cp2=(100, 100))

    bbox = _sample_bbox(registry, bid)

    assert bbox[0] == pytest.approx(0.0)
    assert bbox[1] == pytest.approx(0.0)
    assert bbox[2] >= 100.0
    assert bbox[3] > 50.0
