"""Tests for material test grid assembly module."""

import pytest

from raygeo.ops.assembly.material_test_grid import (
    MaterialTestGridSpec,
    generate_material_test_grid,
    generate_material_test_grid_preview,
)


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


def test_generate_material_test_grid_speed_vs_offset():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Speed vs Offset",
        cols=3,
        rows=3,
    )
    assert result.ops.len() > 0


def test_generate_material_test_grid_speed_vs_offset_custom_range():
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        grid_mode="Speed vs Offset",
        cols=2,
        rows=2,
        min_offset=-1.0,
        max_offset=1.0,
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


# ── Label layout ─────────────────────────────────────────────────


def _label_geometry(ops):
    """Extract the text-label geometry from the grid ops."""
    for section in ops.sections():
        blocks = section.state_blocks_by_name(ops, "labels")
        if not blocks:
            continue
        label_ops = section.state_block_content(ops, blocks[0])
        return label_ops.to_geometry()
    raise AssertionError("material test grid produced no label block")


@pytest.mark.parametrize(
    "speed_range",
    [
        (100.0, 500.0),
        (1000.0, 25000.0),
    ],
)
def test_row_labels_do_not_overlap_axis_title(speed_range):
    """Row labels must stay clear of the vertical axis title.

    Regression test: the axis title used to sit at a fixed fraction of
    the left margin, so wide speed labels (e.g. five-digit values such
    as 25000) ran into it. The layout now accounts for the widest row
    label, so all label components are pairwise disjoint and stay within
    the workpiece bounds.
    """
    min_speed, max_speed = speed_range
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=5,
        rows=5,
        min_speed=min_speed,
        max_speed=max_speed,
        include_labels=True,
    )
    geo = _label_geometry(result.ops)
    min_x, min_y, max_x, max_y = geo.rect()
    assert min_x > 0.0, "axis title is glued to the left boundary"
    components = geo.split_into_components()
    assert len(components) > 1
    for i, a in enumerate(components):
        for b in components[i + 1 :]:
            assert not a.intersects_with(b), (
                "label components overlap - wide row labels collide with "
                "the axis title"
            )


def test_spec_defaults_to_mm_min_labels():
    """The default display unit for engraved speed labels is mm/min."""
    spec = MaterialTestGridSpec(size_mm=(200.0, 200.0))
    assert spec.speed_unit_label == "mm/min"
    assert spec.speed_label_factor == 1.0
    assert spec.speed_label_precision == 0


def test_spec_accepts_custom_speed_label_unit():
    """A custom display unit is stored on the spec."""
    spec = MaterialTestGridSpec(
        size_mm=(200.0, 200.0),
        speed_unit_label="in/min",
        speed_label_factor=25.4,
        speed_label_precision=1,
    )
    assert spec.speed_unit_label == "in/min"
    assert spec.speed_label_factor == 25.4
    assert spec.speed_label_precision == 1


def test_generate_accepts_speed_label_unit_params():
    """The generator accepts display-unit params and still produces ops."""
    result = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=3,
        rows=3,
        speed_unit_label="in/min",
        speed_label_factor=25.4,
        speed_label_precision=1,
    )
    assert result.ops.len() > 0


def test_speed_label_unit_changes_label_geometry():
    """Different display units produce different label geometry."""
    mm_min = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=4,
        rows=4,
        min_speed=100.0,
        max_speed=500.0,
    )
    in_min = generate_material_test_grid(
        size_mm=(200.0, 200.0),
        cols=4,
        rows=4,
        min_speed=100.0,
        max_speed=500.0,
        speed_unit_label="in/min",
        speed_label_factor=25.4,
        speed_label_precision=1,
    )
    assert mm_min.ops.len() > 0
    assert in_min.ops.len() > 0
    assert mm_min.ops.len() != in_min.ops.len()


def test_preview_accepts_speed_label_unit_params():
    """The preview generator accepts display-unit params."""
    img = generate_material_test_grid_preview(
        size_mm=(200.0, 200.0),
        speed_unit_label="in/min",
        speed_label_factor=25.4,
        speed_label_precision=1,
    )
    assert img.ndim == 3
    assert img.shape[2] == 4
