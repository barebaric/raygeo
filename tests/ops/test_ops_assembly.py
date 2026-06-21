"""Smoke tests for the ops::assembly submodule wiring.

After the move of ``polyline``, ``lead_in_out``, ``overscan``,
``tabs``, and ``raster`` into ``ops::assembly``, these tests verify
that every moved function is still reachable through the public
Python API and returns sane results.
"""

import numpy as np

from raygeo.ops import Ops
from raygeo.ops.polyline import (
    LinkStrategy,
    link_passes,
    polyline_to_ops,
)
from raygeo.ops.raster import (
    ScanMode,
    generate_scan_lines,
    rasterize_mask_lines,
)
from raygeo.ops.types import CommandType, SectionType

# --- polyline (assembly/polyline.rs) ---


def test_assembly_polyline_to_ops():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    ops = polyline_to_ops(points, move_first=True)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.MOVE_TO
    assert ops.command_type(1) == CommandType.LINE_TO


def test_assembly_polyline_no_move_first():
    points = [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]
    ops = polyline_to_ops(points, move_first=False)
    assert ops.len() == 3
    assert ops.command_type(0) == CommandType.LINE_TO


def test_assembly_link_passes():
    p1 = polyline_to_ops([(0.0, 0.0, 0.0), (5.0, 0.0, 0.0)], move_first=True)
    p2 = polyline_to_ops(
        [(10.0, 5.0, 0.0), (10.0, 10.0, 0.0)], move_first=True
    )
    linked = link_passes(
        [p1, p2], safe_z=10.0, strategy=LinkStrategy.STAY_DOWN
    )
    assert linked.len() >= p1.len() + p2.len()


# --- lead_in_out (assembly/lead_in_out.rs) ---


def test_assembly_apply_lead_in_out():
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp")
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops.line_to(0, 10)
    ops.line_to(0, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    original_len = ops.len()
    ops.apply_lead_in_out(5.0, 5.0)
    assert ops.len() > original_len


def test_assembly_lead_in_out_empty_no_op():
    ops = Ops()
    ops.apply_lead_in_out(5.0, 5.0)
    assert ops.is_empty()


# --- overscan (assembly/overscan.rs) ---


def test_assembly_apply_overscan():
    ops = Ops()
    ops.ops_section_start(SectionType.RASTER_FILL, "wp")
    ops.set_power(1.0)
    ops.move_to(0, 0)
    ops.scan_to(10, 0, 0)
    ops.move_to(10, 1)
    ops.scan_to(0, 1, 0)
    ops.ops_section_end(SectionType.RASTER_FILL)
    original_len = ops.len()
    ops.apply_overscan(1.0)
    assert ops.len() >= original_len


# --- tabs (assembly/tabs.rs) ---


def test_assembly_apply_tab_gaps():
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp")
    ops.set_power(1.0)
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops.line_to(0, 10)
    ops.line_to(0, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    original_len = ops.len()
    ops.apply_tab_gaps([(5, 0, 1.0)])
    assert ops.len() >= original_len


def test_assembly_apply_tab_power():
    ops = Ops()
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp")
    ops.set_power(1.0)
    ops.move_to(0, 0)
    ops.line_to(10, 0)
    ops.line_to(10, 10)
    ops.line_to(0, 10)
    ops.line_to(0, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    ops.apply_tab_power([(5, 0, 1.0)], tab_power=0.2, original_power=1.0)
    has_power_cmd = any(
        ops.command_type(i) == CommandType.SET_POWER for i in range(ops.len())
    )
    assert has_power_cmd


# --- raster (assembly/raster/) ---


def test_assembly_raster_generate_scan_lines():
    bbox = (0, 9, 0, 9)
    image_size = (10, 10)
    ppm = (10.0, 10.0)
    lines = generate_scan_lines(bbox, image_size, ppm, 0.1, 0.0, 0.0, 0.0)
    assert len(lines) >= 10


def test_assembly_raster_rasterize_mask_lines():
    mask = np.ones((10, 10), dtype=np.uint8)
    ops = rasterize_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
    assert not ops.is_empty()


def test_assembly_raster_empty_mask():
    mask = np.zeros((10, 10), dtype=np.uint8)
    ops = rasterize_mask_lines(mask, (10.0, 10.0), 0.0, 0.0, 0.1)
    assert ops.is_empty()


def test_assembly_raster_scan_mode_enum():
    assert ScanMode.SEGMENTED is not None
    assert ScanMode.FULL_SWEEP is not None
