import dataclasses
import math
import pathlib

from raygeo.geo.shape.polygon import (
    get_polygon_signed_area,
    is_point_inside_polygon,
    offset_polygon,
)

# ── Helper geometry functions ────────────────────────────────────


def rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def circle_polygon(cx, cy, r, n=64):
    pts = []
    for i in range(n):
        a = 2.0 * math.pi * i / n
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


@dataclasses.dataclass
class Scenario:
    """Pocket geometry, cutting parameters, and seed polygons."""

    name: str
    boundary: list
    islands: list
    tool_radius: float
    advance: float
    cut_z: float
    safe_z: float
    area_tolerance: float
    step_over: float = 2.0
    step_length: float = 0.6
    max_deflection_deg: float = 30.0
    wall_margin: float = 0.0
    expansion_batch_size: int = 20
    cut_direction: str = "ccw"
    stock_to_leave: float = 0.0
    cut_feed_rate: int = 1000
    cut_power: float = 0.0


SCENARIOS: dict[str, Scenario] = {}


def register_scenario(scenario: Scenario) -> None:
    SCENARIOS[scenario.name] = scenario


# ── Built-in scenarios ───────────────────────────────────────────

register_scenario(
    Scenario(
        name="default",
        boundary=[(0, 0), (180, 0), (180, 120), (0, 120)],
        islands=[
            [(15, 15), (35, 15), (35, 35), (15, 35)],
            [(70, 40), (90, 40), (90, 60), (70, 60)],
            [(130, 80), (160, 80), (160, 105), (130, 105)],
        ],
        tool_radius=3.0,
        advance=2.0,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=1.0,
    )
)

register_scenario(
    Scenario(
        name="centre-island",
        boundary=rect(0, 0, 60, 60),
        islands=[rect(5, 0, 10, 10)],
        tool_radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=1.0,
        step_length=0.6,
        max_deflection_deg=30.0,
        wall_margin=0.0,
    )
)

register_scenario(
    Scenario(
        name="entry-island",
        boundary=rect(0, 0, 60, 60),
        islands=[rect(5, 0, 10, 10)],
        tool_radius=3.0,
        advance=1.5,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=2.0,
        step_length=0.6,
        max_deflection_deg=30.0,
        wall_margin=0.0,
    )
)

register_scenario(
    Scenario(
        name="island-routing",
        boundary=rect(25, 25, 50, 50),
        islands=[rect(25, 25, 10, 10)],
        tool_radius=3.0,
        advance=1.5,
        step_over=1.5,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=1.0,
        step_length=0.6,
        max_deflection_deg=30.0,
        wall_margin=0.0,
    )
)

register_scenario(
    Scenario(
        name="trace-with-islands",
        boundary=rect(0, 0, 30, 30),
        islands=[rect(8, 0, 6, 6)],
        tool_radius=3.0,
        advance=1.5,
        step_over=2.0,
        cut_z=-5.0,
        safe_z=2.0,
        area_tolerance=4.0,
    )
)


# ── SVG scenario loader ──────────────────────────────────────────


def scenario_from_svg(
    svg_source,
    tool_radius=3.0,
    advance=1.5,
    step_over=2.0,
    cut_z=-5.0,
    safe_z=2.0,
    area_tolerance=1.0,
):
    """Build a Scenario from an SVG document.

    For SVG with multiple contours, the **first** outer contour is used
    as the pocket boundary; its holes become islands.
    """
    import raygeo.svg as _svg

    p = pathlib.Path(svg_source)
    if p.exists():
        svg_str = p.read_text()
    else:
        svg_str = svg_source

    geoms = _svg.svg_string_to_geometries(svg_str)

    all_polys = []
    for g in geoms:
        polys = g.to_polygons(tolerance=0.1)
        all_polys.extend(polys)

    if not all_polys:
        raise ValueError("No polygons found in SVG")

    # Flip Y: SVG Y-down * math Y-up
    all_y = [y for p in all_polys for _, y in p]
    y_min, y_max = min(all_y), max(all_y)
    flipped = []
    for p in all_polys:
        flipped.append([(x, y_min + y_max - y) for x, y in p])

    # Separate outer (CW after flip = negative signed area) and inner
    outer = [p for p in flipped if get_polygon_signed_area(p) < -0.01]
    inner = [p for p in flipped if get_polygon_signed_area(p) >= 0.01]

    if not outer:
        raise ValueError("No outer contour found in SVG")

    boundary = outer[0]

    # Find holes inside the first boundary
    islands = [h for h in inner if is_point_inside_polygon(h[0], boundary)]

    return Scenario(
        name="svg",
        boundary=boundary,
        islands=islands,
        tool_radius=tool_radius,
        advance=advance,
        step_over=step_over,
        cut_z=cut_z,
        safe_z=safe_z,
        area_tolerance=area_tolerance,
    )


