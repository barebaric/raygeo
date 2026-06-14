"""Generate bezier curve example images."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.shape.bezier import (
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


def generate_examples(output_dir):
    images = []

    p0, p1, p2, p3 = (0.0, 0.0), (0.0, 12.0), (15.0, 12.0), (15.0, 0.0)

    t_split = 0.4
    left, right = split_bezier(p0, p1, p2, p3, t_split)
    split_pt = get_bezier_point_at(p0, p1, p2, p3, t_split)
    t_mid = 0.6

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
    path = output_dir / "bezier-split.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "bezier-split.png",
            "caption": "Bezier split at parameter t",
        }
    )

    fig2, ax = plt.subplots(1, 1, figsize=(7, 6))
    _plot_bezier(ax, curve, "steelblue", linewidth=3)
    mid_pt = get_bezier_point_at(p0, p1, p2, p3, t_mid)
    ax.plot(
        mid_pt[0], mid_pt[1], "o", color="tomato", markersize=12, zorder=5
    )
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
    path2 = output_dir / "bezier-point-at.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "bezier-point-at.png",
            "caption": "Bezier point evaluation at parameter t",
        }
    )

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
    path2 = output_dir / "bezier-flatten.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "bezier-flatten.png",
            "caption": (
                "Bezier flattening: adaptive subdivision at varied tolerances"
            ),
        }
    )

    return {
        "title": "Bezier Curves",
        "description": (
            "Cubic bezier curve operations: splitting, point evaluation, "
            "and flattening to line segments."
        ),
        "images": images,
    }
