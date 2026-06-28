"""Visualisation for ops/assembly/adaptive — adaptive clearing."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.collections import LineCollection
from matplotlib.colors import Normalize

from raygeo.geo.shape.polygon import (
    get_circle_polygon,
    get_polygon_area,
    get_polygons_group_difference,
    get_polygons_group_intersection,
)
from raygeo.ops.assembly.adaptive import (
    adaptive_clearing,
    target_area_per_distance,
)
from raygeo.ops.cut.cleared_area import ClearedArea


def _rect(cx, cy, w, h):
    return [
        (cx - w / 2, cy - h / 2),
        (cx + w / 2, cy - h / 2),
        (cx + w / 2, cy + h / 2),
        (cx - w / 2, cy + h / 2),
    ]


def _seed_circle(cx, cy, r, n=64):
    return [
        (
            cx + r * math.cos(2 * math.pi * i / n),
            cy + r * math.sin(2 * math.pi * i / n),
        )
        for i in range(n)
    ]


def _ops_to_segments(ops):
    """Split Ops into list of (points, is_travel) segments."""
    segs = []
    cur_pts = []
    cur_travel = False
    for i in range(ops.len()):
        if not (ops.is_cutting(i) or ops.is_travel(i)):
            continue
        is_travel = ops.is_travel(i)
        if cur_pts and is_travel != cur_travel:
            segs.append((cur_pts, cur_travel))
            cur_pts = []
        cur_travel = is_travel
        ep = ops.endpoint(i)
        cur_pts.append((ep[0], ep[1]))
    if cur_pts:
        segs.append((cur_pts, cur_travel))
    return segs


def generate_adaptive_clearing_demo():
    """Toolpath demo with seed, cuts, and travel."""
    boundary = _rect(0, 0, 200, 200)
    ca = ClearedArea(boundary=boundary, initial=[_seed_circle(0, 0, 20)])
    ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=5.0,
        advance=3.0,
        cut_z=-5.0,
        safe_z=2.0,
        step_length=1.0,
        max_deflection_deg=15.0,
        wall_margin=1.0,
        area_tolerance=50.0,
    )

    fig, ax = plt.subplots(figsize=(10, 10))

    # Pocket boundary
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, alpha=0.4, label="Pocket boundary")

    # Seed circle (entry clearing)
    seed = _seed_circle(0, 0, 20)
    sx = [p[0] for p in seed] + [seed[0][0]]
    sy = [p[1] for p in seed] + [seed[0][1]]
    ax.fill(sx, sy, color="white", alpha=1.0)
    ax.plot(
        sx,
        sy,
        color="#aaaaaa",
        linewidth=1.2,
        alpha=1.0,
        linestyle="--",
        label="Seed clearing",
    )

    # Split into cut and travel segments
    segs = _ops_to_segments(ops)

    # Collect cut segments in order for gradient colouring
    cut_segs = [
        (pts, i) for i, (pts, is_travel) in enumerate(segs) if not is_travel
    ]

    cmap = plt.cm.turbo
    norm = Normalize(vmin=0, vmax=1)

    # Build per-edge colours based on cumulative distance along the whole
    # cut path so the full spectrum is always used, even for short paths.
    segs_list = []
    cum_dists = []
    cum = 0.0
    prev = None
    for pts, _ in cut_segs:
        for p in pts:
            if prev is not None:
                segs_list.append([prev, p])
                cum += math.hypot(p[0] - prev[0], p[1] - prev[1])
                cum_dists.append(cum)
            prev = p
    total = cum if cum > 0 else 1.0

    lc = LineCollection(
        segs_list,
        colors=cmap([d / total for d in cum_dists]),
        linewidth=0.6,
        alpha=1.0,
    )
    ax.add_collection(lc)

    # Colourbar legend for cut progress
    sm = plt.cm.ScalarMappable(cmap=cmap, norm=norm)
    sm.set_array([])
    cbar = fig.colorbar(sm, ax=ax, orientation="vertical", pad=0.02, aspect=30)
    cbar.set_label("Clearing cut progress", fontsize=8)

    # Pass 2: draw all travel segments on top
    for pts, is_travel in segs:
        if is_travel:
            xs = [p[0] for p in pts]
            ys = [p[1] for p in pts]
            ax.plot(
                xs,
                ys,
                color="#888888",
                linestyle=":",
                linewidth=1.2,
                alpha=0.8,
                dashes=(1, 2),
            )

    # Mark start position of each cutting segment
    start_xs = [pts[0][0] for pts, is_travel in segs if not is_travel and pts]
    start_ys = [pts[0][1] for pts, is_travel in segs if not is_travel and pts]
    ax.scatter(
        start_xs,
        start_ys,
        marker="o",
        color="#e31a1c",
        s=12,
        zorder=5,
        label="Cut segment start",
    )

    # Single legend entry for travel (cuts shown via colourbar)
    ax.plot(
        [],
        [],
        color="#888888",
        linestyle=":",
        linewidth=1.2,
        alpha=0.8,
        dashes=(1, 2),
        label="Travel",
    )
    ax.set_aspect("equal")
    cd = ops.cut_distance()
    title = (
        f"Adaptive Clearing — constant engagement\nCut distance: {cd:.1f} mm"
    )
    if ops.len() > 0:
        title += f"  |  {len(_ops_to_points(ops))} path points"
    ax.set_title(title)
    ax.legend(loc="upper right", fontsize=8)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")

    return fig


# Re-use _ops_to_points for the title count
def _ops_to_points(ops):
    out = []
    for i in range(ops.len()):
        if ops.is_cutting(i) or ops.is_travel(i):
            ep = ops.endpoint(i)
            out.append((ep[0], ep[1], ep[2], ops.is_travel(i)))
    return out


# ── target_area_per_distance ──────────────────────────────────────


def generate_target_area_curves():
    """Target area per distance as a function of advance and step length."""
    R = 5.0

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    # ── Left: area vs advance for multiple step lengths ──
    step_lengths = [0.5, 1.0, 2.0, 4.0]
    advances = np.linspace(0, 2.0 * R, 200)
    for sl in step_lengths:
        vals = [target_area_per_distance(R, a, sl) for a in advances]
        ax1.plot(advances, vals, linewidth=2, label=f"step = {sl}")
    ax1.axvline(R, color="gray", linestyle=":", alpha=0.4)
    ax1.set_xlabel("Advance (mm)")
    ax1.set_ylabel("Target area / distance (mm)")
    ax1.set_title("target_area_per_distance vs Advance")
    ax1.legend(fontsize=8, title="Step length")
    ax1.grid(True, alpha=0.3)

    # ── Right: area vs step length for multiple advances ──
    advances2 = [1.0, 2.0, 3.0, 4.0]
    step_vals = np.linspace(0.1, R * 1.5, 200)
    for adv in advances2:
        vals = [target_area_per_distance(R, adv, s) for s in step_vals]
        ax2.plot(step_vals, vals, linewidth=2, label=f"adv = {adv}")
    ax2.set_xlabel("Step length (mm)")
    ax2.set_ylabel("Target area / distance (mm)")
    ax2.set_title("target_area_per_distance vs Step Length")
    ax2.legend(fontsize=8, title="Advance")
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_target_area_geometry():
    """Geometry of the wall-crescent model used by target_area_per_distance."""
    R = 5.0
    advance = 2.0
    step_length = 1.0

    wall_x = R - advance
    c1 = (0.0, 0.0)
    c2 = (0.0, step_length)

    # Disks.
    disk1 = get_circle_polygon(c1, R, 64)
    disk2 = get_circle_polygon(c2, R, 64)
    crescent = get_polygons_group_difference([disk2], [disk1])
    wall_poly = [
        [
            (wall_x, -R * 1.5),
            (wall_x + R * 2, -R * 1.5),
            (wall_x + R * 2, R * 1.5),
            (wall_x, R * 1.5),
        ]
    ]

    # Fresh material = crescent ∩ region_right_of_wall.
    fresh = get_polygons_group_intersection(crescent, wall_poly)

    fig, ax = plt.subplots(figsize=(7, 7))
    theta = np.linspace(0, 2 * math.pi, 100)

    # Wall region (already cleared — left of wall).
    wall_region = [
        [
            (wall_x - R * 2, -R * 1.5),
            (wall_x, -R * 1.5),
            (wall_x, R * 1.5),
            (wall_x - R * 2, R * 1.5),
        ]
    ]
    for poly in wall_region:
        arr = np.array(poly + [poly[0]])
        ax.fill(
            arr[:, 0],
            arr[:, 1],
            "lightgray",
            alpha=0.4,
            label="Previous pass (wall region)",
        )

    # Wall line.
    ax.axvline(
        wall_x,
        color="red",
        linewidth=2,
        linestyle="-",
        label=f"Wall at x = {wall_x:.1f}",
    )

    # Full crescent faintly.
    for poly in crescent:
        arr = np.array(poly)
        ax.fill(arr[:, 0], arr[:, 1], "tomato", alpha=0.2)

    # Fresh material (crescent beyond the wall).
    for poly in fresh:
        arr = np.array(poly)
        ax.fill(
            arr[:, 0],
            arr[:, 1],
            "tomato",
            alpha=0.7,
            label="Fresh material (crescent ∩ right-of-wall)",
        )

    # Disks.
    for centre, style, lbl in [
        (c1, "b--", "Disk(c1) — prev pos"),
        (c2, "b-", "Disk(c2) — current pos"),
    ]:
        ax.plot(
            centre[0] + R * np.cos(theta),
            centre[1] + R * np.sin(theta),
            style,
            linewidth=1.5,
            label=lbl,
        )
    for pt, mk, lbl in [(c1, "bs", "c1"), (c2, "bo", "c2")]:
        ax.plot(*pt, mk, markersize=5)

    val = target_area_per_distance(R, advance, step_length)
    apd = val * step_length
    ax.set_title(
        f"R={R}, advance={advance}, step={step_length}\n"
        f"target_area_per_distance = {val:.3f} mm\n"
        f"(crescent area beyond wall = {apd:.2f} mm²)"
    )
    ax.set_aspect("equal")
    ax.set_xlim(-R * 1.3, R * 1.3)
    ax.set_ylim(-R * 0.5, R * 1.5)
    ax.legend(fontsize=7, loc="upper left")
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.assembly.adaptive.md"]


def _plot_2d_toolpath(ops, ax):
    """Plot a 2D top-down toolpath.

    Travel moves (``move_to`` / G0) → dashed dimgray lines.
    Cutting moves (``line_to`` / G1) → ``LineCollection`` coloured by
    cumulative arc-length through the turbo gradient (full opacity).
    """
    pts = _ops_to_points(ops)
    if not pts:
        return

    segments = []
    cur = []
    for p in pts:
        x, y, z, is_travel = p
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            cur = []
        else:
            cur.append((x, y))
    if len(cur) > 1:
        segments.append(cur)

    segs_list = []
    cum_dists = []
    cum = 0.0
    prev = None
    for seg in segments:
        for p in seg:
            if prev is not None:
                segs_list.append([prev, p])
                cum += math.hypot(p[0] - prev[0], p[1] - prev[1])
                cum_dists.append(cum)
            prev = p
    total = cum if cum > 0 else 1.0
    if segs_list:
        ax.add_collection(
            LineCollection(
                segs_list,
                colors=plt.cm.turbo([d / total for d in cum_dists]),
                linewidth=0.8,
                alpha=1.0,
            )
        )

    prev = None
    for p in pts:
        x, y, z, is_travel = p
        if is_travel:
            if prev is not None:
                ax.plot(
                    [prev[0], x],
                    [prev[1], y],
                    linestyle="--",
                    linewidth=1.0,
                    color="dimgray",
                    alpha=0.8,
                )
            prev = (x, y)
        else:
            prev = (x, y)


def _draw_3d_boundary(ax, boundary, islands, z_plane):
    """Draw boundary and islands on the 3D z-plane."""
    if boundary is not None and z_plane is not None:
        bnd = np.array(list(boundary) + [boundary[0]])
        ax.plot(
            bnd[:, 0],
            bnd[:, 1],
            zs=z_plane,
            zdir="z",
            color="k",
            linewidth=2,
            alpha=0.5,
        )
    if islands and z_plane is not None:
        for isl in islands:
            isl_arr = np.array(list(isl) + [isl[0]])
            ax.plot(
                isl_arr[:, 0],
                isl_arr[:, 1],
                zs=z_plane,
                zdir="z",
                color="gray",
                linewidth=1.5,
                alpha=0.4,
            )


def _plot_3d_toolpath(
    ops,
    ax,
    title,
    boundary=None,
    islands=None,
    z_plane=None,
):
    """Plot 3D toolpath: travel=dashed dimgray, cutting=rainbow by path."""
    pts_list = _ops_to_points(ops)
    if not pts_list:
        fig = ax.figure
        fig.tight_layout()
        return fig

    segments = []
    cur = []
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel:
            if len(cur) > 1:
                segments.append(cur)
            cur = []
        else:
            cur.append((x, y, z))
    if len(cur) > 1:
        segments.append(cur)

    segs_3d = []
    cum_dists = []
    cum = 0.0
    prev = None
    for seg in segments:
        for p in seg:
            if prev is not None:
                segs_3d.append([prev, p])
                d = math.sqrt(
                    (p[0] - prev[0]) ** 2
                    + (p[1] - prev[1]) ** 2
                    + (p[2] - prev[2]) ** 2
                )
                cum += d
                cum_dists.append(cum)
            prev = p
    total = cum if cum > 0 else 1.0
    if segs_3d:
        from mpl_toolkits.mplot3d.art3d import Line3DCollection

        lc3d = Line3DCollection(
            segs_3d,
            colors=plt.cm.turbo([d / total for d in cum_dists]),
            linewidth=0.8,
            alpha=1.0,
        )
        ax.add_collection3d(lc3d)

    prev = None
    for p in pts_list:
        x, y, z, is_travel = p
        if is_travel:
            if prev is not None:
                ax.plot(
                    [prev[0], x],
                    [prev[1], y],
                    [prev[2], z],
                    linestyle="--",
                    linewidth=1.0,
                    color="dimgray",
                    alpha=0.8,
                )
            prev = (x, y, z)
        else:
            prev = (x, y, z)

    _draw_3d_boundary(ax, boundary, islands, z_plane)
    ax.set_title(title)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_zlabel("Z")
    ax.view_init(elev=30, azim=-45)

    xl, xr = ax.get_xlim()
    yl, yr = ax.get_ylim()
    zl, zr = ax.get_zlim()
    half = max(xr - xl, yr - yl, zr - zl) * 0.5
    xm = (xl + xr) * 0.5
    ym = (yl + yr) * 0.5
    zm = (zl + zr) * 0.5
    ax.set_xlim(xm - half, xm + half)
    ax.set_ylim(ym - half, ym + half)
    ax.set_zlim(zm - half, zm + half)


# ── Centre-island pocket (circle seed + clearing) ────────────────────


def generate_adaptive_clearing_centre_island():
    """60×60 pocket with a 10×10 island on centre — circle seed + clearing."""
    target_z = -5.0
    boundary = _rect(0, 0, 60, 60)
    islands = [_rect(5, 0, 10, 10)]

    # Hardcoded seed circle (largest inscribed circle minus tool + margin)
    cx, cy, r = -13.7, 13.7, 12.2
    print(
        f"  Seed circle: centre=({cx:.1f}, {cy:.1f})"
        f"  radius={r:.1f}  diameter={2 * r:.1f}"
    )
    cleared_polys = [get_circle_polygon((cx, cy), r, 64)]

    ca = ClearedArea(boundary=boundary, islands=islands, initial=cleared_polys)
    clear_ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        radius=3.0,
        advance=1.5,
        cut_z=target_z,
        safe_z=2.0,
        area_tolerance=1.0,
    )
    remaining = sum(get_polygon_area(p) for p in ca.remaining())
    combined_ops = clear_ops

    fig = plt.figure(figsize=(14, 6))
    ax3d = fig.add_subplot(1, 2, 1, projection="3d")
    _plot_3d_toolpath(
        combined_ops,
        ax3d,
        "Entry + Clearing Toolpath (3D)",
        boundary=boundary,
        islands=islands,
        z_plane=target_z,
    )
    ax = fig.add_subplot(1, 2, 2)
    ax.set_aspect("equal")
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Pocket boundary")
    for isl in islands:
        ix = [p[0] for p in isl] + [isl[0][0]]
        iy = [p[1] for p in isl] + [isl[0][1]]
        ax.fill(ix, iy, color="gray", alpha=0.4)
        ax.plot(ix, iy, color="dimgray", linewidth=1.2, label="Island")
    seed_area = 0.0
    for poly in cleared_polys:
        if len(poly) < 3:
            continue
        px = [p[0] for p in poly] + [poly[0][0]]
        py = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(px, py, color="steelblue", alpha=0.3)
        ax.plot(
            px,
            py,
            color="steelblue",
            linewidth=1.2,
            linestyle="--",
            label="Seed clearing",
        )
        seed_area += abs(get_polygon_area([(p[0], p[1]) for p in poly]))
    _plot_2d_toolpath(combined_ops, ax)
    for poly in ca.remaining():
        if len(poly) < 3:
            continue
        a = get_polygon_area([(p[0], p[1]) for p in poly])
        if abs(a) < 0.3:
            continue
        rx = [p[0] for p in poly] + [poly[0][0]]
        ry = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(rx, ry, color="crimson", alpha=0.15)
        ax.plot(
            rx,
            ry,
            color="crimson",
            linewidth=0.6,
            alpha=0.5,
            label="Remaining",
        )
    ax.set_title(
        f"Seed = {seed_area:.0f} mm²  |  remaining = {remaining:.0f} mm²\n"
        f"(circle seed — no entry strategy)",
        fontsize=10,
    )
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


# ── Narrow pocket 3D (circle seed + clearing) ────────────────────────


def _narrow_shared():
    """Run circle-seed + clearing for the 80×14 narrow pocket.

    Returns ``(combined_ops, ca, boundary, target_z, cleared_polys)``.
    """
    target_z = -5.0
    tool_radius = 3.0
    boundary = _rect(0, 0, 80, 14)

    # Hardcoded seed circle
    cx, cy, r = -11.1, 0.0, 3.0
    print(
        f"  Seed circle: centre=({cx:.1f}, {cy:.1f})"
        f"  radius={r:.1f}  diameter={2 * r:.1f}"
    )
    cleared_polys = [get_circle_polygon((cx, cy), r, 64)]

    ca = ClearedArea(boundary=boundary, initial=cleared_polys)
    clear_ops = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        radius=tool_radius,
        advance=1.5,
        cut_z=target_z,
        safe_z=2.0,
        area_tolerance=1.0,
    )
    combined_ops = clear_ops
    return combined_ops, ca, boundary, target_z, cleared_polys


def generate_adaptive_clearing_narrow_3d():
    """Narrow pocket — 3D toolpath view (circle seed + clearing)."""
    combined_ops, ca, boundary, target_z, _ = _narrow_shared()
    fig = plt.figure(figsize=(7, 5))
    ax = fig.add_subplot(111, projection="3d")
    _plot_3d_toolpath(
        combined_ops,
        ax,
        "Narrow Pocket — Circle Seed + Clearing (3D)",
        boundary=boundary,
        z_plane=target_z,
    )
    fig.tight_layout()
    _ = ca
    return fig


def generate_adaptive_clearing_narrow_2d():
    """Narrow pocket — 2D top-down with seed and remaining overlay."""
    combined_ops, ca, boundary, target_z, cleared_polys = _narrow_shared()
    remaining = sum(get_polygon_area(p) for p in ca.remaining())
    fig = plt.figure(figsize=(7, 5))
    ax = fig.add_subplot(111)
    ax.set_aspect("equal")
    bx = [p[0] for p in boundary] + [boundary[0][0]]
    by = [p[1] for p in boundary] + [boundary[0][1]]
    ax.plot(bx, by, "k-", linewidth=1.5, label="Pocket boundary")
    seed_area = 0.0
    for poly in cleared_polys:
        if len(poly) < 3:
            continue
        px = [p[0] for p in poly] + [poly[0][0]]
        py = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(px, py, color="steelblue", alpha=0.2)
        seed_area += abs(get_polygon_area([(p[0], p[1]) for p in poly]))
    _plot_2d_toolpath(combined_ops, ax)
    for poly in ca.remaining():
        if len(poly) < 3:
            continue
        a = get_polygon_area([(p[0], p[1]) for p in poly])
        if abs(a) < 0.3:
            continue
        rx = [p[0] for p in poly] + [poly[0][0]]
        ry = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(rx, ry, color="crimson", alpha=0.15)
        ax.plot(
            rx,
            ry,
            color="crimson",
            linewidth=0.6,
            alpha=0.5,
            label="Remaining",
        )
    ax.set_title(
        f"Seed = {seed_area:.0f} mm²  |  remaining = {remaining:.0f} mm²",
        fontsize=10,
    )
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    handles, labels = ax.get_legend_handles_labels()
    seen = set()
    unique = []
    for h, lbl in zip(handles, labels):
        if lbl not in seen:
            unique.append((h, lbl))
            seen.add(lbl)
    ax.legend(*zip(*unique), loc="upper right", fontsize=8)
    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Forward-stepping constant-engagement clearing cuts "
            "(coloured by progress via the full-spectrum turbo gradient) "
            "from a central seed clearing (green), with MAT-routed "
            "travel links (red dashed) between segments."
        ),
        "function": generate_adaptive_clearing_demo,
    },
    {
        "heading": "target_area_per_distance",
        "caption": (
            "Left: target area per distance as a function of advance for"
            " several step lengths. Right: target area per distance as a"
            " function of step length for several advance values."
        ),
        "function": generate_target_area_curves,
    },
    {
        "heading": "target_area_per_distance",
        "caption": (
            "Geometric model underlying ``target_area_per_distance``:"
            " two disks offset by ``step_length`` along the travel"
            " direction, with a vertical wall at ``x = R − advance``"
            " representing the previous pass boundary. The fresh"
            " material (dark red) is the portion of the crescent that"
            " lies to the right of the wall."
        ),
        "function": generate_target_area_geometry,
    },
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Circle-seed clearing in a 60×60 pocket with a 10×10 island at"
            " the centre — 2D top-down shows seed clearing (blue), toolpath"
            " gradient, and remaining bands (red)."
        ),
        "function": generate_adaptive_clearing_centre_island,
    },
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Narrow pocket (80×14) 3D view of circle-seed adaptive clearing."
        ),
        "function": generate_adaptive_clearing_narrow_3d,
    },
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Narrow pocket (80×14) 2D top-down view showing seed clearing,"
            " toolpath gradient, and remaining uncut bands."
        ),
        "function": generate_adaptive_clearing_narrow_2d,
    },
]
