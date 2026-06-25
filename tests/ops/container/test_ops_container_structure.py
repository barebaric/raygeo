from raygeo.ops import Ops


def test_subpath_indices():
    ops = Ops()
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops.move_to(100, 100)
    ops.line_to(110, 100)

    result = ops.subpath_indices()
    assert len(result) == 2
    assert result[0] == [0, 1, 2]
    assert result[1] == [3, 4]


def test_subpath_indices_empty():
    ops = Ops()
    assert ops.subpath_indices() == []


def test_subpath_indices_single():
    ops = Ops()
    ops.move_to(0, 0)
    assert ops.subpath_indices() == [[0]]
