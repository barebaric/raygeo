"""Tests for raygeo.cnc.tool."""

import pytest

from raygeo.cnc.tool import (
    Tool,
    ToolCategory,
    ToolMaterial,
    ToolModel,
)


def _end_mill_model(**overrides):
    params = {
        "diameter": 6.0,
        "shank_diameter": 6.0,
        "cutting_edge_height": 15.0,
        "flute_count": 3.0,
        "overall_length": 50.0,
    }
    params.update(overrides)
    return ToolModel(**params)


def test_model_kwargs_become_parameters():
    m = _end_mill_model()
    assert m.diameter() == 6.0
    assert m.cutting_edge_height() == 15.0
    assert m.get_parameter("flute_count") == 3.0
    assert m.get_parameter("missing") is None
    params = m.get_parameters()
    assert params["diameter"] == 6.0
    assert params["shank_diameter"] == 6.0


def test_model_corner_radius_defaults_to_zero():
    m = _end_mill_model()
    assert m.corner_radius() == 0.0
    bull = _end_mill_model(corner_radius=1.0)
    assert bull.corner_radius() == 1.0


def test_model_is_extensible_with_arbitrary_params():
    # Users add new geometry parameters without changing raygeo.
    vbit = ToolModel(diameter=3.0, angle=60.0, tip_radius=0.1)
    assert vbit.get_parameter("angle") == 60.0
    assert vbit.get_parameter("tip_radius") == 0.1
    assert vbit.diameter() == 3.0


def test_model_accepts_no_params():
    m = ToolModel()
    assert m.get_parameters() == {}
    assert m.diameter() == 0.0


def test_model_equality():
    assert _end_mill_model() == _end_mill_model()
    assert _end_mill_model(diameter=5.0) != _end_mill_model()


def test_category_enum_variants():
    for cat in (
        ToolCategory.EndMill,
        ToolCategory.BallNose,
        ToolCategory.BullNose,
        ToolCategory.Chamfer,
        ToolCategory.Drill,
        ToolCategory.Probe,
        ToolCategory.Vbit,
        ToolCategory.SlittingSaw,
        ToolCategory.Reamer,
        ToolCategory.Tap,
        ToolCategory.ThreadMill,
        ToolCategory.Dovetail,
    ):
        assert cat == cat
    assert ToolCategory.EndMill != ToolCategory.BallNose


def test_material_enum_variants():
    for mat in (
        ToolMaterial.Carbide,
        ToolMaterial.HSS,
        ToolMaterial.HSSE,
        ToolMaterial.Diamond,
        ToolMaterial.CBN,
        ToolMaterial.Ceramic,
    ):
        assert mat == mat
    assert ToolMaterial.Carbide != ToolMaterial.HSS


def test_tool_construction_and_accessors():
    m = _end_mill_model()
    t = Tool(
        label="6mm EM",
        category=ToolCategory.EndMill,
        model=m,
        material=ToolMaterial.Carbide,
        stickout=15.0,
        coating="TiAlN",
    )
    assert t.label == "6mm EM"
    assert t.stickout == 15.0
    assert t.coating == "TiAlN"
    assert t.diameter() == 6.0
    assert t.default_stickout() == 18.0
    assert t.category == ToolCategory.EndMill
    assert t.material == ToolMaterial.Carbide


def test_tool_coating_optional():
    t = Tool(
        label="plain",
        category=ToolCategory.Drill,
        model=ToolModel(diameter=4.0),
        material=ToolMaterial.HSS,
        stickout=10.0,
    )
    assert t.coating is None


def test_tool_model_round_trips():
    m = _end_mill_model(corner_radius=0.5)
    t = Tool(
        label="x",
        category=ToolCategory.BullNose,
        model=m,
        material=ToolMaterial.Carbide,
        stickout=12.0,
    )
    assert t.model == m
    assert t.model.diameter() == 6.0
    assert t.model.corner_radius() == 0.5


def test_default_stickout_reads_cutting_edge_height():
    m = _end_mill_model(cutting_edge_height=22.0)
    t = Tool(
        label="x",
        category=ToolCategory.EndMill,
        model=m,
        material=ToolMaterial.Carbide,
        stickout=20.0,
    )
    assert t.default_stickout() == 25.0


def test_category_is_type_safe_identifier():
    # The category discriminates tools without string comparisons.
    slotting_ok = ToolCategory.EndMill
    slotting_rejected = ToolCategory.Probe
    assert slotting_ok != slotting_rejected
    assert slotting_ok is ToolCategory.EndMill


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
