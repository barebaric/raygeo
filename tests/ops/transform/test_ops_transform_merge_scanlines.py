import math
import time

import pytest

from raygeo.ops import Ops
from raygeo.ops.transform.merge_scanlines import MergeScanlinesSpec
from raygeo.ops.types import CommandType, RasterMode, SectionType

# Machine profile with a modest acceleration so that bridging gaps
# pays off (the #495 use case: low-acceleration machines lose most
# of their time to stop/start cycles between array elements).
ACCEL = 500.0
CUT_SPEED = 6000
RAPID_SPEED = 12000


def make_spec(**kwargs):
    return MergeScanlinesSpec(
        acceleration=kwargs.pop("acceleration", ACCEL),
        cut_speed=kwargs.pop("cut_speed", CUT_SPEED),
        rapid_speed=kwargs.pop("rapid_speed", RAPID_SPEED),
        max_gap_mm=kwargs.pop("max_gap_mm", 0.0),
        tolerance=kwargs.pop("tolerance", 0.05),
        **kwargs,
    )


def apply(ops, spec=None):
    spec = spec or make_spec()
    Ops.apply_transformers(ops, [spec], progress_cb=None)


def scanline_types(ops):
    return [ops.command_type(i) for i in range(ops.len())]


def command_sequence(ops):
    return scanline_types(ops)


def scanline_values(ops, idx):
    return list(ops.scanline_data(idx))


def add_scanline(ops, x0, x1, y, values, z=0.0):
    ops.move_to(x0, y, z)
    ops.scan_to(x1, y, z, power_values=values)


def add_array_workpiece(
    ops, name, x0, rows, row_len=10.0, samples=10, y0=0.0, row_interval=1.0
):
    """Add one full-sweep raster 'workpiece' with *rows* scanlines."""
    ops.workpiece_start(name)
    ops.ops_section_start(
        SectionType.RASTER_FILL,
        name,
        raster_mode=RasterMode.VARIABLE_POWER,
    )
    for r in range(rows):
        y = y0 + r * row_interval
        values = [100 + (r % 100)] * samples
        if r % 2 == 0:
            add_scanline(ops, x0, x0 + row_len, y, values)
        else:
            add_scanline(ops, x0 + row_len, x0, y, values)
    ops.ops_section_end(
        SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
    )
    ops.workpiece_end(name)


