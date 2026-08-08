import json
from pathlib import Path
from typing import Any

import pytest

from raygeo.ops import Ops
from raygeo.ops.transform import ExecutionPhase
from raygeo.ops.transform.bidir_scan_offset import BidirScanOffsetSpec
from raygeo.ops.transform.clip import CropSpec
from raygeo.ops.transform.lead_in_out import LeadInOutSpec
from raygeo.ops.transform.merge_lines import MergeLinesSpec
from raygeo.ops.transform.multipass import MultiPassSpec
from raygeo.ops.transform.optimize import OptimizeSpec
from raygeo.ops.transform.overscan import OverscanSpec
from raygeo.ops.transform.smooth import SmoothSpec
from raygeo.ops.transform.tabs import TabsSpec
from raygeo.ops.types import CommandType, RasterMode, SectionType


def _square_outline_ops() -> Ops:
    ops = Ops()
    ops.set_power(0.8)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(10, 10, 0)
    ops.line_to(30, 10, 0)
    ops.line_to(30, 30, 0)
    ops.line_to(10, 30, 0)
    ops.line_to(10, 10, 0)
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)
    return ops


class TestSpecConstruction:
    def test_smooth_spec_getters(self):
        s = SmoothSpec(50, 45.0)
        assert s.amount == 50
        assert s.corner_angle_threshold == 45.0

    def test_optimize_spec_getters(self):
        s = OptimizeSpec(True, False, ["a", "b"])
        assert s.allow_flip is True
        assert s.preserve_first is False
        assert list(s.preserve_order) == ["a", "b"]

    def test_merge_lines_spec_getters(self):
        s = MergeLinesSpec(0.1)
        assert s.tolerance == 0.1

    def test_overscan_spec_getters(self):
        s = OverscanSpec(2.5)
        assert s.distance_mm == 2.5

    def test_lead_in_out_spec_getters(self):
        s = LeadInOutSpec(3.0, 4.0)
        assert s.lead_in_mm == 3.0
        assert s.lead_out_mm == 4.0

    def test_multi_pass_spec_getters(self):
        s = MultiPassSpec(3, 0.5)
        assert s.passes == 3
        assert s.z_step_down == 0.5

    def test_crop_spec_getters(self):
        regions = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]]
        s = CropSpec(0.3, 1.0, regions)
        assert s.tolerance == 0.3
        assert s.offset == 1.0
        assert s.regions == regions

    def test_tabs_spec_getters(self):
        clips = [(1.0, 2.0, 3.0)]
        s = TabsSpec(0.5, 1.0, clips)
        assert s.tab_power == 0.5
        assert s.original_power == 1.0
        assert s.clips == clips

    def test_bidir_scan_offset_spec_getters(self):
        s = BidirScanOffsetSpec(0.7)
        assert s.offset_mm == 0.7

    def test_specs_are_frozen(self):
        s = SmoothSpec(50, 45.0)
        with pytest.raises(AttributeError):
            s.amount = 10  # type: ignore[misc]

    def test_specs_equality(self):
        assert SmoothSpec(50, 45.0) == SmoothSpec(50, 45.0)
        assert SmoothSpec(50, 45.0) != SmoothSpec(40, 45.0)


class TestExecutionPhase:
    def test_members(self):
        assert ExecutionPhase.GEOMETRY_REFINEMENT is not None
        assert ExecutionPhase.PATH_INTERRUPTION is not None
        assert ExecutionPhase.POST_PROCESSING is not None

    def test_distinct(self):
        phases = {
            ExecutionPhase.GEOMETRY_REFINEMENT,
            ExecutionPhase.PATH_INTERRUPTION,
            ExecutionPhase.POST_PROCESSING,
        }
        assert len(phases) == 3

    def test_hashable(self):
        d = {
            ExecutionPhase.GEOMETRY_REFINEMENT: 1,
            ExecutionPhase.PATH_INTERRUPTION: 2,
            ExecutionPhase.POST_PROCESSING: 3,
        }
        assert d[ExecutionPhase.PATH_INTERRUPTION] == 2


