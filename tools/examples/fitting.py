"""Generate fitting example images."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle

from raygeo.geo import Geometry
from raygeo.geo.algo.fitting import (
    fit_circle_to_3_points,
    fit_circle_to_points,
    flatten_to_points,
    get_polyline_arc_deviation,
    get_polyline_line_deviation,
    linearize_geometry,
    project_circle_center_to_bisector,
)


def generate_examples(output_dir):
    images = []

    rng = np.random.default_rng(42)
    angles = np.linspace(0, 2 * math.pi, 50)
    cx, cy, cr = 50, 50, 30
    pts = [
        (
            cx + cr * math.cos(a) + rng.normal(0, 0.5),
            cy + cr * math.sin(a) + rng.normal(0, 0.5),
        )
        for a in angles
    ]

    result = fit_circle_to_points([(x, y, 0.0) for x, y in pts])
    fc, fr, ferr = result if result else ((0.0, 0.0), 0.0, 0.0)

    fig, ax = plt.subplots(figsize=(7, 7))
    xs, ys = zip(*pts)
    ax.scatter(xs, ys, color="tomato", s=10, label="Noisy points")
    if result:
        circle = Circle(
            (fc[0], fc[1]),
            fr,
            fill=False,
            color="forestgreen",
            linewidth=2,
            label="Fitted circle",
        )
        ax.add_patch(circle)
        ax.scatter(
            fc[0], fc[1], color="forestgreen", marker="x", s=100, linewidths=2
        )
    ax.set_aspect("equal")
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.grid(True, alpha=0.3)
    ax.legend()
    ax.set_title(
        f"Fit circle to points (error: {ferr:.4f})"
        if result
        else "Circle fit failed"
    )

    fig.tight_layout()
    path = output_dir / "fitting-circle.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "fitting-circle.png",
            "caption": "Circle fitted to noisy point cloud",
        }
    )

    n_arc = 30
    arc_pts = [
        (
            50 + 30 * math.cos(math.pi * i / n_arc),
            50 + 30 * math.sin(math.pi * i / n_arc),
        )
        for i in range(n_arc + 1)
    ]

    raw_geom = Geometry.from_points(arc_pts, close=False)
    fit_geom = raw_geom.fit_curves(3.0, arcs=True, beziers=False)

    fit_flat = flatten_to_points(fit_geom, 0.5)
    fit_pts = fit_flat[0] if fit_flat else []

    fig2, axes2 = plt.subplots(1, 2, figsize=(14, 6))
    for ax_i in axes2:
        ax_i.set_aspect("equal")
        ax_i.set_xlim(0, 100)
        ax_i.set_ylim(0, 100)
        ax_i.grid(True, alpha=0.3)

    axes2[0].plot(
        [p[0] for p in arc_pts],
        [p[1] for p in arc_pts],
        "o-",
        color="tomato",
        markersize=3,
        linewidth=1,
        label="Original points",
    )
    axes2[0].set_title("Original polyline")

    if fit_pts:
        axes2[1].plot(
            [p[0] for p in fit_pts],
            [p[1] for p in fit_pts],
            color="forestgreen",
            linewidth=2.5,
            label=f"Fitted ({len(fit_geom)} cmds)",
        )
        axes2[1].legend()
    axes2[1].set_title("Fitted primitives (tol=3.0)")

    fig2.tight_layout()
    path2 = output_dir / "fitting-primitives.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "fitting-primitives.png",
            "caption": "Polyline fitted with arc and line primitives",
        }
    )

    # ── fit_circle_to_3_points ──────────────────────────────────────────
    p1 = (25.0, 25.0)
    p2 = (75.0, 30.0)
    p3 = (50.0, 75.0)

    result_3 = fit_circle_to_3_points(p1, p2, p3)
    if result_3:
        c3, r3 = result_3

        fig3, ax3 = plt.subplots(figsize=(7, 7))
        for label, pt in [("p1", p1), ("p2", p2), ("p3", p3)]:
            ax3.scatter(*pt, color="tomato", s=100, zorder=5)
            ax3.annotate(
                label,
                pt,
                textcoords="offset points",
                xytext=(-12, 8),
                fontsize=11,
                fontweight="bold",
            )
        circle3 = Circle(
            c3,
            r3,
            fill=False,
            color="forestgreen",
            linewidth=2.5,
            label="Fitted circle",
        )
        ax3.add_patch(circle3)
        ax3.scatter(
            *c3,
            color="forestgreen",
            marker="x",
            s=120,
            linewidths=2.5,
            zorder=5,
        )
        ax3.annotate(
            f"C ({c3[0]:.2f}, {c3[1]:.2f})",
            c3,
            textcoords="offset points",
            xytext=(8, -12),
            fontsize=10,
        )
        for pt in [p1, p2, p3]:
            ax3.plot(
                [c3[0], pt[0]],
                [c3[1], pt[1]],
                "--",
                color="gray",
                linewidth=0.8,
            )
        ax3.set_aspect("equal")
        ax3.set_xlim(0, 100)
        ax3.set_ylim(0, 100)
        ax3.grid(True, alpha=0.3)
        ax3.legend(fontsize=10)
        ax3.set_title(
            f"Circle fitted to 3 points  (R = {r3:.2f})", fontsize=12
        )
        fig3.tight_layout()
        fig3.savefig(output_dir / "fitting-3-points.png", dpi=150)
        plt.close(fig3)
        images.append(
            {
                "path": "fitting-3-points.png",
                "caption": "Unique circle passing through three points",
            }
        )

    # ── flatten_to_points ──────────────────────────────────────────────
    arc_seed = [
        (
            50 + 35 * math.cos(math.pi * i / 30),
            50 + 35 * math.sin(math.pi * i / 30),
        )
        for i in range(31)
    ]
    arc_raw = Geometry.from_points(arc_seed, close=False)
    arc_fitted = arc_raw.fit_curves(1.0, arcs=True, beziers=False)
    flat_pts = flatten_to_points(arc_fitted, 2.0)
    flat_pts = flat_pts[0] if flat_pts else []

    fine_flat = flatten_to_points(arc_fitted, 0.1)
    fine_pts = fine_flat[0] if fine_flat else []

    fig_fl, (ax_fl1, ax_fl2) = plt.subplots(1, 2, figsize=(14, 6))
    for ax in [ax_fl1, ax_fl2]:
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)

    if fine_pts:
        ax_fl1.plot(
            [p[0] for p in fine_pts],
            [p[1] for p in fine_pts],
            "-",
            color="dodgerblue",
            linewidth=2.5,
            label="Arc curve",
        )
    ax_fl1.plot(
        [arc_seed[0][0]],
        [arc_seed[0][1]],
        "o",
        color="tomato",
        ms=5,
    )
    ax_fl1.set_title("Original: arc curve", fontsize=12)

    if flat_pts:
        ax_fl2.plot(
            [p[0] for p in flat_pts],
            [p[1] for p in flat_pts],
            "o-",
            color="darkorange",
            linewidth=1.5,
            markersize=3,
            label=f"Flattened ({len(flat_pts)} pts)",
        )
    ax_fl2.set_title("After: flatten_to_points(tol=2.0)", fontsize=12)

    fig_fl.tight_layout()
    fig_fl.savefig(output_dir / "fitting-flatten.png", dpi=150)
    plt.close(fig_fl)
    images.append(
        {
            "path": "fitting-flatten.png",
            "caption": "Arc curve flattened to dense line segments",
        }
    )

    # ── linearize_geometry ─────────────────────────────────────────────
    lin_seed = arc_seed  # reuse same semi-circle
    lin_raw = Geometry.from_points(lin_seed, close=False)
    lin_fitted = lin_raw.fit_curves(1.0, arcs=True, beziers=False)
    linearized = linearize_geometry(lin_fitted, 3.0)

    lin_flat = flatten_to_points(linearized, 0.5)
    lin_pts = lin_flat[0] if lin_flat else []

    fig_lin, (ax_lin1, ax_lin2) = plt.subplots(1, 2, figsize=(14, 6))
    for ax in [ax_lin1, ax_lin2]:
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 100)
        ax.grid(True, alpha=0.3)

    if fine_pts:
        ax_lin1.plot(
            [p[0] for p in fine_pts],
            [p[1] for p in fine_pts],
            "-",
            color="dodgerblue",
            linewidth=2.5,
            label="Arc curve",
        )
    ax_lin1.set_title("Original: arc curve", fontsize=12)

    if lin_pts:
        ax_lin2.plot(
            [p[0] for p in lin_pts],
            [p[1] for p in lin_pts],
            "o-",
            color="crimson",
            linewidth=2,
            markersize=5,
            label=f"Linearized ({len(lin_pts)} pts, tol=3.0)",
        )
    ax_lin2.set_title("After: linearize_geometry(tol=3.0)", fontsize=12)

    fig_lin.tight_layout()
    fig_lin.savefig(output_dir / "fitting-linearize.png", dpi=150)
    plt.close(fig_lin)
    images.append(
        {
            "path": "fitting-linearize.png",
            "caption": "Arc curve linearized with RDP simplification",
        }
    )

    # ── get_polyline_arc_deviation ─────────────────────────────────────
    n_dev = 15
    cx_dev, cy_dev, r_dev = 50.0, 50.0, 30.0
    dev_pts = [
        (
            cx_dev + (r_dev + 0.0) * math.cos(2 * math.pi * i / n_dev),
            cy_dev + (r_dev + 0.0) * math.sin(2 * math.pi * i / n_dev),
        )
        for i in range(n_dev)
    ]
    # Add some deviation to a few points
    rng = np.random.default_rng(7)
    noisy = []
    for x, y in dev_pts:
        nx = x + rng.normal(0, 2.0)
        ny = y + rng.normal(0, 2.0)
        noisy.append((nx, ny))

    noisy_3d = [(x, y, 0.0) for x, y in noisy]
    max_arc_dev = get_polyline_arc_deviation(noisy_3d, (cx_dev, cy_dev), r_dev)

    fig_arc, ax_arc = plt.subplots(figsize=(7, 7))
    ref_circle = Circle(
        (cx_dev, cy_dev),
        r_dev,
        fill=False,
        color="forestgreen",
        linewidth=2,
        linestyle="--",
        label=f"Reference arc (R={r_dev})",
    )
    ax_arc.add_patch(ref_circle)
    ax_arc.scatter(
        cx_dev,
        cy_dev,
        color="forestgreen",
        marker="+",
        s=80,
        linewidths=1.5,
    )
    xs_n = [p[0] for p in noisy]
    ys_n = [p[1] for p in noisy]
    ax_arc.plot(
        xs_n,
        ys_n,
        "o-",
        color="tomato",
        markersize=5,
        linewidth=1.2,
        label="Polyline points",
    )
    # Highlight max deviation point
    devs = []
    for x, y in noisy:
        d = abs(math.hypot(x - cx_dev, y - cy_dev) - r_dev)
        devs.append(d)
    max_idx = devs.index(max(devs))
    ax_arc.scatter(
        noisy[max_idx][0],
        noisy[max_idx][1],
        color="red",
        s=120,
        zorder=5,
        marker="x",
        linewidths=2.5,
    )
    # Draw deviation annotation
    mx, my = noisy[max_idx]
    angle = math.atan2(my - cy_dev, mx - cx_dev)
    near_x = cx_dev + r_dev * math.cos(angle)
    near_y = cy_dev + r_dev * math.sin(angle)
    ax_arc.plot(
        [mx, near_x],
        [my, near_y],
        "r-",
        linewidth=1.5,
    )
    ax_arc.annotate(
        f"Max deviation = {max_arc_dev:.3f}",
        ((mx + near_x) / 2, (my + near_y) / 2),
        textcoords="offset points",
        xytext=(10, -10),
        fontsize=10,
        color="red",
        arrowprops=dict(arrowstyle="->", color="red", lw=1),
    )
    ax_arc.set_aspect("equal")
    ax_arc.set_xlim(0, 100)
    ax_arc.set_ylim(0, 100)
    ax_arc.grid(True, alpha=0.3)
    ax_arc.legend(fontsize=9)
    ax_arc.set_title(
        f"Polyline arc deviation  (max = {max_arc_dev:.3f})", fontsize=12
    )
    fig_arc.tight_layout()
    fig_arc.savefig(output_dir / "fitting-arc-deviation.png", dpi=150)
    plt.close(fig_arc)
    images.append(
        {
            "path": "fitting-arc-deviation.png",
            "caption": "Maximum deviation of a polyline from a reference arc",
        }
    )

    # ── get_polyline_line_deviation ────────────────────────────────────
    n_line = 12
    ln_x = np.linspace(20, 80, n_line)
    ln_y = 30 + 5 * np.sin(np.linspace(0, math.pi, n_line))
    line_pts_3d = [(float(x), float(y), 0.0) for x, y in zip(ln_x, ln_y)]

    max_ld, max_li = get_polyline_line_deviation(line_pts_3d, 0, n_line - 1)

    fig_ln, ax_ln = plt.subplots(figsize=(8, 5))
    xs_ln = [float(x) for x in ln_x]
    ys_ln = [float(y) for y in ln_y]
    ax_ln.plot(
        xs_ln,
        ys_ln,
        "o-",
        color="tomato",
        markersize=6,
        linewidth=1.5,
        label="Polyline points",
    )
    ax_ln.plot(
        [line_pts_3d[0][0], line_pts_3d[-1][0]],
        [line_pts_3d[0][1], line_pts_3d[-1][1]],
        "--",
        color="dodgerblue",
        linewidth=2,
        label="Chord (start → end)",
    )
    # Highlight furthest point
    ax_ln.scatter(
        line_pts_3d[max_li][0],
        line_pts_3d[max_li][1],
        color="red",
        s=150,
        zorder=5,
        marker="x",
        linewidths=2.5,
    )
    # Perpendicular deviation line
    xs, ys = line_pts_3d[0][0], line_pts_3d[0][1]
    xe, ye = line_pts_3d[-1][0], line_pts_3d[-1][1]
    dxc, dyc = xe - xs, ye - ys
    l2 = dxc * dxc + dyc * dyc
    px, py = line_pts_3d[max_li][0], line_pts_3d[max_li][1]
    t = ((px - xs) * dxc + (py - ys) * dyc) / l2 if l2 > 0 else 0
    proj_x = xs + t * dxc
    proj_y = ys + t * dyc
    ax_ln.plot([px, proj_x], [py, proj_y], "r-", linewidth=1.5)
    ax_ln.annotate(
        f"Max deviation = {max_ld:.3f}",
        ((px + proj_x) / 2, (py + proj_y) / 2),
        textcoords="offset points",
        xytext=(10, -15),
        fontsize=10,
        color="red",
        arrowprops=dict(arrowstyle="->", color="red", lw=1),
    )
    ax_ln.set_aspect("equal")
    ax_ln.set_xlim(0, 100)
    ax_ln.set_ylim(0, 60)
    ax_ln.grid(True, alpha=0.3)
    ax_ln.legend(fontsize=10)
    ax_ln.set_title(
        f"Polyline line deviation  (max = {max_ld:.3f} at index {max_li})",
        fontsize=12,
    )
    fig_ln.tight_layout()
    fig_ln.savefig(output_dir / "fitting-line-deviation.png", dpi=150)
    plt.close(fig_ln)
    images.append(
        {
            "path": "fitting-line-deviation.png",
            "caption": (
                "Maximum perpendicular deviation of a polyline from its chord"
            ),
        }
    )

    # ── project_circle_center_to_bisector ──────────────────────────────
    bp1 = (25.0, 30.0)
    bp2 = (75.0, 30.0)
    bc = (42.0, 55.0)  # original center (not on bisector)
    bproj = project_circle_center_to_bisector((*bp1, 0.0), (*bp2, 0.0), bc)

    # Midpoint of chord
    bmx = (bp1[0] + bp2[0]) / 2
    bmy = (bp1[1] + bp2[1]) / 2
    # Perpendicular bisector direction (perpendicular to chord)
    chord_dx = bp2[0] - bp1[0]
    chord_dy = bp2[1] - bp1[1]
    bis_len = 35.0
    # Normalize perpendicular
    perp_norm = math.hypot(-chord_dy, chord_dx)
    perp_dx = -chord_dy / perp_norm * bis_len
    perp_dy = chord_dx / perp_norm * bis_len

    fig_b, ax_b = plt.subplots(figsize=(7, 7))
    ax_b.plot(
        [bp1[0], bp2[0]],
        [bp1[1], bp2[1]],
        "o-",
        color="dodgerblue",
        linewidth=2.5,
        markersize=8,
        label="Chord (p1 → p2)",
    )
    ax_b.annotate(
        "p1",
        bp1,
        textcoords="offset points",
        xytext=(-12, 8),
        fontsize=11,
        fontweight="bold",
    )
    ax_b.annotate(
        "p2",
        bp2,
        textcoords="offset points",
        xytext=(8, 8),
        fontsize=11,
        fontweight="bold",
    )
    # Perpendicular bisector
    ax_b.plot(
        [bmx - perp_dx, bmx + perp_dx],
        [bmy - perp_dy, bmy + perp_dy],
        "--",
        color="gray",
        linewidth=1.5,
        label="Perpendicular bisector",
    )
    # Original center
    ax_b.scatter(*bc, color="tomato", s=120, zorder=5, marker="o")
    ax_b.annotate(
        "C (original)",
        bc,
        textcoords="offset points",
        xytext=(8, -15),
        fontsize=10,
        color="tomato",
    )
    # Projected center
    ax_b.scatter(*bproj, color="forestgreen", s=120, zorder=5, marker="s")
    ax_b.annotate(
        "C' (projected)",
        bproj,
        textcoords="offset points",
        xytext=(8, 8),
        fontsize=10,
        color="forestgreen",
    )
    # Projection line
    ax_b.plot(
        [bc[0], bproj[0]], [bc[1], bproj[1]], "r-", linewidth=1.5, alpha=0.7
    )
    ax_b.set_aspect("equal")
    ax_b.set_xlim(0, 100)
    ax_b.set_ylim(0, 100)
    ax_b.grid(True, alpha=0.3)
    ax_b.legend(fontsize=10)
    ax_b.set_title(
        "Project circle center onto perpendicular bisector", fontsize=12
    )
    fig_b.tight_layout()
    fig_b.savefig(output_dir / "fitting-project-bisector.png", dpi=150)
    plt.close(fig_b)
    images.append(
        {
            "path": "fitting-project-bisector.png",
            "caption": (
                "Circle center projected onto the perpendicular bisector"
                " of chord p1-p2"
            ),
        }
    )

    return {
        "title": "Fitting",
        "description": (
            "Fit circles to point clouds and fit polylines with arc/line "
            "primitives."
        ),
        "images": images,
    }
