"""Generate bezier curve example images."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.shape.bezier import (
    fit_cubic_bezier,
    get_bezier_point_at,
    linearize_bezier_adaptive,
    split_bezier,
)


def _eval_bezier(p0, p1, p2, p3, n=100):
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


def _plot_bezier(ax, pts, color, linewidth=3, label=None, ls="-"):
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    ax.plot(
        xs, ys, color=color, linewidth=linewidth, label=label, linestyle=ls
    )


def generate_split():
    p0, p1, p2, p3 = (0.0, 0.0), (0.0, 12.0), (15.0, 12.0), (15.0, 0.0)

    t_split = 0.4
    left, right = split_bezier(p0, p1, p2, p3, t_split)
    split_pt = get_bezier_point_at(p0, p1, p2, p3, t_split)

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    curve = _eval_bezier(p0, p1, p2, p3)
    _plot_bezier(axes[0], curve, "steelblue", linewidth=3)
    for cp, clr in [(p1, "tomato"), (p2, "forestgreen")]:
        axes[0].plot(cp[0], cp[1], "o", color=clr, markersize=8, zorder=5)
    axes[0].plot(p0[0], p0[1], "o", color="gray", markersize=6, zorder=5)
    axes[0].plot(p3[0], p3[1], "o", color="gray", markersize=6, zorder=5)
    axes[0].set_title("Cubic bezier with control points", fontsize=14)

    left_curve = _eval_bezier(*left)
    right_curve = _eval_bezier(*right)
    _plot_bezier(
        axes[1], curve, "gray", linewidth=2, ls="--", label="Original"
    )
    _plot_bezier(axes[1], left_curve, "tomato", linewidth=3, label="Left half")
    _plot_bezier(
        axes[1], right_curve, "forestgreen", linewidth=3, label="Right half"
    )
    axes[1].plot(
        split_pt[0], split_pt[1], "*", color="gold", markersize=15, zorder=5
    )
    axes[1].set_title(f"Split at t={t_split}", fontsize=14)
    axes[1].legend(fontsize=10)

    for i in range(2):
        axes[i].set_aspect("equal")
        axes[i].grid(True, alpha=0.3)
        axes[i].set_xlim(-2, 17)
        axes[i].set_ylim(-2, 14)

    fig.tight_layout()
    return fig


def generate_point_at():
    p0, p1, p2, p3 = (0.0, 0.0), (0.0, 12.0), (15.0, 12.0), (15.0, 0.0)
    t_mid = 0.6

    fig2, ax = plt.subplots(1, 1, figsize=(7, 6))
    curve = _eval_bezier(p0, p1, p2, p3)
    _plot_bezier(ax, curve, "steelblue", linewidth=3)
    mid_pt = get_bezier_point_at(p0, p1, p2, p3, t_mid)
    ax.plot(mid_pt[0], mid_pt[1], "o", color="tomato", markersize=12, zorder=5)
    ax.annotate(
        f"t={t_mid}",
        mid_pt,
        xytext=(8, 8),
        textcoords="offset points",
        fontsize=13,
        color="tomato",
        fontweight="bold",
    )
    ax.set_title("Evaluate point at parameter t", fontsize=14)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.set_xlim(-2, 17)
    ax.set_ylim(-2, 14)
    fig2.tight_layout()
    return fig2


def generate_flatten():
    fig2, axes2 = plt.subplots(1, 4, figsize=(24, 6))

    p0_2, p1_2, p2_2, p3_2 = (0.0, 0.0), (5.0, 15.0), (15.0, -5.0), (20.0, 5.0)
    curve2 = _eval_bezier(p0_2, p1_2, p2_2, p3_2)
    _plot_bezier(axes2[0], curve2, "steelblue", linewidth=3)
    for cp, clr in [(p1_2, "tomato"), (p2_2, "forestgreen")]:
        axes2[0].plot(cp[0], cp[1], "o", color=clr, markersize=8, zorder=5)
    axes2[0].plot(p0_2[0], p0_2[1], "o", color="gray", markersize=6, zorder=5)
    axes2[0].plot(p3_2[0], p3_2[1], "o", color="gray", markersize=6, zorder=5)
    axes2[0].set_title("Original bezier", fontsize=14)

    for tol_sq, color, idx in [
        (100.0, "tomato", 1),
        (10.0, "darkorange", 2),
        (1.0, "forestgreen", 3),
    ]:
        pts = linearize_bezier_adaptive(p0_2, p1_2, p2_2, p3_2, tol_sq, 10)
        xs = [pt[0] for pt in pts]
        ys = [pt[1] for pt in pts]
        axes2[idx].plot(xs, ys, "-", color=color, linewidth=2.5)
        tol = tol_sq**0.5
        axes2[idx].set_title(f"tol={tol:.0f} ({len(pts)} pts)", fontsize=14)

    for ax in axes2:
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(-2, 22)
        ax.set_ylim(-7, 17)

    fig2.tight_layout()
    return fig2


def generate_fit_cutline():
    """Fit cubic Beziers to sample point sequences.

    Demonstrates :py:func:`fit_cubic_bezier` on several synthetic
    curves — a sine arc, a cosine hump, a circular arc, and a shallow
    wave — showing how the fitted curve (solid line) matches the
    input points (scatter).
    """

    # Several point sequences to fit.
    def _points_along(f, xs):
        return [(x, f(x)) for x in xs]

    sequences = [
        (
            "Sine",
            _points_along(
                lambda x: 10 + 8 * math.sin(x * 0.25),
                [float(i) for i in range(30)],
            ),
        ),
        (
            "Cosine hump",
            _points_along(
                lambda x: 10 + 12 * (1 - math.cos(x * 0.1)),
                [float(i) for i in range(30)],
            ),
        ),
        (
            "Quarter circle",
            _points_along(
                lambda x: 10 + math.sqrt(max(0.0, 600.0 - (x - 25.0) ** 2)),
                [float(5 * i) for i in range(12)],
            ),
        ),
        (
            "Shallow wave",
            _points_along(
                lambda x: 10 + 3 * math.sin(x * 0.3),
                [float(i) for i in range(30)],
            ),
        ),
    ]

    fig, axes = plt.subplots(2, 2, figsize=(12, 10))
    colors = ["#e41a1c", "#377eb8", "#4daf4a", "#984ea3"]

    for ax, (name, pts), color in zip(axes.flat, sequences, colors):
        # fit Bezier
        bz = fit_cubic_bezier(pts)
        if bz is None:
            continue

        # linearize for plotting
        curve = []
        for i in range(201):
            t = i / 200
            curve.append(get_bezier_point_at(bz[0], bz[1], bz[2], bz[3], t))
        cx = [p[0] for p in curve]
        cy = [p[1] for p in curve]

        ax.plot(cx, cy, color=color, linewidth=2.5, label="Bezier fit")
        ax.scatter(
            [p[0] for p in pts],
            [p[1] for p in pts],
            color=color,
            s=20,
            alpha=0.6,
            zorder=5,
            label="Input points",
        )
        # control polygon
        cpx = [bz[0][0], bz[1][0], bz[2][0], bz[3][0]]
        cpy = [bz[0][1], bz[1][1], bz[2][1], bz[3][1]]
        ax.plot(
            cpx,
            cpy,
            color=color,
            linewidth=0.8,
            linestyle="--",
            alpha=0.5,
            label="Control poly",
        )
        ax.plot(bz[0][0], bz[0][1], "o", color=color, markersize=6)
        ax.plot(bz[3][0], bz[3][1], "o", color=color, markersize=6)

        ax.set_aspect("equal")
        ax.set_title(name)
        ax.legend(fontsize=8)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.shape.bezier.md"]
__images__ = [
    {
        "heading": "split_bezier",
        "caption": "Bezier split at parameter t",
        "function": generate_split,
    },
    {
        "heading": "get_bezier_point_at",
        "caption": "Bezier point evaluation at parameter t",
        "function": generate_point_at,
    },
    {
        "heading": "flatten_bezier",
        "caption": (
            "Bezier flattening: adaptive subdivision at varied tolerances"
        ),
        "function": generate_flatten,
    },
    {
        "heading": "fit_cubic_bezier",
        "caption": (
            "Cubic Bezier curves fitted to sample points — sine,"
            " cosine hump, quarter-circle, and shallow wave"
        ),
        "function": generate_fit_cutline,
    },
]
