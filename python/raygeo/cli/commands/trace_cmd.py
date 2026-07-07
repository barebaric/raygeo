import pathlib
import sys

from raygeo.cli.scenarios import SCENARIOS, build_scenario
from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.assembly.profile import profile_inner, profile_outer
from raygeo.ops.cut.cleared_area import ClearedArea


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


def register(subparsers):
    p = subparsers.add_parser(
        "trace",
        help="Run adaptive clearing or profiling and write a trace file.",
    )
    p.add_argument("tracefile", help="Output path for the .bin trace file.")
    p.add_argument(
        "--mode",
        default="adaptive",
        choices=["adaptive", "profile"],
        help="Operation mode (default: adaptive).",
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


def run(args):
    """Run adaptive clearing or profiling with tracing, write trace file."""
    trace_path = args.tracefile

    if args.mode == "profile":
        return _run_profile(args, trace_path)
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
            trace_path=trace_path,
        )
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
                trace_path=trace_path,
            )
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
                trace_path=trace_path,
            )
        print(
            f"  Profile: {result.ops.len()} ops, "
            f"{ca.total_area():.1f} mm² cleared"
        )
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        print("  Partial trace data was written to disk.")

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
