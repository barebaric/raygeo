import pathlib
import sys

from raygeo.cli.scenarios import SCENARIOS, build_scenario
from raygeo.cnc.machining.entry import build_entry_workplan
from raygeo.cnc.machining.plan import Workplan
from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.feature.region import find_regions


def _add_trace_args(p):
    p.add_argument(
        "--scenario",
        default="default",
        choices=list(SCENARIOS),
        help="Pocket scenario to use (default: default).",
    )
    p.add_argument(
        "--svg",
        type=str,
        default=None,
        help="Path to SVG file. Overrides --scenario. "
        "First outer contour becomes pocket boundary; holes become islands.",
    )
    p.add_argument("--tool-radius", type=float, default=None)
    p.add_argument("--advance", type=float, default=None)
    p.add_argument("--step-over", type=float, default=None)
    p.add_argument("--step-length", type=float, default=None)
    p.add_argument("--max-deflection-deg", type=float, default=None)
    p.add_argument("--wall-margin", type=float, default=None)
    p.add_argument("--cut-z", type=float, default=None)
    p.add_argument("--safe-z", type=float, default=None)
    p.add_argument("--area-tolerance", type=float, default=None)
    p.add_argument(
        "--region",
        type=int,
        default=0,
        help="Which island-free sub-region to enter (--mode entry/workplan). "
        "Pockets with islands split into several regions; 0 = largest.",
    )


def register(subparsers):
    p = subparsers.add_parser(
        "trace",
        help="Run an operation and write a trace file.",
    )
    p.add_argument("tracefile", help="Output path for the .bin trace file.")
    p.add_argument(
        "--mode",
        default="adaptive",
        choices=["adaptive", "profile", "entry", "workplan"],
        help="Operation mode (default: adaptive). 'entry' traces the "
        "entry workflow only; 'workplan' traces entry + adaptive clear.",
    )
    p.add_argument(
        "--inner",
        action="store_true",
        default=False,
        help="Profile inner pocket wall (--mode profile only).",
    )
    p.add_argument(
        "--outer",
        action="store_true",
        default=False,
        help="Profile outer stock boundary (--mode profile only).",
    )
    _add_trace_args(p)
    p.set_defaults(func=run)


def _entry_steps(args, scenario):
    """Pick an island-free region and build the entry steps for it.

    ``build_entry_workplan`` requires a single wide sub-region (not the
    whole pocket), so decompose the pocket with ``find_regions`` and use
    the region's own entry point / inscribed radius.
    """
    boundary = [tuple(p) for p in scenario.boundary]
    islands = [[tuple(q) for q in isl] for isl in scenario.islands]
    regions = find_regions(boundary, islands, scenario.tool_radius)
    if not regions:
        print(
            "  ERROR: no entry region found (pocket too small for tool).",
            file=sys.stderr,
        )
        sys.exit(1)
    idx = max(0, min(args.region, len(regions) - 1))
    region_polygon, _area, entry_pt, r_max = regions[idx]
    steps = build_entry_workplan(
        region_polygon,
        entry_pt,
        r_max,
        islands,
        scenario.tool_radius,
        scenario.step_over,
        scenario.safe_z,
        scenario.cut_z,
    )
    return boundary, islands, entry_pt, r_max, steps, len(regions)


def run(args):
    """Run the requested operation with tracing, write a trace file."""
    trace_path = args.tracefile
    if args.mode == "profile":
        return _run_profile(args, trace_path)
    if args.mode == "entry":
        return _run_entry(args, trace_path)
    if args.mode == "workplan":
        return _run_workplan(args, trace_path)
    return _run_adaptive(args, trace_path)


def _run_adaptive(args, trace_path):
    """Run adaptive entry + clearing with tracing."""
    scenario, seed_polys, entry_ops = build_scenario(args)

    print(f"Running {scenario.name} scenario with tracing...")
    print(
        f"  tool_radius={scenario.tool_radius}  advance={scenario.advance}  "
        f"step_length={scenario.step_length}"
    )
    print(
        f"  boundary: {len(scenario.boundary)} verts  "
        f"islands: {len(scenario.islands)}"
    )
    print(
        f"  seeds: {len(seed_polys)} polygons "
        f"({sum(len(p) for p in seed_polys)} verts)"
    )

    ca = ClearedArea(
        boundary=list(scenario.boundary),
        islands=[list(isl) for isl in scenario.islands],
        initial=seed_polys,
    )

    if entry_ops is not None:
        print(f"  Entry: {entry_ops.len()} ops")

    tp = pathlib.Path(trace_path)
    mtime_before = tp.stat().st_mtime_ns if tp.exists() else 0

    try:
        clear_result = adaptive_clearing(
            cleared=ca,
            pocket_boundary=list(scenario.boundary),
            islands=[list(isl) for isl in scenario.islands],
            tool_radius=scenario.tool_radius,
            step_over=scenario.step_over,
            target_z=scenario.cut_z,
            safe_z=scenario.safe_z,
            max_deflection_deg=scenario.max_deflection_deg,
            wall_margin=scenario.wall_margin,
            area_tolerance=scenario.area_tolerance,
            expansion_batch_size=scenario.expansion_batch_size,
            cut_direction=scenario.cut_direction,
        )
        clear_result.write_trace(str(tp), "adaptive", "AdaptiveClear")
        print(
            f"  Clearing: {clear_result.ops.len()} ops, "
            f"{ca.total_area():.1f} mm² cleared, "
            f"{ca.remaining_area():.1f} mm² remaining"
        )
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        print("  Partial trace data was written to disk.")

    _check_trace_written(tp, mtime_before)


