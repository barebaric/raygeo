"""Visualise the sweep-line disk-increment area (cut_area)."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.shape.polygon import (
    get_circle_polygon,
    get_polygons_group_difference,
    get_polygons_group_intersection,
)
from raygeo.ops.part.crescent import cut_area


def generate_disk_increment():
    """Disk increment when stepping, with and without fragments."""
    c1 = (4.0, 5.0)
    c2 = (8.0, 5.0)
    r = 3.0

    # Disk polygons for visualisation.
    disk1 = get_circle_polygon(c1, r, 48)
    disk2 = get_circle_polygon(c2, r, 48)
    crescent = get_polygons_group_difference([disk2], [disk1])

    # Compute increment areas.
    area_no_frags, _ = cut_area(c1, c2, r, [], [])
    cleared_square = [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]]
    area_with_frags, _ = cut_area(c1, c2, r, cleared_square, [])

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    for ax, title, area, fragments in [
        (ax1, "No cleared fragments", area_no_frags, None),
        (
            ax2,
            "With cleared square",
            area_with_frags,
            [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        ),
    ]:
        # Draw crescent.
        if crescent:
            for poly in crescent:
                arr = np.array(poly)
                ax.fill(
                    arr[:, 0], arr[:, 1], "tomato", alpha=0.5, label="Crescent"
                )

        # Draw disks.
        for centre, style, label in [
            (c1, "b--", "Disk(c1)"),
            (c2, "b-", "Disk(c2)"),
        ]:
            theta = np.linspace(0, 2 * math.pi, 100)
            cx, cy = centre
            ax.plot(
                cx + r * np.cos(theta),
                cy + r * np.sin(theta),
                style,
                linewidth=1.5,
                label=label,
            )

        # Draw cleared fragments.
        if fragments:
            f_arr = np.array(fragments + [fragments[0]])
            ax.fill(
                f_arr[:, 0],
                f_arr[:, 1],
                "lightgray",
                alpha=0.6,
                label="Cleared",
            )
            ax.plot(f_arr[:, 0], f_arr[:, 1], "gray", linewidth=1)

        # Marker for centres.
        for centre, style in [(c1, "bs"), (c2, "bo")]:
            ax.plot(*centre, style, markersize=4)

        ax.set_aspect("equal")
        ax.set_xlim(0, 12)
        ax.set_ylim(0, 10)
        ax.set_title(f"{title}\ncut_area = {area:.2f} mm²")
        ax.legend(fontsize=7, loc="upper right")
        ax.grid(True, alpha=0.3)

    fig.suptitle("cut_area — Disk Increment Area", fontsize=13)
    fig.tight_layout()
    return fig


# ── Analytical helpers ────────────────────────────────────────────


def _lens_area(d: float, R: float) -> float:
    """Lens area of two equal circles radius R, centres distance d."""
    if d >= 2.0 * R:
        return 0.0
    if d <= 0.0:
        return math.pi * R * R
    return 2.0 * R * R * math.acos(d / (2.0 * R)) - (d / 2.0) * math.sqrt(
        4.0 * R * R - d * d
    )


def _crescent_area(d: float, R: float) -> float:
    """Crescent: area(disk2) minus lens(disk1 ∩ disk2)."""
    return math.pi * R * R - _lens_area(d, R)


# ── 1. Area vs step distance ─────────────────────────────────────


def generate_crescent_area_vs_distance():
    """Crescent area vs step distance compared to analytical formula."""
    R = 5.0
    c1 = (0.0, 0.0)
    ds = np.linspace(0.001, 2.0 * R, 150)
    totals = []
    lefts = []
    analytical = []
    for d in ds:
        total, left = cut_area(c1, (d, 0.0), R, [], [])
        totals.append(total)
        lefts.append(left)
        analytical.append(_crescent_area(d, R))

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    ax1.plot(ds, totals, "b-", linewidth=2, label="cut_area total")
    ax1.plot(
        ds,
        analytical,
        "r--",
        linewidth=2,
        label="Analytical crescent",
    )
    ax1.set_xlabel("Step distance d (mm)")
    ax1.set_ylabel("Area (mm²)")
    ax1.set_title("Total Crescent Area vs Step Distance")
    ax1.legend(fontsize=8)
    ax1.grid(True, alpha=0.3)

    # Right panel: left-area fraction.
    ax2.plot(ds, lefts, "g-", linewidth=2, label="Left area")
    ax2.plot(
        ds,
        [t / 2 for t in totals],
        "k--",
        linewidth=1,
        alpha=0.5,
        label="total / 2",
    )
    ax2.set_xlabel("Step distance d (mm)")
    ax2.set_ylabel("Area (mm²)")
    ax2.set_title("Left-Side Portion of Crescent")
    ax2.legend(fontsize=8)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


# ── 2. 2D heatmap ────────────────────────────────────────────────


def generate_crescent_heatmap_2d():
    """2D heatmap of cut_area as c2 moves in a grid around c1."""
    R = 5.0
    c1 = (0.0, 0.0)
    n = 45
    half = 2.0 * R
    xs = np.linspace(-half, half, n)
    ys = np.linspace(-half, half, n)
    field = np.empty((n, n))
    for i, x in enumerate(xs):
        for j, y in enumerate(ys):
            total, _ = cut_area(c1, (x, y), R, [], [])
            field[j, i] = total

    fig, ax = plt.subplots(figsize=(7.5, 7))
    im = ax.pcolormesh(xs, ys, field, shading="auto", cmap="viridis")
    cbar = fig.colorbar(im, ax=ax, shrink=0.85)
    cbar.set_label("cut_area (mm²)")

    # Overlay c1 disk.
    theta = np.linspace(0, 2 * math.pi, 100)
    ax.plot(
        c1[0] + R * np.cos(theta),
        c1[1] + R * np.sin(theta),
        "w--",
        linewidth=1.5,
        label="Disk(c1)",
    )
    ax.plot(*c1, "wo", markersize=5)

    ax.set_xlabel("c2.x (mm)")
    ax.set_ylabel("c2.y (mm)")
    ax.set_aspect("equal")
    ax.set_title("cut_area for c2 Positions Around c1")
    ax.legend(fontsize=8)
    fig.tight_layout()
    return fig


# ── 3. Fragment sweep ────────────────────────────────────────────


def generate_crescent_fragment_sweep():
    """Sweep a vertical-wall fragment across the crescent."""
    R = 5.0
    c1 = (0.0, 0.0)
    c2 = (4.0, 0.0)

    # Sweep a vertical wall at x_wall — fragment is region to the RIGHT.
    wall_positions = np.linspace(-R, c2[0] + R, 100)
    areas = []
    lefts = []
    for wx in wall_positions:
        frag = [[(wx, -20.0), (20.0, -20.0), (20.0, 20.0), (wx, 20.0)]]
        total, left = cut_area(c1, c2, R, frag, [])
        areas.append(total)
        lefts.append(left)

    # Pick a wall position mid-sweep for the geometry panel.
    mid_wx = wall_positions[len(wall_positions) // 2]
    mid_frag = [[(mid_wx, -20.0), (20.0, -20.0), (20.0, 20.0), (mid_wx, 20.0)]]
    mid_total, _ = cut_area(c1, c2, R, mid_frag, [])

    # Crescent polygon (no fragments) for geometry.
    disk1 = get_circle_polygon(c1, R, 48)
    disk2 = get_circle_polygon(c2, R, 48)
    free_crescent = get_polygons_group_difference([disk2], [disk1])

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5.5))
    theta = np.linspace(0, 2 * math.pi, 100)

    # ── Left: geometry at mid-point ──
    for poly in free_crescent:
        arr = np.array(poly)
        ax1.fill(arr[:, 0], arr[:, 1], "tomato", alpha=0.25)

    # Fragment (wall to the right of mid_wx).
    f_arr = np.array(
        [
            (mid_wx, -20.0),
            (20.0, -20.0),
            (20.0, 20.0),
            (mid_wx, 20.0),
            (mid_wx, -20.0),
        ]
    )
    ax1.fill(
        f_arr[:, 0], f_arr[:, 1], "lightgray", alpha=0.6, label="Fragment"
    )
    ax1.axvline(mid_wx, color="gray", linewidth=2, linestyle="-")

    for centre, style, label in [
        (c1, "b--", "Disk(c1)"),
        (c2, "b-", "Disk(c2)"),
    ]:
        ax1.plot(
            centre[0] + R * np.cos(theta),
            centre[1] + R * np.sin(theta),
            style,
            linewidth=1.5,
            label=label,
        )
    for pt, style in [(c1, "bs"), (c2, "bo")]:
        ax1.plot(*pt, style, markersize=4)

    no_frag_total, _ = cut_area(c1, c2, R, [], [])
    ax1.set_title(
        f"Wall at x = {mid_wx:.1f}\n"
        f"cut_area = {mid_total:.2f}  (no frag = {no_frag_total:.2f}) mm²"
    )
    ax1.set_aspect("equal")
    ax1.set_xlim(-R * 1.3, c2[0] + R * 0.7)
    ax1.set_ylim(-R * 1.2, R * 1.2)
    ax1.legend(fontsize=7)
    ax1.grid(True, alpha=0.3)

    # ── Right: area vs wall position ──
    ax2.plot(wall_positions, areas, "b-", linewidth=2, label="Total area")
    ax2.plot(
        wall_positions,
        lefts,
        "g--",
        linewidth=2,
        label="Left area",
    )
    ax2.axvline(
        mid_wx,
        color="gray",
        linestyle=":",
        alpha=0.5,
        label=f"Wall at {mid_wx:.1f}",
    )
    ax2.axhline(0, color="gray", linestyle=":", alpha=0.3)
    ax2.axhline(
        no_frag_total,
        color="gray",
        linestyle="--",
        alpha=0.3,
        label=f"No frag = {no_frag_total:.1f}",
    )
    ax2.set_xlabel("Wall x position (mm)")
    ax2.set_ylabel("Area (mm²)")
    ax2.set_title("Crescent Area vs Fragment Wall Position")
    ax2.legend(fontsize=7)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


# ── 4. valid_area clipping ───────────────────────────────────────


def generate_crescent_valid_area_clip():
    """Crescent clipped to different valid-area shapes."""
    R = 5.0
    c1 = (0.0, 0.0)
    c2 = (4.0, 0.0)

    disk1 = get_circle_polygon(c1, R, 48)
    disk2 = get_circle_polygon(c2, R, 48)
    free_crescent = get_polygons_group_difference([disk2], [disk1])

    scenarios = [
        {
            "label": "No valid_area (full)",
            "valid": [],
            "col": "tomato",
        },
        {
            "label": "Left-half clip",
            "valid": [[(-6.0, -7.0), (1.0, -7.0), (1.0, 7.0), (-6.0, 7.0)]],
            "col": "dodgerblue",
        },
        {
            "label": "Window around tip",
            "valid": [[(5.0, -7.0), (9.0, -7.0), (9.0, 7.0), (5.0, 7.0)]],
            "col": "seagreen",
        },
        {
            "label": "Base (excludes tip)",
            "valid": [
                [(-10.0, -10.0), (5.0, -10.0), (5.0, 10.0), (-10.0, 10.0)]
            ],
            "col": "darkorange",
        },
    ]

    fig, axes = plt.subplots(1, 4, figsize=(20, 5))
    theta = np.linspace(0, 2 * math.pi, 100)

    for ax, sc in zip(axes, scenarios):
        total, left = cut_area(c1, c2, R, [], sc["valid"])
        valid_polys = sc["valid"]

        # Draw the full crescent faintly in background.
        for poly in free_crescent:
            arr = np.array(poly)
            ax.fill(arr[:, 0], arr[:, 1], "lightgray", alpha=0.25)

        # Draw the clipped crescent over top.
        if valid_polys:
            clipped = get_polygons_group_intersection(
                free_crescent,
                valid_polys,
            )
        else:
            clipped = free_crescent
        for poly in clipped:
            arr = np.array(poly)
            ax.fill(
                arr[:, 0],
                arr[:, 1],
                sc["col"],
                alpha=0.5,
                label=f"Area = {total:.2f}",
            )

        # Draw valid area boundary.
        for poly in valid_polys:
            v_arr = np.array(poly + [poly[0]])
            ax.plot(
                v_arr[:, 0],
                v_arr[:, 1],
                color=sc["col"],
                linewidth=1.5,
                linestyle="--",
                label="Valid area",
            )

        # Draw disks.
        for centre, style in [(c1, "b--"), (c2, "b-")]:
            ax.plot(
                centre[0] + R * np.cos(theta),
                centre[1] + R * np.sin(theta),
                style,
                linewidth=1,
            )
        for pt, style in [(c1, "bs"), (c2, "bo")]:
            ax.plot(*pt, style, markersize=3)

        ax.set_aspect("equal")
        ax.set_xlim(-R * 1.2, c2[0] + R * 1.2)
        ax.set_ylim(-R * 1.2, R * 1.2)
        ax.set_title(f"{sc['label']}\ntotal={total:.2f}, left={left:.2f}")
        ax.legend(fontsize=7)
        ax.grid(True, alpha=0.3)

    fig.suptitle("cut_area with Valid-Area Clipping", fontsize=13)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.part.crescent.md"]
__images__ = [
    {
        "heading": "cut_area",
        "caption": (
            "Disk increment (red): stepping C1 to C2; left full, right"
            " shows reduction with a cleared fragment"
        ),
        "function": generate_disk_increment,
    },
    {
        "heading": "cut_area",
        "caption": (
            "Crescent area vs step ``d`` compared to analytical formula;"
            " right shows left-side portion"
        ),
        "function": generate_crescent_area_vs_distance,
    },
    {
        "heading": "cut_area",
        "caption": (
            "2D heatmap of ``cut_area`` as c2 orbits c1; zero at"
            " coincidence, maximal at mid distances"
        ),
        "function": generate_crescent_heatmap_2d,
    },
    {
        "heading": "cut_area",
        "caption": (
            "Vertical-wall fragment sweeping crescent; left shows geometry,"
            " right plots area vs wall position"
        ),
        "function": generate_crescent_fragment_sweep,
    },
    {
        "heading": "cut_area",
        "caption": (
            "Crescent clipped to ``valid_area``: no clip, left-half"
            " window, tip window; faint gray = unclipped"
        ),
        "function": generate_crescent_valid_area_clip,
    },
]
