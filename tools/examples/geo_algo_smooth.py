"""Generate smoothing example images."""

import math

import matplotlib.pyplot as plt

from raygeo.geo.algo.smooth import (
    blend_tangent,
    build_smoothed_path,
    chaikin_corner_cut,
    compute_gaussian_kernel,
    shortcut_path,
    smooth_circularly,
    smooth_path,
    smooth_polyline_3d,
    smooth_sub_segment,
)


def generate_overview():
    """Smooth overview."""
    n = 30
    pts = [
        (
            50 + 30 * math.cos(2 * math.pi * i / n) + (i % 3) * 5,
            50 + 30 * math.sin(2 * math.pi * i / n) + (i % 4) * 4,
        )
        for i in range(n)
    ]
    pts_3d = [(x, y, 0.0) for x, y in pts]

    fig, axes = plt.subplots(2, 4, figsize=(20, 9))

    def draw(ax, points, title, color, xlim=(0, 100), ylim=(0, 100)):
        sx, sy = zip(*[(p[0], p[1]) for p in points])
        ax.plot(
            sx + (sx[0],),
            sy + (sy[0],),
            color=color,
            linewidth=2.5,
        )
        ax.set_title(title)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_xlim(*xlim)
        ax.set_ylim(*ylim)

    amounts = [50, 100, 200]

    xs, ys = zip(*pts)
    draw(axes[0, 0], pts, "Original", "gray")
    draw(axes[1, 0], pts, "Original", "gray")

    for col, amount in enumerate(amounts, 1):
        smoothed_no_preserve = smooth_polyline_3d(pts_3d, amount, 0.0, True)
        draw(
            axes[0, col],
            smoothed_no_preserve,
            f"Smooth {amount}, no preserve",
            "tomato",
        )

        smoothed_preserve = smooth_polyline_3d(pts_3d, amount, 120.0, True)
        draw(
            axes[1, col],
            smoothed_preserve,
            f"Smooth {amount}, preserve<120°",
            "forestgreen",
        )

    fig.tight_layout()
    return fig


def generate_gaussian_kernel():
    """Gaussian kernel."""
    amounts_k = [10, 30, 60, 100]
    fig_k, axes_k = plt.subplots(1, 4, figsize=(16, 4))

    for idx, amt in enumerate(amounts_k):
        kernel, sigma = compute_gaussian_kernel(amt)
        xs_k = list(range(len(kernel)))
        axes_k[idx].bar(xs_k, kernel, color="steelblue", width=0.8)
        axes_k[idx].set_title(f"amount={amt}  (σ={sigma:.2f})", fontsize=10)
        axes_k[idx].set_xlabel("Index")
        axes_k[idx].set_ylabel("Weight")
        axes_k[idx].grid(True, alpha=0.3, axis="y")

    fig_k.suptitle("Gaussian kernels at different amounts", fontsize=13)
    fig_k.tight_layout()
    return fig_k


def generate_circular():
    """Circular."""
    n_star = 25
    star_pts = [
        (
            50
            + 28 * math.cos(2 * math.pi * i / n_star)
            + (8 if i % 2 == 0 else -8) * math.cos(2 * math.pi * i / n_star),
            50
            + 28 * math.sin(2 * math.pi * i / n_star)
            + (8 if i % 2 == 0 else -8) * math.sin(2 * math.pi * i / n_star),
        )
        for i in range(n_star)
    ]
    star_3d = [(x, y, 0.0) for x, y in star_pts]

    kernel_s, _ = compute_gaussian_kernel(40)

    # Apply circular smoothing with varying kernel amounts
    fig_sc, axes_sc = plt.subplots(1, 3, figsize=(15, 5))
    amounts_circ = [20, 40, 80]
    titles_circ = ["amount=20", "amount=40", "amount=80"]

    for idx, amt in enumerate(amounts_circ):
        k, _ = compute_gaussian_kernel(amt)
        smoothed = smooth_circularly(star_3d, k)
        ax = axes_sc[idx]
        sx_star = [p[0] for p in star_pts] + [star_pts[0][0]]
        sy_star = [p[1] for p in star_pts] + [star_pts[0][1]]
        ax.plot(
            sx_star,
            sy_star,
            "-",
            color="gray",
            linewidth=1,
            alpha=0.6,
            label="Original",
        )
        ssx = [p[0] for p in smoothed]
        ssy = [p[1] for p in smoothed]
        ax.plot(
            ssx, ssy, "-", color="crimson", linewidth=2.5, label="Smoothed"
        )
        ax.set_title(titles_circ[idx], fontsize=11)
        ax.set_aspect("equal")
        ax.set_xlim(5, 95)
        ax.set_ylim(5, 95)
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)

    fig_sc.suptitle("Circular smoothing of a closed polyline", fontsize=13)
    fig_sc.tight_layout()
    return fig_sc


