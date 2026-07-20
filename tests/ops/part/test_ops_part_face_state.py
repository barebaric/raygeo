"""Tests for :class:`raygeo.ops.part.FaceState`.

Verifies that:
- ``add_face`` creates a new face with independent state.
- A face with geometry reflects its geometry and stock region.
- Running Contour on one face does not affect another face's state.
- The default face ``""`` always exists and matches convenience
  getters.
"""

from raygeo.geo import Geometry
from raygeo.ops.assembly.contour import contour
from raygeo.ops.part import Part


def _make_square_geo() -> Geometry:
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.line_to(0, 0)
    return g


# ── add_face / face API ──────────────────────────────────────────


def test_add_face_creates_independent_state():
    """Each face has its own cleared area."""
    p = Part(size_mm=(10.0, 10.0))
    p.add_face("face_a", None)
    p.add_face("face_b", None)

    fa = p.face("face_a")
    fb = p.face("face_b")
    assert fa is not None
    assert fb is not None
    assert len(fa.cleared.remaining(fa.stock_region)) == 0
    assert len(fb.cleared.remaining(fb.stock_region)) == 0


def test_face_with_geometry():
    """A face with geometry reflects its geometry and stock region."""
    p = Part(size_mm=(20.0, 20.0))
    geo = _make_square_geo()
    p.add_face("face_x", geo)

    fs = p.face("face_x")
    assert fs is not None
    assert fs.geometry is not None
    assert len(fs.stock_region.boundary) > 0


def test_contour_on_default_face_leaves_other_untouched():
    """Contour on the default face leaves another face's state unchanged."""
    geo_a = _make_square_geo()

    geo_b = Geometry()
    geo_b.move_to(20, 20)
    geo_b.line_to(30, 20)
    geo_b.line_to(30, 30)
    geo_b.line_to(20, 30)
    geo_b.line_to(20, 20)

    p = Part(geometry=geo_a, size_mm=(30.0, 30.0))
    p.add_face("face_b", geo_b)

    fb_before = p.face("face_b")
    assert fb_before is not None
    bnd_before = fb_before.stock_region.boundary
    assert len(bnd_before) > 0

    contour(p)

    fb_after = p.face("face_b")
    assert fb_after is not None
    assert fb_after.stock_region.boundary == bnd_before


# ── Default face backward compatibility ──────────────────────────


def test_default_face_exists():
    """A Part constructed with geometry has the default face."""
    g = _make_square_geo()
    p = Part(geometry=g, size_mm=(10.0, 10.0))
    f = p.face("")
    assert f is not None
    assert f.geometry is not None
    assert len(f.stock_region.boundary) > 0


def test_default_face_matches_convenience_getters():
    """The default face's state matches the Part-level convenience getters."""
    g = _make_square_geo()
    p = Part(geometry=g, size_mm=(10.0, 10.0))
    f = p.face("")

    assert f is not None
    assert (f.geometry is not None) == (p.geometry is not None)
    if f.geometry is not None and p.geometry is not None:
        assert repr(f.geometry) == repr(p.geometry)
    assert len(f.stock_region.boundary) == len(p.stock_region.boundary)
    assert len(f.stock_region.islands) == len(p.stock_region.islands)
