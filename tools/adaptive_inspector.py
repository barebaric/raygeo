#!/usr/bin/env python
"""Interactive adaptive clearing inspector.

Three subcommands:

    trace   — run adaptive clearing and write a trace file.
    inspect — open the interactive viewer for a trace file.
    print   — dump all trace records as an event log.

Usage::

    python tools/adaptive_inspector.py trace /tmp/trace.bin
    python tools/adaptive_inspector.py trace /tmp/tr.bin
    python tools/adaptive_inspector.py trace /tmp/tr.bin \
        --scenario centre-island
    python tools/adaptive_inspector.py trace /tmp/tr.bin \
        --svg logo.svg --tool-radius 2.5
    python tools/adaptive_inspector.py inspect /tmp/trace.bin
    python tools/adaptive_inspector.py inspect /tmp/trace.bin 500
    python tools/adaptive_inspector.py print /tmp/trace.bin

Controls (inspect):
     TextBox + Go button  — jump to any step number
     ◀ / ▶ buttons        — previous / next step
     ◀◀ Seg / Seg ▶▶  — previous / next segment
     Left / Right arrows   — previous / next step
     Shift+Left / Right    — previous / next segment
     Home / End            — first / last step
"""

import argparse
import math
import pathlib
import sys
from pathlib import Path as _Path

sys.path.insert(0, str(_Path(__file__).resolve().parent.parent))

import matplotlib.pyplot as plt

from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.cut.cleared_area import ClearedArea
from tools.cli.inspector import Inspector
from tools.cli.scenarios import (
    SCENARIOS,
    build_scenario,
)
from tools.cli.trace import (
    KIND_NAMES,
    RESUME_SOURCE_NAMES,
    ROUTE_DETAIL_LABELS,
    ROUTE_SOURCE_NAMES,
    STATUS_NAMES,
    TraceFile,
)

# ── Subcommands ──────────────────────────────────────────────────


def cmd_print(args: argparse.Namespace) -> None:
    """Dump all trace records as a human-readable event log."""
    trace_path = args.tracefile
    print(f"Trace file: {trace_path}")

    trace = TraceFile(trace_path)
    n = len(trace)
    print(f"Records: {n}")
    print()

    geo = trace.geometry
    tp = trace.toolpath
    print("Geometry:")
    print(f"  tool_radius={geo.tool_radius}")
    print(f"  boundary: {len(geo.boundary)} verts")
    print(f"  islands: {len(geo.islands)}")
    print(f"  seeds: {len(geo.seeds)} polygon(s)")
    print(f"  toolpath: {len(tp)} moves")
    print()

    for i in range(n):
        rec = trace[i]
        kind_name = KIND_NAMES.get(rec.kind, str(rec.kind))
        status_name = STATUS_NAMES.get(rec.status, str(rec.status))

        h_deg = math.degrees(rec.heading)
        sh_deg = math.degrees(rec.smoothed_heading)
        pa_deg = math.degrees(rec.predicted_angle)
        ia_deg = math.degrees(rec.iteration_angle)
        eng_deg = math.degrees(rec.eng_angle)
        step_dist = math.hypot(rec.pos_x - rec.prev_x, rec.pos_y - rec.prev_y)

        route_src = ""
        if rec.route_source:
            rs = ROUTE_SOURCE_NAMES.get(
                rec.route_source, str(rec.route_source)
            )
            route_src = f" route={rs}"

        resume_src = ""
        if rec.kind in (2, 3, 4) and rec.resume_source:
            rs = RESUME_SOURCE_NAMES.get(
                rec.resume_source, str(rec.resume_source)
            )
            resume_src = f" resume_via={rs}"

        print(
            f"{i}\t{kind_name}\t{status_name}{route_src}{resume_src}"
            f"\tpos=({rec.pos_x:.4f},{rec.pos_y:.4f})"
            f"\tprev=({rec.prev_x:.4f},{rec.prev_y:.4f})"
            f"\tdist={step_dist:.4f}"
            f"\thdg={h_deg:.4f}"
            f"\tsmooth={sh_deg:.4f}"
            f"\tpred={pa_deg:.4f}"
            f"\titer={ia_deg:.4f}"
            f"\teng_angle={eng_deg:.4f}"
            f"\teng_area={rec.eng_area:.4f}"
            f"\teng_chord={rec.eng_chord:.4f}"
            f"\tcut_area={rec.cut_area:.4f}"
            f"\ttotal_area={rec.total_area:.4f}"
            f"\trem_area={rec.remaining_area:.4f}"
            f"\titers={rec.iters}"
            f"\tops_len={rec.ops_len}"
            f"\tstrat="
            + "|".join(
                "WSMFEI"[i]
                + (":" + [".", "X", "B"][v] if v <= 2 else ":?")
                + (
                    f"[{rec.resume_strategy_details[i]}]"
                    if rec.resume_strategy_details[i]
                    else ""
                )
                for i, v in enumerate(rec.resume_strategy_reasons)
            )
            + "\trout="
            + "|".join(
                "DFMA"[i]
                + ":"
                + ROUTE_DETAIL_LABELS.get(
                    rec.route_strategy_details[i],
                    str(rec.route_strategy_details[i]),
                )
                for i in range(4)
            )
        )