def generate_sub_segment():
    """Sub segment."""
    n_sub = 20
    zig = [(i * 4 + 10, 20 + 5 * math.sin(i * 0.8)) for i in range(n_sub)]
    zig_3d = [(x, y, 0.0) for x, y in zig]

    kernel_sub, _ = compute_gaussian_kernel(30)
    out = smooth_sub_segment(zig_3d, kernel_sub)

    fig_sub, ax_sub = plt.subplots(figsize=(8, 5))
    zx_s, zy_s = zip(*zig)
    ax_sub.plot(
        zx_s,
        zy_s,
        "o-",
        color="gray",
        linewidth=1.5,
        markersize=4,
        alpha=0.7,
        label="Original",
    )
    sx_s = [p[0] for p in out]
    sy_s = [p[1] for p in out]
    ax_sub.plot(
        sx_s,
        sy_s,
        "o-",
        color="crimson",
        linewidth=2.5,
        markersize=4,
        label="Smoothed sub-segment",
    )
    # Highlight endpoints preserved
    for label, pt in [("start", zig[0]), ("end", zig[-1])]:
        ax_sub.scatter(*pt, color="forestgreen", s=80, zorder=5, marker="s")
        ax_sub.annotate(
            f"{label} preserved",
            pt,
            textcoords="offset points",
            xytext=(5, -12),
            fontsize=9,
            color="forestgreen",
        )
    ax_sub.set_aspect("equal")
    ax_sub.set_xlim(0, 100)
    ax_sub.set_ylim(0, 50)
    ax_sub.grid(True, alpha=0.3)
    ax_sub.legend(fontsize=10)
    ax_sub.set_title(
        "Sub-segment smoothing (endpoints preserved)", fontsize=12
    )
    fig_sub.tight_layout()
    return fig_sub


def generate_smooth_path():
    """Constrained path smoothing around obstacles."""
    obstacle = [(35, 30), (55, 30), (55, 55), (35, 55)]

    raw_path = [
        (5, 45),
        (20, 45),
        (25, 45),
        (30, 45),
        (45, 75),
        (60, 75),
        (70, 45),
        (75, 45),
        (80, 45),
        (95, 45),
    ]

    clearance = 3.0
    amounts = [0, 30, 80]

    fig_sp, axes_sp = plt.subplots(
        1, len(amounts) + 1, figsize=(5 * (len(amounts) + 1), 5)
    )

    def draw_panel(ax, path, title):
        obs_x = [p[0] for p in obstacle] + [obstacle[0][0]]
        obs_y = [p[1] for p in obstacle] + [obstacle[0][1]]
        ax.fill(obs_x, obs_y, color="dimgray", alpha=0.3, zorder=1)
        ax.plot(obs_x, obs_y, color="dimgray", linewidth=1.5, zorder=2)

        px = [p[0] for p in path]
        py = [p[1] for p in path]
        ax.plot(
            px, py, "o-", color="crimson", linewidth=2, markersize=3, zorder=3
        )

        ax.set_title(title, fontsize=11)
        ax.set_aspect("equal")
        ax.set_xlim(0, 100)
        ax.set_ylim(20, 90)
        ax.grid(True, alpha=0.3)

    draw_panel(axes_sp[0], raw_path, f"Original ({len(raw_path)} pts)")

    for idx, amt in enumerate(amounts):
        smoothed = smooth_path(raw_path, [obstacle], clearance, amt)
        draw_panel(
            axes_sp[idx + 1],
            smoothed,
            f"amount={amt} ({len(smoothed)} pts)",
        )

    fig_sp.suptitle(
        "Constrained smoothing — path avoids obstacle", fontsize=13
    )
    fig_sp.tight_layout()
    return fig_sp


