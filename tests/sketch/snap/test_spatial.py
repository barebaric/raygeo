"""Tests for the snap line index."""

import pytest

from raygeo.sketch.snap.spatial import IndexedLine, SnapLineIndex
from raygeo.sketch.snap.types import SnapLine, SnapLineType


def _line(coordinate, is_horizontal=True, kind=SnapLineType.ENTITY_POINT):
    return SnapLine(
        is_horizontal=is_horizontal,
        coordinate=coordinate,
        line_type=kind,
        source=None,
    )


def test_index_keeps_entries_sorted():
    index = SnapLineIndex()
    for coordinate in (5.0, 1.0, 3.0):
        index.add(_line(coordinate))
    coords = [entry.coordinate for entry in index._horizontal]
    assert coords == [1.0, 3.0, 5.0]
    assert len(index) == 3


def test_index_query_horizontal_and_vertical():
    index = SnapLineIndex()
    index.add(_line(2.0, is_horizontal=True))
    index.add(_line(7.0, is_horizontal=False))

    horizontal = index.query_horizontal(2.5, 1.0)
    assert len(horizontal) == 1
    assert horizontal[0][1] == pytest.approx(0.5)

    vertical = index.query_vertical(6.0, 3.0)
    assert len(vertical) == 1

    assert index.query_horizontal(10.0, 1.0) == []


def test_query_sorts_by_distance_then_priority():
    index = SnapLineIndex()
    # Equal distances above and below the query point; the higher
    # priority kind must come first.
    index.add(_line(-1.0, kind=SnapLineType.CENTER))
    index.add(_line(1.0, kind=SnapLineType.ENTITY_POINT))

    results = index.query(0.0, 0.0, 2.0)
    assert len(results) == 2
    assert results[0][0].line_type.name == "ENTITY_POINT"
    assert results[1][0].line_type.name == "CENTER"


def test_index_clear_and_dirty():
    index = SnapLineIndex()
    index.add(_line(1.0))
    assert index._dirty is True
    index.clear()
    assert len(index) == 0
    assert index._dirty is False


def test_index_add_all_rejects_none():
    index = SnapLineIndex()
    with pytest.raises(Exception):
        index.add_all([None])
    index.add_all([_line(1.0), _line(2.0)])
    assert len(index) == 2


def test_indexed_line_value_object():
    entry = IndexedLine(snap_line=_line(3.0), coordinate=3.0)
    assert entry.coordinate == 3.0
    assert entry.snap_line.coordinate == 3.0
    assert IndexedLine(coordinate=1.0) < IndexedLine(coordinate=2.0)
