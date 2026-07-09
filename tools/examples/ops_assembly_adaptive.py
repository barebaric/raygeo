"""Visualisation for ops/assembly/adaptive — adaptive clearing."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.shape.polygon import (
    get_circle_polygon,
    get_polygon_area,
    get_polygon_signed_area,
    get_polygons_group_difference,
    get_polygons_group_intersection,
)
from raygeo.ops.assembly.adaptive import (
    adaptive_clearing,
    target_area_per_distance,
)
from raygeo.ops.cut.cleared_area import ClearedArea
from tools.plot import plot_ops_2d, plot_ops_3d


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


def generate_adaptive_clearing_demo():
    """Toolpath demo with seed, cuts, and travel."""
    boundary = _rect(0, 0, 200, 200)
    ca = ClearedArea(boundary=boundary, initial=[_seed_circle(0, 0, 20)])
    result = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=5.0,
        step_over=3.0,
        target_z=-5.0,
        safe_z=2.0,
        max_deflection_deg=15.0,
        wall_margin=1.0,
        area_tolerance=50.0,
    )

    fig, ax = plt.subplots(figsize=(10, 10))

    # Pocket boundary (semi-transparent)
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

    plot_ops_2d(ax, result.ops, mark_cut_start=True)

    cd = result.ops.cut_distance()
    title = (
        f"Adaptive Clearing — constant engagement\nCut distance: {cd:.1f} mm"
    )
    if result.ops.len() > 0:
        title += f"  |  {len(_ops_to_points(result.ops))} path points"
    ax.set_title(title)

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


# ── Centre-island pocket (circle seed + clearing) ────────────────────


def generate_adaptive_clearing_centre_island():
    """60×60 pocket with a 10×10 island on centre — circle seed + clearing."""
    target_z = -5.0
    boundary = _rect(0, 0, 60, 60)
    islands = [_rect(5, 0, 10, 10)]

    # Hardcoded seed circle (largest inscribed circle minus tool + margin)
    cx, cy, r = -13.7, 13.7, 12.2
    cleared_polys = [get_circle_polygon((cx, cy), r, 64)]

    ca = ClearedArea(boundary=boundary, islands=islands, initial=cleared_polys)
    result = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        islands=islands,
        tool_radius=3.0,
        step_over=1.5,
        target_z=target_z,
        safe_z=2.0,
        area_tolerance=1.0,
    )
    remaining = sum(get_polygon_area(p) for p in ca.remaining())
    combined_ops = result.ops

    fig = plt.figure(figsize=(14, 6))
    ax3d = fig.add_subplot(1, 2, 1, projection="3d")
    plot_ops_3d(ax3d, combined_ops, boundary=boundary, islands=islands)
    ax = fig.add_subplot(1, 2, 2)
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
        )
        seed_area += abs(get_polygon_area([(p[0], p[1]) for p in poly]))
    for poly in ca.remaining():
        if len(poly) < 3:
            continue
        a_s = get_polygon_signed_area([(p[0], p[1]) for p in poly])
        if abs(a_s) < 0.3:
            continue
        rx = [p[0] for p in poly] + [poly[0][0]]
        ry = [p[1] for p in poly] + [poly[0][1]]
        if a_s > 0:
            ax.fill(rx, ry, color="crimson", alpha=0.15)
            ax.plot(
                rx,
                ry,
                color="crimson",
                linewidth=0.6,
                alpha=0.5,
            )
        else:
            ax.fill(rx, ry, color="white")
    plot_ops_2d(ax, combined_ops, boundary=boundary, islands=islands)
    ax.set_title(
        f"Seed = {seed_area:.0f} mm²  |  remaining = {remaining:.0f} mm²\n"
        f"(circle seed — no entry strategy)",
        fontsize=10,
    )
    fig.tight_layout()
    return fig


# ── Narrow pocket 3D (circle seed + clearing) ────────────────────────


def _narrow_shared():
    """Run circle-seed + clearing for the 80×14 narrow pocket.

    Returns
    ``(combined_ops, ca, boundary, target_z, cleared_polys, tool_radius)``.
    """
    target_z = -5.0
    tool_radius = 3.0
    boundary = _rect(0, 0, 80, 14)

    # Hardcoded seed circle
    cx, cy, r = -11.1, 0.0, 3.0
    cleared_polys = [get_circle_polygon((cx, cy), r, 64)]

    ca = ClearedArea(boundary=boundary, initial=cleared_polys)
    result = adaptive_clearing(
        cleared=ca,
        pocket_boundary=boundary,
        tool_radius=tool_radius,
        step_over=1.5,
        target_z=target_z,
        safe_z=2.0,
        area_tolerance=1.0,
    )
    combined_ops = result.ops
    return combined_ops, ca, boundary, target_z, cleared_polys, tool_radius


def generate_adaptive_clearing_narrow():
    """Narrow pocket (80×14) — 3D + 2D combined view."""
    (combined_ops, ca, boundary, target_z, cleared_polys, tool_radius) = (
        _narrow_shared()
    )
    remaining = sum(get_polygon_area(p) for p in ca.remaining())
    fig = plt.figure(figsize=(14, 6))
    ax3d = fig.add_subplot(1, 2, 1, projection="3d")
    plot_ops_3d(ax3d, combined_ops, boundary=boundary)
    ax = fig.add_subplot(1, 2, 2)
    envelope = ca.envelope(tool_radius)
    for poly in envelope:
        if len(poly) < 3:
            continue
        ex = [p[0] for p in poly] + [poly[0][0]]
        ey = [p[1] for p in poly] + [poly[0][1]]
        ax.plot(ex, ey, "--", color="gray", linewidth=1.0)
    seed_area = 0.0
    for poly in cleared_polys:
        if len(poly) < 3:
            continue
        px = [p[0] for p in poly] + [poly[0][0]]
        py = [p[1] for p in poly] + [poly[0][1]]
        ax.fill(px, py, color="steelblue", alpha=0.2)
        seed_area += abs(get_polygon_area([(p[0], p[1]) for p in poly]))
    for poly in ca.remaining():
        if len(poly) < 3:
            continue
        a_s = get_polygon_signed_area([(p[0], p[1]) for p in poly])
        if abs(a_s) < 0.3:
            continue
        rx = [p[0] for p in poly] + [poly[0][0]]
        ry = [p[1] for p in poly] + [poly[0][1]]
        if a_s > 0:
            ax.fill(rx, ry, color="crimson", alpha=0.15)
            ax.plot(
                rx,
                ry,
                color="crimson",
                linewidth=0.6,
                alpha=0.5,
            )
        else:
            ax.fill(rx, ry, color="white")
    plot_ops_2d(ax, combined_ops, boundary=boundary)
    ax.set_title(
        f"Seed = {seed_area:.0f} mm²  |  remaining = {remaining:.0f} mm²",
        fontsize=10,
    )
    fig.tight_layout()
    return fig


__images__ = [
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Constant-engagement clearing cuts, MAT-routed travel links,"
            " coloured by progress."
        ),
        "function": generate_adaptive_clearing_demo,
    },
    {
        "heading": "target_area_per_distance",
        "caption": (
            "Left: area/distance vs advance for several step lengths."
            " Right: vs step length for several advances."
        ),
        "function": generate_target_area_curves,
    },
    {
        "heading": "target_area_per_distance",
        "caption": (
            "Two offset disks and a wall at x=R−advance: crescent"
            " beyond wall is fresh material."
        ),
        "function": generate_target_area_geometry,
    },
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Circle-seed clearing in a square pocket with central island:"
            " seed, toolpath, and remaining."
        ),
        "function": generate_adaptive_clearing_centre_island,
    },
    {
        "heading": "adaptive_clearing",
        "caption": (
            "Narrow pocket — 3D toolpath view (left) and 2D"
            " top-down with seed/remaining overlay (right)."
        ),
        "function": generate_adaptive_clearing_narrow,
    },
]