def generate_shortcut_path():
    """Shortcut path: iterative collision-free waypoint removal.

    The input path goes around a tall obstacle by dropping below it,
    with many redundant waypoints on the straight sections.  Shortcutting
    removes the redundant waypoints while preserving the dip that clears
    the obstacle.
    """
    obstacle = [(40, 20), (60, 20), (60, 80), (40, 80)]
    raw_path = [
        (10, 50),
        (20, 50),
        (30, 50),
        (30, 5),
        (50, 5),
        (70, 5),
        (70, 50),
        (80, 50),
        (90, 50),
    ]
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(10, 4))

    def draw(ax, pts, title):
        xs, ys = zip(*pts)
        ax.plot(xs, ys, "-o", color="tab:blue", lw=2, markersize=5)
        ox, oy = zip(*(obstacle + [obstacle[0]]))
        ax.fill(ox, oy, alpha=0.3, color="tab:red", label="obstacle")
        ax.set_xlim(0, 100)
        ax.set_ylim(-5, 90)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)
        ax.set_title(title, fontsize=11)

    draw(ax1, raw_path, f"Before ({len(raw_path)} pts)")
    result = shortcut_path(raw_path, [obstacle], 1.0)
    draw(ax2, result, f"After ({len(result)} pts)")
    fig.tight_layout()
    return fig


def generate_chaikin_corner_cut():
    """Chaikin corner cutting on a stair-step polyline.

    The obstacle sits inside the corner at (40, 40).  Without the
    obstacle every sharp corner is rounded; with it, that corner is
    preserved because the rounded sweep would cut into the obstacle.
    """
    pts = [(10, 70), (40, 70), (40, 40), (70, 40), (70, 10), (90, 10)]
    obstacle = [(43, 43), (48, 43), (48, 48), (43, 48)]

    fig, (ax1, ax2, ax3) = plt.subplots(1, 3, figsize=(15, 5))

    def draw(ax, points, title, show_obs):
        xs, ys = zip(*points)
        ax.plot(xs, ys, "-o", color="tab:blue", lw=2, markersize=5)
        if show_obs:
            ox, oy = zip(*(obstacle + [obstacle[0]]))
            ax.fill(ox, oy, alpha=0.3, color="tab:red", label="Obstacle")
            ax.legend(fontsize=8)
        ax.set_xlim(0, 100)
        ax.set_ylim(0, 80)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.set_title(title, fontsize=11)

    draw(ax1, pts, f"Original ({len(pts)} pts)", show_obs=False)
    result = chaikin_corner_cut(pts, [], 1.0, 3)
    draw(ax2, result, f"No obstacles ({len(result)} pts)", show_obs=False)
    result_safe = chaikin_corner_cut(pts, [obstacle], 1.0, 3)
    draw(
        ax3,
        result_safe,
        f"With obstacle ({len(result_safe)} pts)",
        show_obs=True,
    )

    fig.suptitle(
        "chaikin_corner_cut — collision-aware corner rounding", fontsize=13
    )
    fig.tight_layout()
    return fig


def generate_build_smoothed_path():
    """Multi-stage smooth path construction.

    Path routes from bottom-left to bottom-right, detouring up and
    around a central obstacle.  Smoothing rounds the corners while
    respecting clearance.
    """
    last = (10, 50)
    first = (90, 50)
    waypoints = [(30, 50), (30, 85), (70, 85), (70, 50)]
    obstacle = [(35, 30), (65, 30), (65, 75), (35, 75)]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    def draw(ax, pts, title):
        xs, ys = zip(*pts)
        ax.plot(xs, ys, "-o", color="tab:blue", lw=2, markersize=4)
        ox, oy = zip(*(obstacle + [obstacle[0]]))
        ax.fill(ox, oy, alpha=0.3, color="tab:red", label="Obstacle")
        ax.plot(last[0], last[1], "s", color="green", ms=8, label="Start")
        ax.plot(first[0], first[1], "D", color="purple", ms=8, label="End")
        ax.set_xlim(-5, 105)
        ax.set_ylim(15, 95)
        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=8)
        ax.set_title(title, fontsize=11)

    draw(
        ax1, [last] + waypoints + [first], f"Input ({len(waypoints) + 2} pts)"
    )
    result = build_smoothed_path(last, first, waypoints, [obstacle], 3.0, 80)
    draw(ax2, result, f"Smoothed ({len(result)} pts)")

    fig.suptitle("build_smoothed_path — multi-stage smooth path", fontsize=13)
    fig.tight_layout()
    return fig