# ── Seed / entry helpers ────────────────────────────────────────


def run_entry(scenario):
    """Return (None, seed_polys) where seed_polys is the eroded boundary.

    Uses polygon offset to compute the initial cleared area.  Entry ops
    are None — Steps 12+ will build and execute the workplan.
    """
    eroded = offset_polygon(list(scenario.boundary), -scenario.tool_radius)
    if eroded:
        seed_polys = eroded
    else:
        seed_polys = [list(scenario.boundary)]
    return None, seed_polys


def build_scenario(args):
    """Build (scenario, seed_polys, entry_ops) from parsed CLI args."""
    if args.svg:
        scenario = scenario_from_svg(
            args.svg,
            tool_radius=args.tool_radius,
            advance=args.advance,
            step_over=args.step_over,
            cut_z=args.cut_z,
            safe_z=args.safe_z,
            area_tolerance=args.area_tolerance,
        )
        entry_ops, seed_polys = run_entry(scenario)
        print(
            f"  SVG scenario: boundary={len(scenario.boundary)} verts, "
            f"{len(scenario.islands)} islands"
        )

    elif args.scenario == "centre-island":
        scenario = SCENARIOS["centre-island"]
        if args.tool_radius is not None:
            scenario = dataclasses.replace(
                scenario, tool_radius=args.tool_radius
            )
        if args.advance is not None:
            scenario = dataclasses.replace(scenario, advance=args.advance)
        if args.step_over is not None:
            scenario = dataclasses.replace(scenario, step_over=args.step_over)
        if args.cut_z is not None:
            scenario = dataclasses.replace(scenario, cut_z=args.cut_z)
        if args.safe_z is not None:
            scenario = dataclasses.replace(scenario, safe_z=args.safe_z)
        if args.area_tolerance is not None:
            scenario = dataclasses.replace(
                scenario, area_tolerance=args.area_tolerance
            )
        if args.step_length is not None:
            scenario = dataclasses.replace(
                scenario, step_length=args.step_length
            )
        if args.max_deflection_deg is not None:
            scenario = dataclasses.replace(
                scenario, max_deflection_deg=args.max_deflection_deg
            )
        if args.wall_margin is not None:
            scenario = dataclasses.replace(
                scenario, wall_margin=args.wall_margin
            )

        seed_polys = [circle_polygon(-13.7, 13.7, 12.2, 64)]
        entry_ops = None
        print(
            "  Centre-island scenario: circle seed "
            "centre=(-13.7,13.7) radius=12.2"
        )

    elif args.scenario == "trace-with-islands":
        scenario = SCENARIOS["trace-with-islands"]
        if args.tool_radius is not None:
            scenario = dataclasses.replace(
                scenario, tool_radius=args.tool_radius
            )
        if args.advance is not None:
            scenario = dataclasses.replace(scenario, advance=args.advance)
        if args.step_over is not None:
            scenario = dataclasses.replace(scenario, step_over=args.step_over)
        if args.cut_z is not None:
            scenario = dataclasses.replace(scenario, cut_z=args.cut_z)
        if args.safe_z is not None:
            scenario = dataclasses.replace(scenario, safe_z=args.safe_z)
        if args.area_tolerance is not None:
            scenario = dataclasses.replace(
                scenario, area_tolerance=args.area_tolerance
            )

        seed_polys = [circle_polygon(0, 0, 5, 32)]
        entry_ops = None
        print(
            "  Trace-with-islands scenario: circle seed centre=(0,0) radius=5"
        )

    else:
        scenario = SCENARIOS.get(args.scenario)
        if scenario is None:
            raise ValueError(
                f"Unknown scenario: {args.scenario}. "
                f"Available: {', '.join(SCENARIOS)}"
            )
        if args.tool_radius is not None:
            scenario = dataclasses.replace(
                scenario, tool_radius=args.tool_radius
            )
        if args.advance is not None:
            scenario = dataclasses.replace(scenario, advance=args.advance)
        if args.step_over is not None:
            scenario = dataclasses.replace(scenario, step_over=args.step_over)
        if args.cut_z is not None:
            scenario = dataclasses.replace(scenario, cut_z=args.cut_z)
        if args.safe_z is not None:
            scenario = dataclasses.replace(scenario, safe_z=args.safe_z)
        if args.area_tolerance is not None:
            scenario = dataclasses.replace(
                scenario, area_tolerance=args.area_tolerance
            )

        entry_ops, seed_polys = run_entry(scenario)

    return scenario, seed_polys, entry_ops
