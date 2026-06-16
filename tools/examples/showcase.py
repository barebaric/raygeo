"""Generate a composite showcase image for the README."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import to_hex

from raygeo.geo import Geometry
from raygeo.geo.algo import hull
from raygeo.geo.algo.nest2d.placement import place_parts
from raygeo.geo.algo.smooth import smooth_polyline
from raygeo.geo.shape.bezier import linearize_bezier_adaptive
from raygeo.geo.shape.polygon import get_polygon_convex_hull
from raygeo.ops.raster import ScanMode, rasterize_power_modulation
from raygeo.ops.types import CommandType
from tools.plot import make_pattern, plot_geometry


def _make_two_squares(h, w):
    img = np.zeros((h, w), dtype=bool)
    img[30:70, 30:70] = True
    img[130:170, 130:170] = True
    return img


def _plot_concave_hull(ax):
    height, width = 200, 200
    img = _make_two_squares(height, width)
    gravity = 0.5

    convex_geo = hull.get_enclosing_hull(img)
    concave_geo = hull.get_concave_hull(img, gravity=gravity)

    ax.imshow(
        img,
        origin="upper",
        cmap="Blues",
        alpha=0.3,
        extent=(0, width, height, 0),
    )
    if convex_geo is not None:
        plot_geometry(
            ax, convex_geo, color="tomato", label="Convex", linewidth=1.5
        )
    if concave_geo is not None:
        plot_geometry(
            ax, concave_geo, color="forestgreen", label="Concave", linewidth=2
        )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8)
    ax.set_title("Concave Hull", fontsize=10)


def _plot_arc_fitting(ax):
    n_arc = 30
    cx, cy, r = 0, 0, 30
    arc_pts = [
        (
            cx + r * math.cos(math.pi * i / n_arc),
            cy + r * math.sin(math.pi * i / n_arc),
        )
        for i in range(n_arc + 1)
    ]

    raw_geom = Geometry.from_points(arc_pts, close=False)
    fit_geom = raw_geom.fit_curves(3.0, arcs=True, beziers=False)

    ax.plot(
        [p[0] for p in arc_pts],
        [p[1] for p in arc_pts],
        "o-",
        color="tomato",
        markersize=4,
        linewidth=0.8,
        label="Original points",
    )
    plot_geometry(
        ax,
        fit_geom,
        color="forestgreen",
        linewidth=2.5,
        label=f"Fitted ({len(fit_geom)} cmds)",
    )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8)
    ax.set_title("Arc Fitting", fontsize=10)


def _plot_nesting(ax):
    size = 30
    sheet_w, sheet_h = 200, 200
    spacing = 2.0
    rng = np.random.default_rng(42)

    def _make_part(i):
        if i % 2 == 0:
            w = size * (0.5 + 0.5 * rng.random())
            h = size * (0.5 + 0.5 * rng.random())
            return [(0, 0), (w, 0), (w, h), (0, h)]
        else:
            leg_w = size * (0.3 + 0.3 * rng.random())
            leg_h = size * (0.3 + 0.3 * rng.random())
            body_w = size * (0.5 + 0.3 * rng.random())
            body_h = size * (0.5 + 0.3 * rng.random())
            return [
                (0, 0),
                (body_w, 0),
                (body_w, leg_h),
                (leg_w, leg_h),
                (leg_w, body_h),
                (0, body_h),
            ]

    n_parts = 10
    part_polys = [[_make_part(i)] for i in range(n_parts)]
    part_hulls = [
        [get_polygon_convex_hull(part_polys[i][0])] for i in range(n_parts)
    ]
    sheet_poly = [
        (0.0, 0.0),
        (sheet_w, 0.0),
        (sheet_w, sheet_h),
        (0.0, sheet_h),
    ]
    sheet_offsets = [(0.0, 0.0)]
    rotations = [0.0] * n_parts
    fh = [False] * n_parts
    fv = [False] * n_parts

    result = place_parts(
        part_polys,
        part_hulls,
        [sheet_poly],
        sheet_offsets,
        rotations,
        fh,
        fv,
        spacing=spacing,
    )

    ax.plot(
        [p[0] for p in sheet_poly] + [sheet_poly[0][0]],
        [p[1] for p in sheet_poly] + [sheet_poly[0][1]],
        color="black",
        linewidth=2,
        label="Sheet",
    )
    cmap = plt.get_cmap("tab10")
    if result:
        for pi, pl in enumerate(result[0]["placements"]):
            for poly in pl["polygons"]:
                px = [p[0] for p in poly] + [poly[0][0]]
                py = [p[1] for p in poly] + [poly[0][1]]
                color = to_hex(cmap(pi % 10))
                ax.fill(px, py, alpha=0.25, color=color)
                ax.plot(px, py, color=color, linewidth=1.5)
    ax.set_aspect("equal")
    ax.set_xlim(-spacing * 2, sheet_w + spacing * 2)
    ax.set_ylim(-spacing * 2, sheet_h + spacing * 2)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8, loc="upper right")
    ax.set_title("Nesting", fontsize=10)


def _plot_raster_power_modulation(ax):
    img_size = 64
    gray = make_pattern(img_size, img_size, "Radial")
    ppm = 10.0
    line_interval = 0.1

    alpha = np.full((img_size, img_size), 255, dtype=np.uint8)
    ops = rasterize_power_modulation(
        gray,
        alpha,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval,
        0.05,
        min_power=0.0,
        max_power=1.0,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )

    max_mm = img_size / ppm
    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    power_cmap = plt.get_cmap("plasma")

    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = ops.endpoint(i)
            continue
        if ct in (
            CommandType.SET_POWER,
            CommandType.SET_CUT_SPEED,
            CommandType.SET_TRAVEL_SPEED,
            CommandType.SET_LASER,
            CommandType.SET_FREQUENCY,
            CommandType.SET_PULSE_WIDTH,
            CommandType.ENABLE_AIR_ASSIST,
            CommandType.DISABLE_AIR_ASSIST,
        ):
            continue
        st = ops.state(i)
        pwr = st.power if st is not None and st.power is not None else 1.0
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=power_cmap(pwr),
                linewidth=1.2,
            )
            pos = ep
        elif ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            sd = ops.scanline_data(i)
            n = len(sd)
            if n > 0:
                xs = np.linspace(pos[0], ep[0], n)
                ys = np.linspace(pos[1], ep[1], n)
                power_arr = (
                    np.frombuffer(sd, dtype=np.uint8).astype(np.float64)
                    / 255.0
                )
                for j in range(n - 1):
                    ax.plot(
                        xs[j : j + 2],
                        ys[j : j + 2],
                        color=power_cmap(power_arr[j]),
                        linewidth=1.2,
                    )
            pos = ep

    ax.set_xlim(0, max_mm)
    ax.set_ylim(0, max_mm)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_title("Raster Power Modulation", fontsize=10)


def _plot_smooth(ax):
    n = 30
    pts = [
        (
            50 + 30 * math.cos(2 * math.pi * i / n) + (i % 3) * 5,
            50 + 30 * math.sin(2 * math.pi * i / n) + (i % 4) * 4,
        )
        for i in range(n)
    ]
    pts_3d = [(x, y, 0.0) for x, y in pts]
    smoothed = smooth_polyline(pts_3d, 100, 0.0, True)

    sx, sy = zip(*pts)
    ax.plot(
        sx + (sx[0],),
        sy + (sy[0],),
        color="gray",
        linewidth=1.5,
        label="Original",
    )
    ssx, ssy, _ = zip(*smoothed)
    ax.plot(
        ssx + (ssx[0],),
        ssy + (ssy[0],),
        color="tomato",
        linewidth=2.5,
        label="Smoothed",
    )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8)
    ax.set_title("Smoothing (amount=100)", fontsize=10)


def _plot_linearization(ax):
    p0, p1, p2, p3 = (0.0, 0.0), (5.0, 15.0), (15.0, -5.0), (20.0, 5.0)

    def _eval(n=100):
        ts = np.linspace(0, 1, n)
        pts = []
        for t in ts:
            u = 1 - t
            x = (
                u**3 * p0[0]
                + 3 * u**2 * t * p1[0]
                + 3 * u * t**2 * p2[0]
                + t**3 * p3[0]
            )
            y = (
                u**3 * p0[1]
                + 3 * u**2 * t * p1[1]
                + 3 * u * t**2 * p2[1]
                + t**3 * p3[1]
            )
            pts.append((x, y))
        return pts

    curve = _eval()
    flat = linearize_bezier_adaptive(p0, p1, p2, p3, 1.0, 10)

    xs = [p[0] for p in curve]
    ys = [p[1] for p in curve]
    ax.plot(xs, ys, color="gray", linewidth=1.5, label="Original")
    fxs = [p[0] for p in flat]
    fys = [p[1] for p in flat]
    ax.plot(
        fxs,
        fys,
        "-o",
        color="forestgreen",
        linewidth=2.5,
        markersize=3,
        label=f"tol=1 ({len(flat)} pts)",
    )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=8)
    ax.set_title("Linearization", fontsize=10)


def generate_examples(output_dir):
    fig, axes = plt.subplots(2, 3, figsize=(18, 12))

    _plot_concave_hull(axes[0, 0])
    _plot_arc_fitting(axes[0, 1])
    _plot_nesting(axes[0, 2])
    _plot_raster_power_modulation(axes[1, 0])
    _plot_smooth(axes[1, 1])
    _plot_linearization(axes[1, 2])

    fig.tight_layout()
    path = output_dir / "showcase.png"
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)

    return {
        "title": "Showcase",
        "description": "Composite showcase of key raygeo features",
        "images": [
            {
                "path": "showcase.png",
                "caption": "Concave hull, arc fitting, nesting, raster power"
                " modulation, smoothing, and linearization",
            }
        ],
    }