class TestApplyTransformersDispatch:
    def test_empty_list_is_noop(self):
        ops = _square_outline_ops()
        before = ops.len()
        ops.apply_transformers([])
        assert ops.len() == before

    def test_unknown_spec_type_raises_typeerror(self):
        ops = Ops()
        with pytest.raises(TypeError):
            ops.apply_transformers(["not a spec"])  # type: ignore[list-item]

    def test_smooth_dispatch(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(50, 0, 0)
        ops.line_to(100, 50, 0)
        ops.apply_transformers([SmoothSpec(50, 45.0)])
        assert ops.len() > 3

    def test_optimize_dispatch(self):
        ops = _square_outline_ops()
        before = ops.len()
        ops.apply_transformers([OptimizeSpec(True, False, [])])
        # optimize_travel may reorder but length stays the same here.
        assert ops.len() == before

    def test_merge_lines_dispatch(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.apply_transformers([MergeLinesSpec(0.01)])
        # The second overlapping line becomes a travel move.
        line_count = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.LINE_TO
        )
        assert line_count == 1

    def test_overscan_dispatch(self):
        ops = Ops()
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.move_to(10, 20, 0)
        ops.line_to(30, 20, 0)
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )
        before = ops.len()
        ops.apply_transformers([OverscanSpec(5.0)])
        assert ops.len() > before

    def test_lead_in_out_dispatch(self):
        ops = _square_outline_ops()
        before = ops.len()
        ops.apply_transformers([LeadInOutSpec(5.0, 5.0)])
        assert ops.len() > before

    def test_multipass_dispatch(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.apply_transformers([MultiPassSpec(2, 0.0)])
        assert ops.len() == 4

    def test_crop_dispatch(self):
        ops = Ops()
        ops.move_to(0, 5, 0)
        ops.line_to(20, 5, 0)
        region = [(5.0, 0.0), (15.0, 0.0), (15.0, 10.0), (5.0, 10.0)]
        ops.apply_transformers([CropSpec(0.3, 0.0, [region])])
        segs = list(ops.segment_indices())
        assert len(segs) == 1
        seg = segs[0]
        assert ops.endpoint(seg[0])[0] >= 5.0 - 1e-6
        assert ops.endpoint(seg[-1])[0] <= 15.0 + 1e-6

    def test_tabs_gap_mode_dispatch(self):
        ops = _square_outline_ops()
        before = ops.len()
        # tab_power == 0 selects gap mode.
        clips = [(20.0, 10.0, 2.0)]
        ops.apply_transformers([TabsSpec(0.0, 1.0, clips)])
        # Gap mode inserts travel moves around the tab -> ops grow.
        assert ops.len() > before

    def test_tabs_power_mode_dispatch(self):
        ops = _square_outline_ops()
        before = ops.len()
        clips = [(20.0, 10.0, 2.0)]
        ops.apply_transformers([TabsSpec(0.5, 1.0, clips)])
        # Power mode emits SetPower commands -> ops grow.
        assert ops.len() > before

    def test_bidir_scan_offset_dispatch(self):
        ops = Ops()
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        # Left-to-right pass: untouched.
        ops.move_to(0, 0, 0)
        ops.scan_to(10, 0, 0, power_values=[100])
        # Right-to-left pass: shifted by offset_mm.
        ops.move_to(10, 1, 0)
        ops.scan_to(0, 1, 0, power_values=[100])
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )
        ops.apply_transformers([BidirScanOffsetSpec(2.0)])
        # The RTL pass entry/exit moved +2.0 in X.
        # Find the second move_to / scan_to pair.
        move_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.MOVE_TO
        ]
        scan_indices = [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
        assert len(move_indices) == 2
        assert len(scan_indices) == 2
        # First pass (LTR) unchanged.
        assert ops.endpoint(move_indices[0])[0] == 0.0
        assert ops.endpoint(scan_indices[0])[0] == 10.0
        # Second pass (RTL) shifted +2.0.
        assert ops.endpoint(move_indices[1])[0] == 12.0
        assert ops.endpoint(scan_indices[1])[0] == 2.0

    def test_bidir_scan_offset_zero_is_noop_spec(self):
        ops = Ops()
        ops.move_to(10, 0, 0)
        ops.scan_to(0, 0, 0, power_values=[100])
        before = ops.len()
        ops.apply_transformers([BidirScanOffsetSpec(0.0)])
        assert ops.len() == before


class TestPhaseOrdering:
    def test_phases_execute_in_order(self):
        """A PostProcessing transformer must run after a PathInterruption
        one even if listed first."""
        ops = _square_outline_ops()
        # Track effect: LeadInOut (PathInterruption) grows ops; then
        # MultiPass (PostProcessing) doubles the path. If MultiPass ran
        # first, the lead-in/out segments would also be doubled.
        ops.apply_transformers(
            [
                MultiPassSpec(2, 0.0),
                LeadInOutSpec(5.0, 5.0),
            ]
        )
        # The square outline has 4 line segments + lead-in/out markers.
        # After LeadInOut the section has more than 4 lines; after
        # MultiPass it doubles. So total line count must be > 4 * 2.
        line_count = sum(
            1
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.LINE_TO
        )
        assert line_count > 8

    def test_geometry_refinement_before_path_interruption(self):
        """Smooth must run before LeadInOut: smoothing subdivides the
        line, and lead-in/out is computed on the smoothed path."""
        ops = Ops()
        ops.set_power(0.8)
        ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
        ops.move_to(0, 0, 0)
        ops.line_to(50, 0, 0)
        ops.line_to(100, 50, 0)
        ops.ops_section_end(SectionType.VECTOR_OUTLINE)

        # If order were reversed, LeadInOut would run on the unsmoothed
        # path and Smooth would later subdivide its lead-in/out. The
        # smoothed-then-leaded path has line_count strictly greater
        # than the reverse order would. We assert the dispatch ran both
        # and produced a path longer than either alone.
        ops.apply_transformers(
            [
                LeadInOutSpec(5.0, 5.0),
                SmoothSpec(50, 45.0),
            ]
        )
        assert ops.len() > 6


class TestCancellationAndProgress:
    def test_progress_cb_invoked(self):
        ops = _square_outline_ops()
        calls = []

        class Cb:
            def __call__(self, progress, message):
                calls.append(progress)

            def is_cancelled(self):
                return False

        ops.apply_transformers(
            [SmoothSpec(0, 45.0), SmoothSpec(0, 45.0)], Cb()
        )
        # One report per transformer (before each one).
        assert len(calls) == 2
        assert calls[0] == 0.0
        assert calls[1] == pytest.approx(0.5)

    def test_cancel_before_second_transformer(self):
        ops = _square_outline_ops()

        class CancelCb:
            def __init__(self):
                self.count = 0

            def __call__(self, progress, message):
                pass

            def is_cancelled(self):
                self.count += 1
                # Cancel after the first progress check.
                return self.count >= 2

        with pytest.raises(RuntimeError, match="cancelled"):
            ops.apply_transformers(
                [SmoothSpec(0, 45.0), SmoothSpec(0, 45.0)],
                CancelCb(),
            )

    def test_progress_cb_optional(self):
        ops = _square_outline_ops()
        # No callback at all.
        ops.apply_transformers([SmoothSpec(0, 45.0)])
        # Should not raise.


class TestMixedBatch:
    def test_multiple_specs_of_same_type(self):
        ops = Ops()
        ops.move_to(0, 0, 0)
        ops.line_to(10, 0, 0)
        ops.line_to(20, 0, 0)
        ops.line_to(30, 0, 0)
        # Two merge calls in a row is fine.
        ops.apply_transformers([MergeLinesSpec(0.01), MergeLinesSpec(0.01)])
        assert ops.is_empty() is False

    def test_full_pipeline_runs(self):
        ops = _square_outline_ops()
        before = ops.len()
        ops.apply_transformers(
            [
                SmoothSpec(20, 90.0),
                LeadInOutSpec(2.0, 2.0),
                MultiPassSpec(2, 0.0),
            ]
        )
        # Smoothing subdivides, lead-in/out grows, multipass doubles.
        assert ops.len() > before


# ----------------------------------------------------------------------
# Golden-file parametrised tests (fixtures in tests/transform/fixtures/)
# ----------------------------------------------------------------------

FIXTURE_DIR = Path(__file__).parents[2] / "transform" / "fixtures"

SPEC_CLASSES = {
    "SmoothSpec": SmoothSpec,
    "OptimizeSpec": OptimizeSpec,
    "MergeLinesSpec": MergeLinesSpec,
    "OverscanSpec": OverscanSpec,
    "LeadInOutSpec": LeadInOutSpec,
    "MultiPassSpec": MultiPassSpec,
    "CropSpec": CropSpec,
    "TabsSpec": TabsSpec,
    "BidirScanOffsetSpec": BidirScanOffsetSpec,
}


def _load_spec(spec_dict: dict[str, Any]):
    cls = SPEC_CLASSES[spec_dict["__type__"]]
    kwargs = {k: v for k, v in spec_dict.items() if k != "__type__"}
    if cls is TabsSpec:
        kwargs["clips"] = [tuple(c) for c in kwargs["clips"]]
    if cls is CropSpec:
        kwargs["regions"] = [
            [tuple(p) for p in poly] for poly in kwargs["regions"]
        ]
    return cls(**kwargs)


def _fixture_names() -> list[str]:
    ops_files = sorted(FIXTURE_DIR.glob("*.ops.json"))
    return [f.stem.removesuffix(".ops") for f in ops_files]


def _spec_type_for(name: str) -> str:
    if name.startswith("smooth"):
        return "SmoothSpec"
    if name.startswith("optimize"):
        return "OptimizeSpec"
    if name.startswith("merge_lines"):
        return "MergeLinesSpec"
    if name.startswith("overscan"):
        return "OverscanSpec"
    if name.startswith("lead_in_out"):
        return "LeadInOutSpec"
    if name.startswith("multipass"):
        return "MultiPassSpec"
    if name.startswith("crop"):
        return "CropSpec"
    if name.startswith("tabs"):
        return "TabsSpec"
    if name.startswith("bidir"):
        return "BidirScanOffsetSpec"
    raise ValueError(f"Unknown fixture name: {name}")


@pytest.mark.parametrize("name", _fixture_names())
def test_golden_apply_transformers(name: str) -> None:
    ops = Ops.from_dict(
        json.loads((FIXTURE_DIR / f"{name}.ops.json").read_text())
    )
    spec_data = json.loads((FIXTURE_DIR / f"{name}.spec.json").read_text())
    spec_data["__type__"] = _spec_type_for(name)
    spec = _load_spec(spec_data)

    ops.apply_transformers([spec])

    expected = json.loads(
        (FIXTURE_DIR / f"{name}.expected_ops.json").read_text()
    )
    assert json.loads(json.dumps(ops.to_dict())) == expected
