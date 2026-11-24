"""Tests for the sketch type vocabulary."""

import pytest

from raygeo.sketch.registry import EntityRegistry
from raygeo.sketch.types import WaypointType


def test_constants_are_typed_instances():
    assert isinstance(WaypointType.SHARP, WaypointType)
    assert isinstance(WaypointType.SMOOTH, WaypointType)
    assert isinstance(WaypointType.SYMMETRIC, WaypointType)


def test_value_and_name():
    assert WaypointType.SHARP.value == "sharp"
    assert WaypointType.SMOOTH.value == "smooth"
    assert WaypointType.SYMMETRIC.value == "symmetric"
    assert WaypointType.SHARP.name == "SHARP"
    assert WaypointType.SMOOTH.name == "SMOOTH"
    assert WaypointType.SYMMETRIC.name == "SYMMETRIC"


def test_string_representation():
    assert str(WaypointType.SHARP) == "sharp"
    assert repr(WaypointType.SMOOTH) == "WaypointType.SMOOTH"


def test_equality_with_constants_and_strings():
    assert WaypointType.SHARP == WaypointType.SHARP
    assert WaypointType.SHARP != WaypointType.SMOOTH
    assert WaypointType.SHARP == "sharp"
    assert "sharp" == WaypointType.SHARP
    assert WaypointType.SHARP != "smooth"
    assert WaypointType.SHARP != 42


def test_hash_matches_serialized_string():
    lookup: dict[object, int] = {"sharp": 1}
    assert hash(WaypointType.SHARP) == hash("sharp")
    assert lookup[WaypointType.SHARP] == 1


def test_point_setter_accepts_constants_and_strings():
    registry = EntityRegistry()
    point = registry.get_point(registry.add_point(0.0, 0.0))

    point.waypoint_type = WaypointType.SMOOTH
    assert isinstance(point.waypoint_type, WaypointType)
    assert point.waypoint_type == WaypointType.SMOOTH

    point.waypoint_type = "symmetric"
    assert point.waypoint_type == WaypointType.SYMMETRIC


def test_point_setter_rejects_other_types():
    registry = EntityRegistry()
    point = registry.get_point(registry.add_point(0.0, 0.0))

    with pytest.raises(TypeError):
        point.waypoint_type = 42


def test_registry_accessors_use_typed_values():
    registry = EntityRegistry()
    point_id = registry.add_point(0.0, 0.0)

    assert isinstance(registry.waypoint_type(point_id), WaypointType)
    assert registry.waypoint_type(point_id) == WaypointType.SHARP

    assert registry.set_point_waypoint_type(point_id, WaypointType.SYMMETRIC)
    assert registry.waypoint_type(point_id) == WaypointType.SYMMETRIC

    assert registry.set_point_waypoint_type(point_id, "smooth")
    assert registry.waypoint_type(point_id) == WaypointType.SMOOTH


def test_setter_writes_through_to_registry():
    registry = EntityRegistry()
    point = registry.get_point(registry.add_point(0.0, 0.0))

    point.waypoint_type = WaypointType.SMOOTH

    assert registry.waypoint_type(point.id) == WaypointType.SMOOTH
