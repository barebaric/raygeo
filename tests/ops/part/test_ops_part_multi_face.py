"""Tests for Part.from_geometry_multi_face (Step 2).

Verifies that:

* Single-pocket geometry produces a single default face ``""``.
* Two disconnected outer contours produce two faces; the largest is
  ``""`` and the second is ``"1"`` (sorted by area descending).
* Island (inner) contours are attached to the outer that contains
  their centroid, and do not leak into other faces.
* Empty geometry yields a single empty default face ``""`` (no panic),
  matching the single-face ``Part(...)`` constructor.
* ``face_ids`` exposes every face id.
"""

from raygeo.geo import Geometry
from raygeo.ops.part import Part


def _rect(x: float, y: float, w: float, h: float) -> Geometry:
    """A closed rectangular contour."""
    g = Geometry()
    g.move_to(x, y)
    g.line_to(x + w, y)
    g.line_to(x + w, y + h)
    g.line_to(x, y + h)
    g.line_to(x, y)
    return g


def _two_disjoint_outers_geo() -> Geometry:
    """Two separate rectangles: a large one and a smaller one."""
    g = Geometry()
    large = _rect(0, 0, 10, 10)
    small = _rect(20, 20, 4, 4)
    g.extend(large)
    g.extend(small)
    return g


def _two_outers_with_island_geo() -> Geometry:
    """A large outer with an island, plus a separate smaller outer."""
    g = Geometry()
    large = _rect(0, 0, 10, 10)
    island = _rect(2, 2, 2, 2)  # fully inside the large outer
    small = _rect(30, 30, 3, 3)  # disjoint
    g.extend(large)
    g.extend(island)
    g.extend(small)
    return g


# ── Single-pocket → single face ──────────────────────────────────


def test_single_outer_produces_single_default_face():
    p = Part.from_geometry_multi_face(_rect(0, 0, 5, 5), size_mm=(5.0, 5.0))
    assert p.face_ids == [""]


def test_single_outer_face_has_boundary():
    p = Part.from_geometry_multi_face(_rect(0, 0, 5, 5), size_mm=(5.0, 5.0))
    f = p.face("")
    assert f is not None
    assert len(f.stock_region.boundary) >= 3
    assert f.stock_region.islands == []


# ── Multi-pocket → multiple faces ───────────────────────────────


def test_two_disjoint_outers_produce_two_faces():
    p = Part.from_geometry_multi_face(
        _two_disjoint_outers_geo(), size_mm=(30.0, 30.0)
    )
    assert set(p.face_ids) == {"", "1"}


def test_largest_outer_becomes_default_face():
    p = Part.from_geometry_multi_face(
        _two_disjoint_outers_geo(), size_mm=(30.0, 30.0)
    )
    default = p.face("")
    second = p.face("1")
    assert default is not None and second is not None
    # The 10x10 outer (area 100) is larger than the 4x4 outer (area 16).
    # The largest pocket becomes the default face, so it must be the one
    # spanning 0..10; the smaller 20..24 pocket is face "1".
    default_pts = default.stock_region.boundary
    second_pts = second.stock_region.boundary
    assert any(x == 0.0 and y == 0.0 for x, y in default_pts)
    assert any(x == 10.0 for x, y in default_pts)
    assert any(x == 20.0 and y == 20.0 for x, y in second_pts)
    assert any(x == 24.0 for x, y in second_pts)


def test_each_face_has_its_own_boundary():
    p = Part.from_geometry_multi_face(
        _two_disjoint_outers_geo(), size_mm=(30.0, 30.0)
    )
    f0 = p.face("")
    f1 = p.face("1")
    assert f0 is not None and f1 is not None
    # Both faces have a real boundary; the default face's is bigger.
    assert len(f0.stock_region.boundary) >= 3
    assert len(f1.stock_region.boundary) >= 3


# ── Island association by centroid containment ──────────────────


def test_island_attaches_to_containing_outer():
    p = Part.from_geometry_multi_face(
        _two_outers_with_island_geo(), size_mm=(40.0, 40.0)
    )
    # Still exactly two faces (the island is an inner contour, not a face).
    assert set(p.face_ids) == {"", "1"}

    default = p.face("")
    second = p.face("1")
    assert default is not None and second is not None
    # The island is inside the large outer → it belongs to the default face.
    assert len(default.stock_region.islands) == 1
    # ... and the separate smaller face has no islands.
    assert second.stock_region.islands == []


def test_island_boundary_is_nonempty():
    p = Part.from_geometry_multi_face(
        _two_outers_with_island_geo(), size_mm=(40.0, 40.0)
    )
    default = p.face("")
    assert default is not None
    (island,) = default.stock_region.islands
    assert len(island) >= 3


# ── Empty geometry → single empty face ──────────────────────────


def test_empty_geometry_yields_single_default_face():
    p = Part.from_geometry_multi_face(Geometry(), size_mm=(1.0, 1.0))
    assert p.face_ids == [""]
    f = p.face("")
    assert f is not None
    assert f.stock_region.islands == []


# ── face_ids getter ──────────────────────────────────────────────


def test_face_ids_returns_all_faces():
    p = Part.from_geometry_multi_face(
        _two_disjoint_outers_geo(), size_mm=(30.0, 30.0)
    )
    assert sorted(p.face_ids) == ["", "1"]


def test_face_ids_single_face_for_single_part():
    p = Part.from_geometry_multi_face(_rect(0, 0, 5, 5), size_mm=(5.0, 5.0))
    assert p.face_ids == [""]
