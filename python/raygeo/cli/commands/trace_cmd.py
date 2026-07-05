import pathlib
import sys

from raygeo.cli.scenarios import SCENARIOS, build_scenario
from raygeo.ops.assembly.adaptive import adaptive_clearing
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
        "trace", help="Run adaptive clearing and write a trace file."
    )
    p.add_argument("tracefile", help="Output path for the .bin trace file.")
    _add_trace_args(p)
    p.set_defaults(func=run)


def run(args):
    """Run adaptive entry + clearing with tracing, write trace file."""
    trace_path = args.tracefile

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
        clear_ops = adaptive_clearing(
            cleared=ca,
            pocket_boundary=list(scenario.boundary),
            islands=[list(isl) for isl in scenario.islands],
            radius=scenario.tool_radius,
            advance=scenario.advance,
            cut_z=scenario.cut_z,
            safe_z=scenario.safe_z,
            step_length=scenario.step_length,
            max_deflection_deg=scenario.max_deflection_deg,
            wall_margin=scenario.wall_margin,
            area_tolerance=scenario.area_tolerance,
            expansion_batch_size=scenario.expansion_batch_size,
            cut_direction=scenario.cut_direction,
            trace_path=trace_path,
        )
        print(
            f"  Clearing: {clear_ops.len()} ops, "
            f"{ca.total_area():.1f} mm² cleared, "
            f"{ca.remaining_area():.1f} mm² remaining"
        )
    except RuntimeError as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        print("  Partial trace data was written to disk.")

    mtime_after = tp.stat().st_mtime_ns if tp.exists() else 0
    if mtime_after == mtime_before:
        print(
            f"  ERROR: Trace file '{trace_path}' was not written.",
            file=sys.stderr,
        )
        sys.exit(1)
    print(f"  Trace written: {trace_path}")