def cmd_trace(args: argparse.Namespace) -> None:
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
            f"  ERROR: Trace file '{trace_path}' was not written.\n"
            f"  Tracing requires a debug build. Run:\n"
            f"    make dev  # install debug build (use the dev venv)"
        )
        sys.exit(1)
    print(f"  Trace written: {trace_path}")


def cmd_inspect(args: argparse.Namespace) -> None:
    """Open the interactive viewer for a trace file."""
    trace_path = args.tracefile
    initial_step = args.step or 0

    print(f"Loading trace from {trace_path}")
    trace = TraceFile(trace_path)
    print(f"  {len(trace)} trace records")

    geo = trace.geometry
    tp = trace.toolpath
    print(
        f"  geometry: tool_radius={geo.tool_radius}  "
        f"boundary={len(geo.boundary)} verts  "
        f"islands={len(geo.islands)}  "
        f"seeds={len(geo.seeds)}"
    )
    print(f"  toolpath: {len(tp)} moves")

    seed_polys = geo.seeds

    inspector = Inspector(trace, tp, seed_polys, geo)
    if initial_step > 0:
        inspector._draw(initial_step)
    plt.show()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Interactive adaptive clearing inspector."
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_trace = sub.add_parser(
        "trace", help="Run adaptive clearing and write a trace file."
    )
    p_trace.add_argument(
        "tracefile", help="Output path for the .bin trace file."
    )
    p_trace.add_argument(
        "--scenario",
        default="default",
        choices=list(SCENARIOS),
        help="Pocket scenario to use (default: default).",
    )
    p_trace.add_argument(
        "--svg",
        type=str,
        default=None,
        help="Path to SVG file. Overrides --scenario. "
        "First outer contour becomes pocket boundary; holes become islands.",
    )
    p_trace.add_argument("--tool-radius", type=float, default=None)
    p_trace.add_argument("--advance", type=float, default=None)
    p_trace.add_argument("--step-over", type=float, default=None)
    p_trace.add_argument("--step-length", type=float, default=None)
    p_trace.add_argument("--max-deflection-deg", type=float, default=None)
    p_trace.add_argument("--wall-margin", type=float, default=None)
    p_trace.add_argument("--cut-z", type=float, default=None)
    p_trace.add_argument("--safe-z", type=float, default=None)
    p_trace.add_argument("--area-tolerance", type=float, default=None)
    p_trace.set_defaults(func=cmd_trace)

    p_inspect = sub.add_parser(
        "inspect", help="Open the interactive viewer for a trace file."
    )
    p_inspect.add_argument(
        "tracefile", help="Input path for the .bin trace file."
    )
    p_inspect.add_argument(
        "step",
        nargs="?",
        type=int,
        default=0,
        help="Initial step to display (default: 0).",
    )
    p_inspect.set_defaults(func=cmd_inspect)

    p_print = sub.add_parser(
        "print", help="Dump all trace records as an event log."
    )
    p_print.add_argument(
        "tracefile", help="Input path for the .bin trace file."
    )
    p_print.set_defaults(func=cmd_print)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