def generate_blend_tangent():
    """Show blend_tangent inserting G1 extension points."""
    link = [(0.0, 0.0), (100.0, 0.0)]
    prev_tail = [(-20.0, 30.0), (0.0, 0.0)]
    next_head = [(100.0, 0.0), (120.0, 30.0)]

    result = blend_tangent(link, prev_tail, next_head, 10.0)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))

    # Left: before blending
    ax1.set_title("Before blend_tangent")
    ax1.set_aspect("equal")
    # Previous cut
    px, py = zip(*prev_tail)
    ax1.plot(px, py, "o-", color="blue", linewidth=2, label="Prev cut")
    # Link (travel segment)
    lx, ly = zip(*link)
    ax1.plot(lx, ly, "s--", color="orange", linewidth=2.5, label="Link")
    # Next cut
    nx, ny = zip(*next_head)
    ax1.plot(nx, ny, "o-", color="green", linewidth=2, label="Next cut")
    # Mark the sharp junctions
    ax1.plot(link[0][0], link[0][1], "ro", markersize=8, zorder=5)
    ax1.plot(link[-1][0], link[-1][1], "ro", markersize=8, zorder=5)
    ax1.legend(fontsize=8)
    ax1.grid(True, alpha=0.3)

    # Right: after blending
    ax2.set_title("After blend_tangent")
    ax2.set_aspect("equal")
    px2, py2 = zip(*prev_tail)
    ax2.plot(
        px2, py2, "o-", color="blue", linewidth=2, alpha=0.4, label="Prev cut"
    )
    rx, ry = zip(*result)
    ax2.plot(rx, ry, "s-", color="orange", linewidth=2.5, label="Blended link")
    ax2.plot(result[0][0], result[0][1], "ro", markersize=8, zorder=5)
    ax2.plot(result[-1][0], result[-1][1], "ro", markersize=8, zorder=5)
    nx2, ny2 = zip(*next_head)
    ax2.plot(
        nx2, ny2, "o-", color="green", linewidth=2, alpha=0.4, label="Next cut"
    )

    # Highlight inserted points
    if len(result) > 2:
        interior = result[1:-1]
        ix, iy = zip(*interior)
        ax2.scatter(ix, iy, color="red", s=40, zorder=6, label="Extension pts")

    ax2.legend(fontsize=8)
    ax2.grid(True, alpha=0.3)

    fig.suptitle(
        f"blend_tangent — inserts {len(result) - 2} G1 extension points",
        fontsize=12,
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.smooth.md"]
__images__ = [
    {
        "heading": "chaikin_corner_cut",
        "caption": (
            "``chaikin_corner_cut`` rounds sharp corners via"
            " Chaikin cut, respecting obstacle clearance"
        ),
        "function": generate_chaikin_corner_cut,
    },
    {
        "heading": "build_smoothed_path",
        "caption": (
            "``build_smoothed_path`` builds a smooth path via"
            " resample → shortcut → Gaussian relaxation"
        ),
        "function": generate_build_smoothed_path,
    },
    {
        "heading": "smooth_polyline_3d",
        "caption": "Gaussian smoothing",
        "function": generate_overview,
    },
    {
        "heading": "compute_gaussian_kernel",
        "caption": "Gaussian kernel weights",
        "function": generate_gaussian_kernel,
    },
    {
        "heading": "smooth_circularly",
        "caption": "Circular smoothing",
        "function": generate_circular,
    },
    {
        "heading": "smooth_sub_segment",
        "caption": "Sub-segment smoothing",
        "function": generate_sub_segment,
    },
    {
        "heading": "smooth_path",
        "caption": "Constrained path smoothing",
        "function": generate_smooth_path,
    },
    {
        "heading": "shortcut_path",
        "caption": "Iterative waypoint removal",
        "function": generate_shortcut_path,
    },
    {
        "heading": "blend_tangent",
        "caption": (
            "``blend_tangent`` adds tangent points at travel-link ends"
            " for G1 continuity at cut–travel junctions"
        ),
        "function": generate_blend_tangent,
    },
]
