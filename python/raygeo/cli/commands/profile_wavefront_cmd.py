import pathlib
import time

from raygeo.cli.scenarios import (
    SCENARIOS,
    build_scenario,
)
from raygeo.geo import Geometry
from raygeo.ops.assembly.wavefront import (
    adaptive_wavefronts_multi_pocket,
)
from raygeo.ops.part import Part
from raygeo.svg import svg_string_to_geometries


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
    p.add_argument("--step-over", type=float, default=None)
    p.add_argument("--area-tolerance", type=float, default=None)
    p.add_argument("--offset", type=float, default=None)
    p.add_argument("--precision", type=float, default=None)
    p.add_argument("--cut-feed-rate", type=int, default=None)
    p.add_argument("--cut-power", type=float, default=None)


def register(subparsers):
    p = subparsers.add_parser(
        "profile-wavefront",
        help=("Profile adaptive_wavefronts_multi_pocket performance."),
    )
    _add_scenario_args(p)
    p.set_defaults(func=run)


def _build_geometry(args):
    """Return (Geometry, name) from SVG or default scenario."""
    if args.svg:
        svg_path = pathlib.Path(args.svg)
        if not svg_path.exists():
            raise FileNotFoundError(f"SVG not found: {args.svg}")
        svg_str = svg_path.read_text()
        geoms = svg_string_to_geometries(svg_str)
        all_polys_raw = []
        for g in geoms:
            polys = g.to_polygons(tolerance=0.1)
            all_polys_raw.extend(polys)
        all_y = [y for p in all_polys_raw for _, y in p]
        y_min, y_max = min(all_y), max(all_y)
        all_polys = []
        for p in all_polys_raw:
            all_polys.append([(x, y_min + y_max - y) for x, y in p])
        all_geo = Geometry()
        for poly in all_polys:
            all_geo.move_to(poly[0][0], poly[0][1], 0.0)
            for x, y in poly[1:]:
                all_geo.line_to(x, y, 0.0)
            all_geo.close_path()
        name = svg_path.stem
        step = args.step_over or 2.0
        tol = args.area_tolerance or 0.01
        return all_geo, name, step, tol

    scenario, _, _ = build_scenario(args)
    boundary = list(scenario.boundary)
    islands = [list(isl) for isl in scenario.islands]
    geo = Geometry()
    for poly in [boundary] + islands:
        geo.move_to(poly[0][0], poly[0][1], 0.0)
        for x, y in poly[1:]:
            geo.line_to(x, y, 0.0)
        geo.close_path()
    step = args.step_over if args.step_over is not None else scenario.step_over
    tol = (
        args.area_tolerance
        if args.area_tolerance is not None
        else scenario.area_tolerance
    )
    return geo, scenario.name, step, tol


def run(args):
    geo, name, step, tol = _build_geometry(args)
    part = Part(geometry=geo, size_mm=(200, 100))

    t0 = time.perf_counter()
    result = adaptive_wavefronts_multi_pocket(
        part,
        step_over=step,
        offset_mm=args.offset or 0.0,
        area_tolerance=tol,
        precision=args.precision or 0.0,
        cut_feed_rate=args.cut_feed_rate or 500,
        cut_power=args.cut_power or 1.0,
        profile=True,
    )
    t1 = time.perf_counter()

    cut = result.ops.count_cutting()
    travel = result.ops.count_travel()

    print(f"\n--- adaptive_wavefronts_multi_pocket profile ({name}) ---")
    print(f"Wall clock:  {t1 - t0:.2f}s")
    print(f"Cut points:  {cut}")
    print(f"Travel ops:  {travel}")
    print()
