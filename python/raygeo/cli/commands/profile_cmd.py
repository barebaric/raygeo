import dataclasses
import sys
import time

from raygeo.cli.scenarios import SCENARIOS, build_scenario, circle_polygon
from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.cut.cleared_area import ClearedArea


def _add_scenario_args(p):
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
        help="Path to SVG file. Overrides --scenario.",
    )
    p.add_argument("--tool-radius", type=float, default=None)
    p.add_argument("--advance", type=float, default=None)
    p.add_argument("--step-over", type=float, default=None)
    p.add_argument("--step-length", type=float, default=None)


def register(subparsers):
    p = subparsers.add_parser(
        "profile", help="Profile adaptive_clearing performance."
    )
    _add_scenario_args(p)
    p.set_defaults(func=run)


def run(args):
    if args.svg:
        scenario, seed_polys, _ = build_scenario(args)
    else:
        scenario = SCENARIOS.get(args.scenario)
        if scenario is None:
            print(
                f"Unknown scenario: {args.scenario}. "
                f"Available: {', '.join(SCENARIOS)}",
                file=sys.stderr,
            )
            sys.exit(1)
        if args.tool_radius is not None:
            scenario = dataclasses.replace(
                scenario, tool_radius=args.tool_radius
            )
        if args.advance is not None:
            scenario = dataclasses.replace(scenario, advance=args.advance)
        if args.step_over is not None:
            scenario = dataclasses.replace(scenario, step_over=args.step_over)
        if args.step_length is not None:
            scenario = dataclasses.replace(
                scenario, step_length=args.step_length
            )
        seed_polys = [circle_polygon(-13.7, 13.7, 12.2)]

    boundary = list(scenario.boundary)
    islands = [list(isl) for isl in scenario.islands]

    ca = ClearedArea(
        boundary=boundary,
        islands=islands,
        initial=seed_polys,
    )

    t0 = time.perf_counter()
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        radius=scenario.tool_radius,
        advance=scenario.advance,
        cut_z=scenario.cut_z,
        safe_z=scenario.safe_z,
        step_length=scenario.step_length,
        profile=True,
    )
    t1 = time.perf_counter()

    cut = sum(1 for i in range(ops.len()) if ops.is_cutting(i))
    travel = sum(1 for i in range(ops.len()) if ops.is_travel(i))

    print(f"\n--- adaptive_clearing profile ({scenario.name}) ---")
    print(f"Wall clock:  {t1 - t0:.2f}s")
    print(f"Cut points:  {cut}")
    print(f"Travel ops:  {travel}")
    print()
