"""Tests for text-to-geometry conversion."""

from raygeo.geo.shape.text import FontConfig, text_to_geometry


def test_font_config_defaults():
    fc = FontConfig()
    assert fc.family == "sans-serif"
    assert fc.size == 10.0
    assert fc.bold is False
    assert fc.italic is False


def test_font_config_custom():
    fc = FontConfig(family="DejaVu Sans", size=12.0, bold=True, italic=False)
    assert fc.family == "DejaVu Sans"
    assert fc.size == 12.0
    assert fc.bold is True
    assert fc.italic is False


def test_text_to_geometry_simple():
    fc = FontConfig(family="DejaVu Sans", size=12.0)
    geo = text_to_geometry("Hello", font_config=fc)
    assert geo is not None
    assert len(geo.data) > 0


def test_text_to_geometry_single_char():
    fc = FontConfig(family="DejaVu Sans", size=12.0)
    geo = text_to_geometry("A", font_config=fc)
    assert geo is not None
    assert len(geo.data) > 0


def test_text_to_geometry_empty():
    fc = FontConfig(family="DejaVu Sans", size=12.0)
    geo = text_to_geometry("", font_config=fc)
    assert geo is not None


def test_text_to_geometry_bold():
    fc = FontConfig(family="DejaVu Sans", size=12.0, bold=True)
    geo = text_to_geometry("Hi", font_config=fc)
    assert geo is not None
    assert len(geo.data) > 0


def test_text_to_geometry_multiple_chars():
    fc = FontConfig(family="DejaVu Sans", size=12.0)
    geo = text_to_geometry("Hi!", font_config=fc)
    assert geo is not None
    assert len(geo.data) > 0


def test_text_to_geometry_default_font():
    """Should work with default font_config (may return None if no
    system font named 'sans-serif' is found)."""
    geo = text_to_geometry("A")
    # Default may or may not find a font — just check it doesn't crash.
    assert geo is None or len(geo.data) > 0


def test_text_to_geometry_has_no_self_intersections():
    """Glyph outlines must be free of self-intersections and overlaps.

    Some system fonts contain such glyphs; they render fine but break
    winding analysis and offsetting downstream.
    """
    for family in ["sans-serif", "serif", "monospace"]:
        fc = FontConfig(family=family, size=12.0)
        geo = text_to_geometry("ABC abc 123 +=", font_config=fc)
        if geo is None:
            continue
        assert not geo.has_self_intersections(), (
            f"text in {family!r} has self-intersections"
        )
