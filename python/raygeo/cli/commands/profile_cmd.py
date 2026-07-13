import time

from raygeo.cli.scenarios import SCENARIOS, build_scenario
from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.part import Part


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
    p.add_argument("--max-deflection-deg", type=float, default=None)
    p.add_argument("--wall-margin", type=float, default=None)
    p.add_argument("--cut-z", type=float, default=None)
    p.add_argument("--safe-z", type=float, default=None)
    p.add_argument("--area-tolerance", type=float, default=None)


def register(subparsers):
    p = subparsers.add_parser(
        "profile", help="Profile adaptive_clearing performance."
    )
    _add_scenario_args(p)
    p.set_defaults(func=run)


def run(args):
    scenario, seed_polys, _ = build_scenario(args)

    boundary = list(scenario.boundary)
    islands = [list(isl) for isl in scenario.islands]

    t0 = time.perf_counter()
    part = Part.from_polygons(boundary, islands, initial=seed_polys)
    result = adaptive_clearing(
        part,
        tool_radius=scenario.tool_radius,
        step_over=scenario.step_over,
        target_z=scenario.cut_z,
        safe_z=scenario.safe_z,
        max_deflection_deg=scenario.max_deflection_deg,
        wall_margin=scenario.wall_margin,
        area_tolerance=scenario.area_tolerance,
        expansion_batch_size=scenario.expansion_batch_size,
        cut_direction=scenario.cut_direction,
        profile=True,
    )
    t1 = time.perf_counter()

    cut = result.ops.count_cutting()
    travel = result.ops.count_travel()

    print(f"\n--- adaptive_clearing profile ({scenario.name}) ---")
    print(f"Wall clock:  {t1 - t0:.2f}s")
    print(f"Cut points:  {cut}")
    print(f"Travel ops:  {travel}")
    print()
