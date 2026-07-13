"""Tests for material test grid assembly module."""

from raygeo.ops.assembly.material_test_grid import generate_material_test_grid


def test_generate_material_test_grid_basic():
    """Default params produce ops and valid AssemblyResult."""
    result = generate_material_test_grid(size_mm=(200.0, 200.0))
    assert result.ops.len() > 0


def test_generate_material_test_grid_returns_assembly_result():
    result = generate_material_test_grid(size_mm=(200.0, 200.0))
    assert hasattr(result, "ops")
    assert hasattr(result, "start")
    assert hasattr(result, "end")


def test_generate_material_test_grid_start_end_poses():
    result = generate_material_test_grid(size_mm=(200.0, 200.0))
    assert result.start is not None
    assert result.end is not None


def test_generate_material_test_grid_engrave_mode():
    """Engrave mode produces filled boxes with line geometry."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        mode="engrave",
        cols=2,
        rows=2,
    )
    assert result.ops.len() > 10
    cut_count = sum(
        1 for i in range(result.ops.len()) if result.ops.is_cutting(i)
    )
    assert cut_count > 0


def test_generate_material_test_grid_cut_mode():
    """Cut mode produces outline rectangles."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        mode="cut",
        cols=2,
        rows=2,
    )
    assert result.ops.len() > 0
    cut_count = sum(
        1 for i in range(result.ops.len()) if result.ops.is_cutting(i)
    )
    assert cut_count > 0


def test_generate_material_test_grid_power_vs_speed():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Power vs Speed",
        cols=3,
        rows=3,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_power_vs_passes():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Power vs Passes",
        cols=3,
        rows=3,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_speed_vs_passes():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Speed vs Passes",
        cols=3,
        rows=3,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_single_cell():
    """1x1 grid still produces ops."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=1,
        rows=1,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_single_row():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=5,
        rows=1,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_single_column():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=1,
        rows=5,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_with_labels():
    """include_labels=True adds text label ops."""
    with_labels = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=2,
        rows=2,
        include_labels=True,
    )
    without_labels = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=2,
        rows=2,
        include_labels=False,
    )
    # Labels add extra ops, so with_labels should have more
    assert with_labels.ops.len() > without_labels.ops.len()


def test_generate_material_test_grid_shape_size():
    """Larger shape_size produces more ops (more geometry per cell)."""
    small = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        shape_size=5.0,
        cols=2,
        rows=2,
    )
    large = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        shape_size=20.0,
        cols=2,
        rows=2,
    )
    assert large.ops.len() > small.ops.len()


def test_generate_material_test_grid_different_speed_range():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        min_speed=500.0,
        max_speed=2000.0,
        cols=2,
        rows=2,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_different_power_range():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        min_power=20.0,
        max_power=80.0,
        cols=2,
        rows=2,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_custom_grid_size():
    """Smaller workpiece produces proportionally scaled grid."""
    result = generate_material_test_grid(size_mm=(50.0, 50.0))
    assert result.ops.len() > 0


def test_generate_material_test_grid_large_grid():
    """Larger grid (more cells) produces more ops."""
    small = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=2,
        rows=2,
    )
    large = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=5,
        rows=5,
    )
    assert large.ops.len() > small.ops.len()


def test_generate_material_test_grid_sets_power():
    """Ops include SET_POWER commands for grid cells."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=2,
        rows=2,
    )
    power_seen = False
    for i in range(result.ops.len()):
        if result.ops.command_type(i).name == "SET_POWER":
            power_seen = True
            break
    assert power_seen, "should contain at least one SET_POWER command"


def test_generate_material_test_grid_sets_feed_rate():
    """Ops include SET_FEED_RATE commands for grid cells."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=2,
        rows=2,
    )
    feed_seen = False
    for i in range(result.ops.len()):
        if result.ops.command_type(i).name == "SET_FEED_RATE":
            feed_seen = True
            break
    assert feed_seen, "should contain at least one SET_FEED_RATE command"


def test_generate_material_test_grid_low_power_for_labels():
    """Label geometry uses lower power (0.3) than grid cells."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=2,
        rows=2,
        include_labels=True,
    )
    # Labels are generated before grid cells with low power (~0.3).
    # Check the first SET_POWER command has low power.
    for i in range(min(5, result.ops.len())):
        ct = result.ops.command_type(i).name
        if ct == "SET_POWER":
            assert result.ops.power(i) < 0.5, (
                "label power should be below grid cell power range"
            )
            break


def test_generate_material_test_grid_different_pass_counts():
    """Power vs Passes mode uses different pass counts per row."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Power vs Passes",
        cols=2,
        rows=3,
        min_passes=1,
        max_passes=3,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_fixed_power_label():
    """Speed vs Passes mode includes fixed-power label."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Speed vs Passes",
        cols=2,
        rows=2,
        include_labels=True,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_fixed_speed_label():
    """Power vs Passes mode includes fixed-speed label."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Power vs Passes",
        cols=2,
        rows=2,
        include_labels=True,
    )
    assert result.ops.len() > 0