def _run_profile(args, trace_path):
    """Run profiling (inner or outer) with tracing."""
    scenario, seed_polys, _entry_ops = build_scenario(args)

    kind = "inner" if args.inner else "outer"
    print(f"Running profile_{kind} with tracing...")
    print(f"  tool_radius={scenario.tool_radius}")
    print(
        f"  boundary: {len(scenario.boundary)} verts  "
        f"islands: {len(scenario.islands)}"
    )

    ca = ClearedArea(
        boundary=list(scenario.boundary),
        islands=[list(isl) for isl in scenario.islands],
        initial=seed_polys,
    )

    tp = pathlib.Path(trace_path)
    mtime_before = tp.stat().st_mtime_ns if tp.exists() else 0

    try:
        if args.inner:
            result = profile_inner(
                cleared=ca,
                boundary=list(scenario.boundary),
                islands=[list(isl) for isl in scenario.islands],
                tool_radius=scenario.tool_radius,
                target_z=scenario.cut_z,
                safe_z=scenario.safe_z,
                step_length=scenario.step_length,
                wall_margin=scenario.wall_margin,
                stock_to_leave=scenario.stock_to_leave,
                cut_feed_rate=scenario.cut_feed_rate,
                cut_power=scenario.cut_power,
                cut_direction=scenario.cut_direction,
            )
            result.write_trace(str(tp), "profile", "ProfileInner")
        else:
            result = profile_outer(
                cleared=ca,
                boundary=list(scenario.boundary),
                tool_radius=scenario.tool_radius,
                step_over=scenario.step_over,
                target_z=scenario.cut_z,
                safe_z=scenario.safe_z,
                step_length=scenario.step_length,
                wall_margin=scenario.wall_margin,
                stock_to_leave=scenario.stock_to_leave,
                cut_feed_rate=scenario.cut_feed_rate,
                cut_power=scenario.cut_power,
                cut_direction=scenario.cut_direction,
            )
            result.write_trace(str(tp), "profile", "ProfileOuter")
        print(
            f"  Profile: {result.ops.len()} ops, "
            f"{ca.total_area():.1f} mm² cleared"
        )
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        print("  Partial trace data was written to disk.")

    _check_trace_written(tp, mtime_before)


def _run_entry(args, trace_path):
    """Run the entry workflow (helix/spiral/toroid) with tracing."""
    scenario, _seed_polys, _entry_ops = build_scenario(args)
    boundary, islands, entry_point, r_max, steps, n_regions = _entry_steps(
        args, scenario
    )

    kinds = ", ".join(sorted({s["kind"] for s in steps})) or "(none)"
    print("Running entry workflow with tracing...")
    print(
        f"  tool_radius={scenario.tool_radius}  "
        f"entry_point={entry_point}  r_max={r_max}  "
        f"region {args.region}/{n_regions}"
    )
    print(f"  boundary: {len(boundary)} verts  islands: {len(islands)}")
    print(f"  entry steps: {kinds}")

    wp = Workplan(boundary, islands, scenario.safe_z)
    wp.extend(steps)
    tp = pathlib.Path(trace_path)
    mtime_before = tp.stat().st_mtime_ns if tp.exists() else 0
    result = wp.execute(
        cut_feed_rate=scenario.cut_feed_rate,
        cut_power=scenario.cut_power,
        trace=trace_path,
    )
    print(f"  Entry: {result.ops.len()} ops")
    _check_trace_written(tp, mtime_before)


def _run_workplan(args, trace_path):
    """Run entry + adaptive clearing (full workplan) with tracing."""
    scenario, _seed_polys, _entry_ops = build_scenario(args)
    boundary, islands, entry_point, r_max, entry_steps, n_regions = (
        _entry_steps(args, scenario)
    )

    print("Running full workplan (entry + adaptive clear) with tracing...")
    print(
        f"  tool_radius={scenario.tool_radius}  "
        f"entry_point={entry_point}  r_max={r_max}  "
        f"region {args.region}/{n_regions}"
    )
    print(f"  boundary: {len(boundary)} verts  islands: {len(islands)}")

    adaptive_step = {
        "kind": "AdaptiveClear",
        "pocket_boundary": boundary,
        "islands": islands,
        "tool_radius": scenario.tool_radius,
        "step_over": scenario.step_over,
        "step_length": scenario.step_length,
        "target_z": scenario.cut_z,
        "safe_z": scenario.safe_z,
        "max_deflection_deg": scenario.max_deflection_deg,
        "wall_margin": scenario.wall_margin,
        "area_tolerance": scenario.area_tolerance,
        "angular_step": 0.1,
    }
    wp = Workplan(boundary, islands, scenario.safe_z)
    wp.extend(entry_steps)
    wp.extend([adaptive_step])
    tp = pathlib.Path(trace_path)
    mtime_before = tp.stat().st_mtime_ns if tp.exists() else 0
    result = wp.execute(
        cut_feed_rate=scenario.cut_feed_rate,
        cut_power=scenario.cut_power,
        trace=trace_path,
    )
    print(f"  Workplan: {result.ops.len()} ops")
    _check_trace_written(tp, mtime_before)


def _check_trace_written(tp, mtime_before):
    """Exit with error if the trace file was not written."""
    mtime_after = tp.stat().st_mtime_ns if tp.exists() else 0
    if mtime_after == mtime_before:
        print(
            f"  ERROR: Trace file '{tp}' was not written.",
            file=sys.stderr,
        )
        sys.exit(1)
    print(f"  Trace written: {tp}")
