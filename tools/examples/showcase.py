"""Generate a composite showcase image for the README."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import to_hex

from raygeo.cnc.machining.plan import Workplan
from raygeo.cnc.machining.wavefront import build_wavefront_workplan
from raygeo.geo import Geometry
from raygeo.geo.algo import hull
from raygeo.geo.algo.cylindrical import transform_to_cylinder
from raygeo.geo.algo.helix import HelixDirection, generate_helix_3d
from raygeo.geo.algo.nest2d.placement import place_parts
from raygeo.geo.algo.smooth import smooth_polyline_3d
from raygeo.geo.shape.bezier import linearize_bezier_adaptive
from raygeo.geo.shape.polygon import (
    get_circle_polygon,
    get_polygon_convex_hull,
)
from raygeo.geo.shape.polygon3d import fillet_polyline_3d, offset_polyline_3d
from raygeo.image.scan import ScanMode
from raygeo.ops import Ops
from raygeo.ops.assembly.adaptive import adaptive_clearing
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.types import CommandType
from tools.plot import make_pattern, plot_geometry, plot_ops_2d


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
    ops = Ops.from_power_modulated_image(
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
        scan_mode=ScanMode.SEGMENTED,
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
            CommandType.SET_FEED_RATE,
            CommandType.SET_RAPID_RATE,
            CommandType.SET_HEAD,
            CommandType.SET_FREQUENCY,
            CommandType.SET_PULSE_WIDTH,
            CommandType.SET_COOLANT,
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
    smoothed = smooth_polyline_3d(pts_3d, 100, 0.0, True)

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


def _plot_cylindrical(ax):
    diameter = 20.0
    radius = diameter / 2.0

    verts = []
    for x in range(10, 50, 5):
        verts.extend([(x, -80, 0), (x, 80, 0)])
    for y in range(-60, 70, 20):
        verts.extend([(10, y, 0), (45, y, 0)])

    verts_np = np.array(verts, dtype=np.float32)
    transformed, _, _ = transform_to_cylinder(
        verts_np, diameter, colors=None, degrees_input=True
    )
    t = transformed.reshape(-1, 3)

    theta = np.linspace(-np.pi, np.pi, 32)
    z_cyl = np.linspace(5, 50, 20)
    th, zz = np.meshgrid(theta, z_cyl)
    xx = zz
    yy = radius * np.sin(th)
    zz2 = radius * np.cos(th)
    ax.plot_surface(
        xx, yy, zz2, alpha=0.1, color="gray", edgecolors="gray", linewidth=0.25
    )

    for i in range(0, len(t), 2):
        ax.plot(
            t[i : i + 2, 0],
            t[i : i + 2, 1],
            t[i : i + 2, 2],
            "tomato",
            linewidth=1.5,
        )

    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title("Cylindrical Transform", fontsize=10)
    ax.view_init(elev=25, azim=-65)
    ax.set_box_aspect((1.5, 1, 1))


def _plot_conical_helix(ax):
    pts = generate_helix_3d(
        center=(0, 0),
        start_radius=10,
        end_radius=30,
        z_start=0,
        z_end=-30,
        pitch=10,
        direction=HelixDirection.Cw,
        angular_step=0.05,
        min_revolutions=3,
    )
    xs, ys, zs = zip(*pts)
    ax.plot(xs, ys, zs, "crimson", linewidth=2)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title("Conical Helix", fontsize=10)
    ax.view_init(elev=25, azim=-60)


def _plot_3d_offset(ax):
    n = 16
    curve = []
    for i in range(n + 1):
        t = i / n
        a = t * math.pi / 2
        x = 6 * math.cos(a)
        y = 6 * math.sin(a)
        z = 6 - x / 3 + y / 3
        curve.append((x, y, z))
    off = offset_polyline_3d(curve, 1.2)

    xs, ys, zs = zip(*curve)
    ax.plot(xs, ys, zs, "o-", color="steelblue", linewidth=2, label="Original")
    xs_o, ys_o, zs_o = zip(*off)
    ax.plot(
        xs_o,
        ys_o,
        zs_o,
        "o-",
        color="tomato",
        linewidth=3,
        label="Offset",
        alpha=0.8,
    )
    for i in range(0, len(curve), 3):
        ax.plot(
            [curve[i][0], off[i][0]],
            [curve[i][1], off[i][1]],
            [curve[i][2], off[i][2]],
            color="gray",
            linewidth=1,
            linestyle=":",
        )
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title("3D Polyline Offset", fontsize=10)
    ax.view_init(elev=25, azim=-55)
    ax.legend(fontsize=8)


def _plot_adaptive_2d(ax):
    target_z = -5.0
    boundary = [(0, 0), (80, 0), (80, 80), (0, 80)]
    islands = [
        [(20, 20), (35, 20), (35, 35), (20, 35)],
        [(50, 50), (65, 50), (65, 65), (50, 65)],
    ]
    seed = get_circle_polygon((15, 65), 10, 48)
    ca = ClearedArea(boundary=boundary, islands=islands, initial=[seed])
    result = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=1.8,
        target_z=target_z,
        safe_z=2.0,
        area_tolerance=5.0,
    )
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.4)
    for island in islands:
        ix = [p[0] for p in island] + [island[0][0]]
        iy = [p[1] for p in island] + [island[0][1]]
        ax.fill(ix, iy, color="lightgray", alpha=0.5, linewidth=0)
        ax.plot(ix, iy, "k-", linewidth=1.2)
    sx = [p[0] for p in seed] + [seed[0][0]]
    sy = [p[1] for p in seed] + [seed[0][1]]
    ax.fill(sx, sy, color="steelblue", alpha=0.2, linewidth=0)
    ax.plot(sx, sy, color="steelblue", linewidth=1.2, linestyle="--")
    plot_ops_2d(
        ax,
        result.ops,
        boundary=boundary,
        islands=islands,
        mark_cut_start=False,
    )
    ax.set_title("Adaptive Clearing", fontsize=10)


def _plot_fillet_polyline_3d(ax):
    poly = [
        (0.0, 0.0, 0.0),
        (8.0, 0.0, 0.0),
        (8.0, 6.0, 3.0),
        (2.0, 6.0, 3.0),
        (2.0, 0.0, 6.0),
        (10.0, 0.0, 6.0),
    ]
    radius = 1.5
    result = fillet_polyline_3d(poly, radius)

    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    zs = [p[2] for p in poly]
    ax.plot(xs, ys, zs, "o-", color="steelblue", linewidth=2, label="Original")

    xs_r = [p[0] for p in result]
    ys_r = [p[1] for p in result]
    zs_r = [p[2] for p in result]
    ax.plot(
        xs_r,
        ys_r,
        zs_r,
        "o-",
        color="tomato",
        linewidth=3,
        alpha=0.85,
        label=f"Filleted (r={radius})",
    )

    if len(result) > len(poly):
        ax.plot(
            xs_r, ys_r, zs_r, "o", color="tomato", markersize=4, alpha=0.85
        )

    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.set_title("3D Fillet Polyline", fontsize=10)
    ax.view_init(elev=25, azim=-65)
    ax.legend(fontsize=8)


def _plot_adaptive_wavefronts(ax):
    """Wavefront contours in a pocket with islands."""
    boundary = [(0, 0), (180, 0), (180, 120), (0, 120)]
    islands = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [
            (
                80 + 10 * math.cos(2 * math.pi * i / 32),
                50 + 10 * math.sin(2 * math.pi * i / 32),
            )
            for i in range(32)
        ],
        [(130, 80), (160, 80), (160, 105), (130, 105)],
    ]
    steps = build_wavefront_workplan(
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=2.0,
        target_z=-5.0,
        area_tolerance=1.0,
    )
    wp = Workplan(boundary, islands=islands, safe_z=2.0)
    wp.extend(steps)
    result = wp.execute()

    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.3)

    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(
            ix,
            iy,
            facecolor="lightgray",
            edgecolor="gray",
            hatch="///",
            linewidth=1,
        )

    # Seed disk from the FlatSpiral step (the cleared core the
    # wavefronts expand from).
    seed = next(s for s in steps if s["kind"] == "FlatSpiral")
    scx, scy = seed["center"]
    sr = seed["end_radius"]
    stheta = np.linspace(0, 2 * np.pi, 65)
    ax.fill(
        scx + sr * np.cos(stheta),
        scy + sr * np.sin(stheta),
        "white",
        zorder=2,
    )
    ax.plot(
        scx + sr * np.cos(stheta),
        scy + sr * np.sin(stheta),
        "steelblue",
        linewidth=1,
        alpha=0.4,
        zorder=2,
    )

    # Wavefront rings: subpaths after the FlatSpiral seed path.
    subpaths = result.ops.split_into_subpaths()
    ring_subpaths = subpaths[1:] if len(subpaths) > 1 else subpaths
    n_wf = len(ring_subpaths)
    for i, sub in enumerate(ring_subpaths):
        t = i / max(n_wf - 1, 1)
        color = (0.9 - 0.6 * t, 0.2 + 0.5 * t, 0.2)
        pts = []
        for j in range(sub.len()):
            if sub.is_cutting(j):
                ep = sub.endpoint(j)
                pts.append((ep[0], ep[1]))
        if len(pts) >= 2:
            xs, ys = zip(*pts)
            ax.plot(xs, ys, color=color, linewidth=0.7, alpha=0.8)

    ax.set_aspect("equal")
    ax.set_title("Adaptive Wavefronts", fontsize=10)


def generate_showcase():
    fig = plt.figure(figsize=(24, 16), layout="constrained")

    axs = [
        [
            fig.add_subplot(3, 4, 1),
            fig.add_subplot(3, 4, 2),
            fig.add_subplot(3, 4, 3),
            fig.add_subplot(3, 4, 4),
        ],
        [
            fig.add_subplot(3, 4, 5),
            fig.add_subplot(3, 4, 6),
            fig.add_subplot(3, 4, 7),
            fig.add_subplot(3, 4, 8),
        ],
        [
            fig.add_subplot(3, 4, 9, projection="3d"),
            fig.add_subplot(3, 4, 10, projection="3d"),
            fig.add_subplot(3, 4, 11, projection="3d"),
            fig.add_subplot(3, 4, 12, projection="3d"),
        ],
    ]

    _plot_concave_hull(axs[0][0])
    _plot_arc_fitting(axs[0][1])
    _plot_nesting(axs[0][2])
    _plot_adaptive_wavefronts(axs[0][3])
    _plot_raster_power_modulation(axs[1][0])
    _plot_smooth(axs[1][1])
    _plot_linearization(axs[1][2])
    _plot_adaptive_2d(axs[1][3])
    _plot_cylindrical(axs[2][0])
    _plot_conical_helix(axs[2][1])
    _plot_3d_offset(axs[2][2])
    _plot_fillet_polyline_3d(axs[2][3])

    return fig


__docs_target__ = ["raygeo.showcase.md"]
__images__ = [
    {
        "caption": "Composite showcase of key raygeo features",
        "function": generate_showcase,
    },
]
