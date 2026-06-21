"""Tests for A* pathfinding on a rasterised grid."""

from raygeo.geo.algo.astar import find_path


def _rect(x1, y1, x2, y2):
    """Axis-aligned rectangle polygon."""
    return [(x1, y1), (x2, y1), (x2, y2), (x1, y2)]


def test_simple_clear_path():
    """Open rectangle with no obstacles — path is a straight line."""
    free = [_rect(0, 0, 100, 50)]
    result = find_path(
        from_=(10, 25),
        to=(90, 25),
        free_space=free,
        obstacles=[],
        cell_size=2.0,
    )
    assert result is not None
    assert len(result.waypoints) >= 2
    # start and end should be near the requested positions
    assert abs(result.waypoints[0][0] - 10) < 2.0
    assert abs(result.waypoints[-1][0] - 90) < 2.0
    assert result.visited > 0
    assert result.length > 0


def test_path_around_obstacle():
    """Obstacle in the middle forces a detour around it."""
    free = [_rect(0, 0, 100, 50)]
    # Obstacle that doesn't span full height, so path can go around
    obstacle = [_rect(30, 10, 70, 40)]
    result = find_path(
        from_=(5, 25),
        to=(95, 25),
        free_space=free,
        obstacles=obstacle,
        obstacle_margin=0.0,
        cell_size=2.0,
    )
    assert result is not None
    assert len(result.waypoints) >= 2
    # the straight-line path through the obstacle centre should be blocked
    for x, y in result.waypoints:
        assert not (30 < x < 70 and 10 < y < 40)


def test_obstacle_margin_dilation():
    """With obstacle_margin, path stays further from obstacle."""
    free = [_rect(0, 0, 100, 50)]
    obstacle = [_rect(40, 15, 60, 35)]
    result = find_path(
        from_=(5, 25),
        to=(95, 25),
        free_space=free,
        obstacles=obstacle,
        obstacle_margin=10.0,
        cell_size=2.0,
    )
    assert result is not None


def test_start_goal_on_same_cell():
    """When start and goal snap to the same cell, path is a single point."""
    free = [_rect(0, 0, 10, 10)]
    result = find_path(
        from_=(3, 3),
        to=(3.5, 3.5),
        free_space=free,
        obstacles=[],
        cell_size=2.0,
    )
    assert result is not None
    assert len(result.waypoints) == 1
    assert result.length == 0.0
    assert result.visited == 1


def test_empty_free_space_returns_none():
    """No walkable area — returns None."""
    result = find_path(
        from_=(0, 0),
        to=(10, 10),
        free_space=[],
        obstacles=[],
        cell_size=1.0,
    )
    assert result is None


def test_zero_cell_size_returns_none():
    """Invalid cell size — returns None."""
    free = [_rect(0, 0, 10, 10)]
    result = find_path(
        from_=(0, 0),
        to=(10, 10),
        free_space=free,
        obstacles=[],
        cell_size=0.0,
    )
    assert result is None


def test_start_outside_free_space():
    """Start point outside free space snaps to nearest free cell."""
    free = [_rect(10, 10, 50, 50)]
    result = find_path(
        from_=(0, 0),
        to=(40, 40),
        free_space=free,
        obstacles=[],
        cell_size=2.0,
    )
    assert result is not None
    assert len(result.waypoints) >= 2


def test_multiple_free_space_polygons():
    """Pathfinding with free_space defined by multiple overlapping polygons."""
    free = [
        _rect(0, 0, 60, 50),
        _rect(40, 0, 100, 50),
    ]
    result = find_path(
        from_=(10, 25),
        to=(90, 25),
        free_space=free,
        obstacles=[],
        cell_size=2.0,
    )
    assert result is not None
    assert len(result.waypoints) >= 2
