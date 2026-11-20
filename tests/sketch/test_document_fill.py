from raygeo.sketch.document import (
    DEFAULT_FILL_COLOR,
    Fill,
    FillStyle,
)


def test_fill_defaults():
    fill = Fill("fill-1", [(3, True), (4, False)])
    assert fill.uid == "fill-1"
    assert fill.boundary == [(3, True), (4, False)]
    assert fill.style is FillStyle.SOLID
    assert fill.color == DEFAULT_FILL_COLOR
    assert fill.gradient_stops == []
    assert fill.gradient_angle == 0.0


def test_fill_style_values():
    assert FillStyle.SOLID.value == "solid"
    assert FillStyle.LINEAR_GRADIENT.value == "linear_gradient"
    assert FillStyle.RADIAL_GRADIENT.value == "radial_gradient"


def test_fill_to_dict_defaults():
    fill = Fill("fill-1", [(3, True)])
    assert fill.to_dict() == {
        "uid": "fill-1",
        "boundary": [[3, True]],
        "style": "solid",
        "color": [0.85, 0.85, 0.85, 0.7],
    }


def test_fill_to_dict_gradient():
    fill = Fill(
        "fill-2",
        [(1, True)],
        style=FillStyle.LINEAR_GRADIENT,
        color=(0.9, 0.8, 0.7, 1.0),
        gradient_stops=[(0.0, (1.0, 0.0, 0.0, 1.0))],
        gradient_angle=0.5,
    )
    assert fill.to_dict() == {
        "uid": "fill-2",
        "boundary": [[1, True]],
        "style": "linear_gradient",
        "color": [0.9, 0.8, 0.7, 1.0],
        "gradient_stops": [
            [0.0, [1.0, 0.0, 0.0, 1.0]],
        ],
        "gradient_angle": 0.5,
    }


def test_fill_from_dict_round_trip():
    data = {
        "uid": "fill-3",
        "boundary": [[7, False]],
        "style": "radial_gradient",
        "color": [0.1, 0.2, 0.3, 0.4],
        "gradient_stops": [[0.25, [1.0, 0.5, 0.0, 1.0]]],
        "gradient_angle": 1.25,
    }
    fill = Fill.from_dict(data)
    assert fill.uid == "fill-3"
    assert fill.boundary == [(7, False)]
    assert fill.style is FillStyle.RADIAL_GRADIENT
    assert fill.color == (0.1, 0.2, 0.3, 0.4)
    assert fill.gradient_stops == [(0.25, (1.0, 0.5, 0.0, 1.0))]
    assert fill.gradient_angle == 1.25
    assert fill.to_dict() == data


def test_fill_from_dict_generates_missing_uid():
    fill = Fill.from_dict(
        {
            "boundary": [[1, True]],
            "style": "solid",
            "color": [0.0, 0.0, 0.0, 1.0],
        }
    )
    assert fill.uid
    assert len(fill.uid) == 36


def test_fill_fields_are_mutable():
    fill = Fill("fill-4", [])
    fill.boundary = [(9, True)]
    fill.style = FillStyle.RADIAL_GRADIENT
    fill.color = (0.5, 0.6, 0.7, 0.8)
    fill.gradient_stops = [(1.0, (0.0, 1.0, 0.0, 1.0))]
    fill.gradient_angle = 2.0

    assert fill.boundary == [(9, True)]
    assert fill.style is FillStyle.RADIAL_GRADIENT
    assert fill.color == (0.5, 0.6, 0.7, 0.8)
    assert fill.gradient_stops == [(1.0, (0.0, 1.0, 0.0, 1.0))]
    assert fill.gradient_angle == 2.0


from raygeo.sketch.document import DocumentMeta  # noqa: E402


def test_document_meta_defaults():
    meta = DocumentMeta("uid-1", "Sketch")
    assert meta.format_version == 1
    assert meta.uid == "uid-1"
    assert meta.name == "Sketch"
    assert meta.hidden is False
    assert meta.origin_id == 0


def test_document_meta_round_trip():
    meta = DocumentMeta("uid-2", "Name")
    meta.hidden = True
    meta.origin_id = 7
    data = meta.to_dict()
    restored = DocumentMeta.from_dict(data)
    assert restored.format_version == 1
    assert restored.uid == "uid-2"
    assert restored.name == "Name"
    assert restored.hidden is True
    assert restored.origin_id == 7


def test_document_meta_generates_missing_uid():
    meta = DocumentMeta.from_dict({"name": "Old", "origin_id": 1})
    assert len(meta.uid) == 36
    assert meta.name == "Old"
    assert meta.origin_id == 1


def test_document_meta_copy_is_independent():
    meta = DocumentMeta("uid-3", "Name")
    copy = meta.copy()
    copy.name = "Other"
    assert meta.name == "Name"