class TestBasic:
    def test_empty_ops(self):
        ops = Ops()
        apply(ops)
        assert ops.is_empty()

    def test_single_line_untouched(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        before = command_sequence(ops)
        apply(ops)
        assert command_sequence(ops) == before

    def test_no_cost_model_and_no_ceiling_means_no_merge(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        add_scanline(ops, 12, 22, 0, [128] * 10)
        apply(ops, make_spec(acceleration=0.0))
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 2


class TestScanlineMerging:
    def test_array_elements_merge_into_shared_rows(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_array_workpiece(ops, "wp-a", 0.0, rows=2)
        add_array_workpiece(ops, "wp-b", 12.0, rows=2)
        apply(ops)

        scanlines = ops.indices_of(CommandType.SCAN_LINE)
        assert len(scanlines) == 2

        # Row 0 sweeps left-to-right across both elements.
        ops_idx = scanlines[0]
        assert ops.endpoint(ops_idx - 1) == (0.0, 0.0, 0.0)
        assert ops.endpoint(ops_idx) == (22.0, 0.0, 0.0)

        data = scanline_values(ops, ops_idx)
        # 2mm gap at 1 sample/mm => 2 zero samples in between.
        assert data[:10] == [100] * 10
        assert data[10:12] == [0, 0]
        assert data[12:] == [100] * 10

    def test_gap_too_expensive_not_merged(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        # Huge gap: dragging it at scan speed must lose against a
        # rapid move.
        add_scanline(ops, 0, 10, 0, [128] * 10)
        add_scanline(ops, 200, 210, 0, [128] * 10)
        apply(ops)
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 2

    def test_high_acceleration_reduces_merging(self):
        # The same 10mm gap merges at ACCEL (stop/start cycles are
        # expensive) but not when acceleration is high enough that a
        # rapid travel across the gap wins.
        def build():
            ops = Ops()
            ops.set_feed_rate(CUT_SPEED)
            ops.set_rapid_rate(RAPID_SPEED)
            add_scanline(ops, 0, 10, 0, [128] * 10)
            add_scanline(ops, 20, 30, 0, [128] * 10)
            return ops

        low_accel = build()
        apply(low_accel)
        assert len(low_accel.indices_of(CommandType.SCAN_LINE)) == 1

        high_accel = build()
        apply(high_accel, make_spec(acceleration=20000.0))
        assert len(high_accel.indices_of(CommandType.SCAN_LINE)) == 2

    def test_merging_that_loses_time_is_rejected(self):
        """Slow scan speeds with modest acceleration decline to merge.

        At 1010 mm/min and 1000 mm/s² the stop/start saving
        (v²/a ≈ 0.3mm worth of sweep) is smaller than sweeping a 1mm
        gap at scan speed, so the unit must pass through unchanged —
        merging would make the job slower.
        """
        ops = Ops()
        ops.set_feed_rate(1010)
        ops.set_rapid_rate(3000)
        add_scanline(ops, 0, 25, 0, [128] * 25)
        add_scanline(ops, 26, 51, 0, [128] * 25)
        before = command_sequence(ops)
        apply(ops, make_spec(acceleration=1000.0))
        assert command_sequence(ops) == before

        # The same geometry merges on a low-acceleration machine,
        # where stop/start cycles dominate.
        ops2 = Ops()
        ops2.set_feed_rate(1010)
        ops2.set_rapid_rate(3000)
        add_scanline(ops2, 0, 25, 0, [128] * 25)
        add_scanline(ops2, 26, 51, 0, [128] * 25)
        apply(ops2, make_spec(acceleration=100.0))
        assert len(ops2.indices_of(CommandType.SCAN_LINE)) == 1

    def test_manual_ceiling_overrides_the_cost_model(self):
        """A nonzero max gap is an explicit decision to bridge.

        Even where the cost model would decline (slow feed, modest
        acceleration), a manual ceiling must force the bridges —
        including bypassing the unit cost guard.
        """
        ops = Ops()
        ops.set_feed_rate(1010)
        ops.set_rapid_rate(3000)
        add_scanline(ops, 0, 25, 0, [128] * 25)
        add_scanline(ops, 26, 51, 0, [128] * 25)
        apply(ops, make_spec(acceleration=1000.0, max_gap_mm=2.0))
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 1

        # Without machine acceleration at all, the ceiling still
        # decides.
        ops2 = Ops()
        ops2.set_feed_rate(1010)
        ops2.set_rapid_rate(3000)
        add_scanline(ops2, 0, 25, 0, [128] * 25)
        add_scanline(ops2, 26, 51, 0, [128] * 25)
        apply(ops2, make_spec(acceleration=0.0, max_gap_mm=2.0))
        assert len(ops2.indices_of(CommandType.SCAN_LINE)) == 1

    def test_max_gap_ceiling_blocks_merge(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        add_scanline(ops, 12, 22, 0, [128] * 10)
        apply(ops, make_spec(max_gap_mm=1.0))
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 2

    def test_serpentine_rows_reverse_power_values(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        for x0, name, values in [
            (0.0, "wp-a", list(range(100, 110))),
            (12.0, "wp-b", list(range(200, 210))),
        ]:
            ops.workpiece_start(name)
            ops.ops_section_start(
                SectionType.RASTER_FILL,
                name,
                raster_mode=RasterMode.VARIABLE_POWER,
            )
            # Row 0: L->R. Row 1: R->L (serpentine).
            add_scanline(ops, x0, x0 + 10, 0, values)
            add_scanline(ops, x0 + 10, x0, 1, values)
            ops.ops_section_end(
                SectionType.RASTER_FILL,
                raster_mode=RasterMode.VARIABLE_POWER,
            )
            ops.workpiece_end(name)
        apply(ops)

        scanlines = ops.indices_of(CommandType.SCAN_LINE)
        assert len(scanlines) == 2

        # Row 0 is emitted L->R: each segment's values run along the
        # traversal, so the array is the concatenation in cut order.
        row0 = scanline_values(ops, scanlines[0])
        assert row0 == list(range(100, 110)) + [0, 0] + list(range(200, 210))
        assert ops.endpoint(scanlines[0]) == (22.0, 0.0, 0.0)

        # Row 1 continues from the nearest end (22, 1) and runs R->L.
        # The values must still map to the same positions: the sample
        # at x=22 is wp-b's original first sample (200), not a mirror
        # of the row-0 profile.
        row1 = scanline_values(ops, scanlines[1])
        assert row1 == list(range(200, 210)) + [0, 0] + list(range(100, 110))
        assert ops.endpoint(scanlines[1]) == (0.0, 1.0, 0.0)

    def test_run_absorbs_whole_row(self):
        """A row of four segments merges into one sweep.

        Regression test: the extend-vs-close comparison must charge
        the accumulated run span on the "close" side. Comparing only
        the previous segment stopped runs after one bridge (pairs
        instead of full-width sweeps).
        """
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        for k in range(4):
            x0 = k * 11.0
            add_scanline(ops, x0, x0 + 10, 0, [128] * 10)
        apply(ops)

        scanlines = ops.indices_of(CommandType.SCAN_LINE)
        assert len(scanlines) == 1
        assert ops.endpoint(scanlines[0]) == (43.0, 0.0, 0.0)
        data = scanline_values(ops, scanlines[0])
        assert data == ([128] * 10 + [0]) * 3 + [128] * 10

    def test_overscan_padded_neighbours_merge_on_content(self):
        """Zero-power overscan pads neither overlap rows nor shrink gaps.

        Two 10mm contents 3mm apart, each padded by 2mm of zero power
        on both ends, so the padded extents overlap by 1mm. They must
        still merge: rows are keyed on the non-zero content, interior
        pads are trimmed, and the outer pads survive.
        """
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)

        def padded(x0):
            ops.move_to(x0 - 2, 0, 0)
            ops.scan_to(
                x0 + 12, 0, 0, power_values=[0] * 2 + [100] * 10 + [0] * 2
            )

        padded(0.0)
        padded(13.0)
        apply(ops)

        scanlines = ops.indices_of(CommandType.SCAN_LINE)
        assert len(scanlines) == 1
        assert ops.endpoint(scanlines[0]) == (25.0, 0.0, 0.0)
        data = scanline_values(ops, scanlines[0])
        assert data == [0] * 2 + [100] * 10 + [0] * 3 + [100] * 10 + [0] * 2

    def test_differing_sample_density_not_merged(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)  # 1 sample/mm
        add_scanline(ops, 12, 22, 0, [128] * 100)  # 10 samples/mm
        apply(ops)
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 2

    def test_differing_z_not_merged(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10, z=0.0)
        add_scanline(ops, 12, 22, 0, [128] * 10, z=1.0)
        apply(ops)
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 2

    def test_rotated_rows_merge(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        # Two 45-degree lines on the same row, separated along it.
        d = math.sqrt(2.0) / 2.0
        ops.move_to(0, 0, 0)
        ops.scan_to(10 * d, 10 * d, 0, power_values=[128] * 10)
        ops.move_to(12 * d, 12 * d, 0)
        ops.scan_to(22 * d, 22 * d, 0, power_values=[128] * 10)
        apply(ops)

        scanlines = ops.indices_of(CommandType.SCAN_LINE)
        assert len(scanlines) == 1
        end = ops.endpoint(scanlines[0])
        assert end[0] == pytest.approx(22 * d, abs=1e-6)
        assert end[1] == pytest.approx(22 * d, abs=1e-6)
        data = scanline_values(ops, scanlines[0])
        assert data == [128] * 10 + [0, 0] + [128] * 10

    def test_perpendicular_lines_not_merged(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        ops.move_to(5, -5, 0)
        ops.scan_to(5, 5, 0, power_values=[128] * 10)
        apply(ops)
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 2

    def test_different_rows_not_merged(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        add_scanline(ops, 12, 22, 1, [128] * 10)  # different row
        apply(ops)
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 2

    def test_overlapping_projections_passthrough(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        add_scanline(ops, 5, 15, 0, [128] * 10)  # overlaps the first
        before = command_sequence(ops)
        apply(ops)
        assert command_sequence(ops) == before

    def test_multisegment_paths_passthrough(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        ops.set_power(0.8)
        # A polyline path: not a single straight line.
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.line_to(20, 0, 0)
        # A mergeable single-line path in the same row.
        ops.move_to(22, 0, 0)
        ops.line_to(30, 0, 0)
        before = command_sequence(ops)
        apply(ops)
        assert command_sequence(ops) == before


class TestConstantPowerLines:
    def test_lines_bridged_at_zero_power(self):
        ops = Ops()
        ops.set_power(0.8)
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(12, 0, 0)
        ops.line_to(22, 0, 0)
        apply(ops)

        # One contiguous path: cut, zero-power bridge, cut.
        moves = ops.indices_of(CommandType.MOVE_TO)
        lines = ops.indices_of(CommandType.LINE_TO)
        assert len(moves) == 1
        assert len(lines) == 3
        assert lines == sorted(lines)

        powers = {}
        for idx in ops.indices_of(CommandType.SET_POWER):
            powers[idx] = ops.power(idx)
        # Power is established, dropped for the bridge, and the
        # bridge geometry spans the gap.
        bridge = lines[1]
        assert ops.endpoint(bridge) == (12.0, 0.0, 0.0)
        assert powers.get(bridge - 1) == 0.0
        assert any(p == pytest.approx(0.8) for p in powers.values())

    def test_different_powers_not_merged(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        ops.set_power(0.8)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.set_power(0.4)
        ops.move_to(12, 0, 0)
        ops.line_to(22, 0, 0)
        apply(ops)
        assert len(ops.indices_of(CommandType.MOVE_TO)) == 2


class TestMarkersAndSections:
    def test_workpiece_markers_collapsed_to_bookends(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_array_workpiece(ops, "wp-a", 0.0, rows=1)
        add_array_workpiece(ops, "wp-b", 12.0, rows=1)
        apply(ops)

        types = command_sequence(ops)
        assert types.count(CommandType.WORKPIECE_START) == 1
        assert types.count(CommandType.WORKPIECE_END) == 1
        assert types.count(CommandType.OPS_SECTION_START) == 1
        assert types.count(CommandType.OPS_SECTION_END) == 1

        # Workpiece markers wrap the section, as in the aggregate.
        start = types.index(CommandType.WORKPIECE_START)
        section = types.index(CommandType.OPS_SECTION_START)
        section_end = types.index(CommandType.OPS_SECTION_END)
        end = types.index(CommandType.WORKPIECE_END)
        assert start < section < section_end < end

    def test_unmerged_regions_keep_their_markers(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_array_workpiece(ops, "wp-a", 0.0, rows=1)
        add_array_workpiece(ops, "wp-b", 200.0, rows=1)  # too far
        apply(ops)

        types = command_sequence(ops)
        assert types.count(CommandType.WORKPIECE_START) == 2
        assert types.count(CommandType.WORKPIECE_END) == 2


class TestInvariants:
    def test_burn_energy_preserved(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_array_workpiece(ops, "wp-a", 0.0, rows=3, samples=13)
        add_array_workpiece(ops, "wp-b", 12.0, rows=3, samples=13)
        before = sum(
            sum(scanline_values(ops, i))
            for i in ops.indices_of(CommandType.SCAN_LINE)
        )
        apply(ops)
        after = sum(
            sum(scanline_values(ops, i))
            for i in ops.indices_of(CommandType.SCAN_LINE)
        )
        assert after == before

    def test_merged_job_is_faster_at_low_acceleration(self):
        def build():
            ops = Ops()
            ops.set_feed_rate(CUT_SPEED)
            ops.set_rapid_rate(RAPID_SPEED)
            add_array_workpiece(ops, "wp-a", 0.0, rows=10)
            add_array_workpiece(ops, "wp-b", 12.0, rows=10)
            return ops

        unmerged = build()
        merged = build()
        apply(merged)

        t_unmerged = unmerged.estimate_time(CUT_SPEED, RAPID_SPEED, ACCEL)
        t_merged = merged.estimate_time(CUT_SPEED, RAPID_SPEED, ACCEL)
        assert t_merged < t_unmerged

    def test_passthrough_is_verbatim(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        ops.set_power(0.5)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        ops.move_to(200, 0, 0)
        ops.line_to(210, 0, 0)
        before = [
            (ops.command_type(i), ops.endpoint(i)) for i in range(ops.len())
        ]
        apply(ops)
        after = [
            (ops.command_type(i), ops.endpoint(i)) for i in range(ops.len())
        ]
        assert after == before


class TestConvenienceMethod:
    def test_apply_merge_scanlines(self):
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        add_scanline(ops, 0, 10, 0, [128] * 10)
        add_scanline(ops, 12, 22, 0, [128] * 10)
        ops.apply_merge_scanlines(ACCEL, CUT_SPEED, RAPID_SPEED)
        assert len(ops.indices_of(CommandType.SCAN_LINE)) == 1


class TestPerformance:
    def test_100k_scanlines(self):
        """A quadratic or exponential implementation times out here.

        Four 'workpieces' of 25k scanlines each. The linear pipeline
        finishes this in a few seconds even in a debug build.
        """
        workpieces = 4
        rows = 25_000
        samples = 8
        ops = Ops()
        ops.set_feed_rate(CUT_SPEED)
        ops.set_rapid_rate(RAPID_SPEED)
        for w in range(workpieces):
            add_array_workpiece(
                ops,
                f"wp-{w}",
                x0=w * 12.0,
                rows=rows,
                row_len=10.0,
                samples=samples,
            )

        t0 = time.monotonic()
        apply(ops)
        elapsed = time.monotonic() - t0

        scanlines = ops.indices_of(CommandType.SCAN_LINE)
        assert len(scanlines) == rows
        # Generous bound: catches O(n²) blowups without flaking on
        # slow CI machines.
        assert elapsed < 60.0, f"merge_scanlines took {elapsed:.1f}s"
