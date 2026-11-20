from raygeo.sketch.array_model import (
    Array,
    ArrayDefinition,
    CircularArray,
    CurveAlongArray,
)


def test_circular_array_round_trip():
    array = CircularArray("u1", 7, [(0, [1])], count=3, total_angle_deg=90)
    data = array.to_dict()
    restored = Array.from_dict(data)
    assert isinstance(restored, CircularArray)
    assert restored.uid == "u1"
    assert restored.guide_circle_id == 7
    assert restored.members == [(0, [1])]
    assert restored.total_angle_deg == 90


def test_curve_array_round_trip():
    array = CurveAlongArray(
        "u2",
        8,
        [(0, [2])],
        count=4,
        path_entity_id=9,
        spacing=1.5,
        template_anchor=((1.0, 2.0), 0.5),
    )
    restored = Array.from_dict(array.to_dict())
    assert isinstance(restored, CurveAlongArray)
    assert restored.path_entity_id == 9
    assert restored.spacing == 1.5
    assert restored.template_anchor == ((1.0, 2.0), 0.5)


def test_state_is_backed_by_rust_definition():
    array = CircularArray("u3", 1)
    assert isinstance(array._def, ArrayDefinition)
    array.count = 9
    assert array._def.count == 9
