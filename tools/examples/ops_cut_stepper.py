"""Generate stepper example images (step, run_segment, StepperOptions)."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle

from raygeo.geo.algo.engagement import compute_engagement
from raygeo.ops.cut.cleared_area import ClearedArea
from raygeo.ops.cut.stepper import StepperOptions, run_segment


def _engagement_at(pt, ca, tool_radius):
    """Compute engagement by measuring signed distance to cleared boundary."""
    d = ca.signed_boundary_distance(pt[0], pt[1])
    angle, _, _ = compute_engagement(d, tool_radius)
    return angle


def _generate_wall_following(metric="angle"):
    """Wall following along four shapes with vertical boundaries (1×4)."""
    tool_radius = 3.0
    step_len = 1.0
    target_eng = math.pi

    fig, axes = plt.subplots(1, 4, figsize=(16, 10))

    def _draw(ax, ca, path, xlim, ylim, title):
        engagements = [_engagement_at(p, ca, tool_radius) for p in path]
        max_eng = max(engagements) if engagements else 1.0
        norm_eng = [e / max_eng for e in engagements]

        for frag in ca.query_window((xlim[0], ylim[0], xlim[1], ylim[1])):
            fx = [p[0] for p in frag] + [frag[0][0]]
            fy = [p[1] for p in frag] + [frag[0][1]]
            ax.fill(fx, fy, "steelblue", alpha=0.2)
            ax.plot(fx, fy, "steelblue", linewidth=1, alpha=0.5)

        for i in range(len(path) - 1):
            sx, sy = [path[i][0], path[i + 1][0]], [path[i][1], path[i + 1][1]]
            c = plt.cm.RdYlGn(norm_eng[i])
            ax.plot(sx, sy, color=c, linewidth=2)

        for i in range(0, len(path), 10):
            c = Circle(
                path[i],
                tool_radius,
                fill=False,
                edgecolor=plt.cm.RdYlGn(norm_eng[i]),
                linewidth=1,
                linestyle="--",
            )
            ax.add_patch(c)

        ax.set_aspect("equal")
        ax.set_xlim(*xlim)
        ax.set_ylim(*ylim)
        ax.set_xlabel("X (mm)")
        ax.set_ylabel("Y (mm)")
        ax.set_title(title, fontsize=10)
        ax.grid(True, alpha=0.3)

    opts = StepperOptions()
    opts.radius = tool_radius
    opts.step_length = step_len
    opts.target_engagement = target_eng
    opts.max_deflection = 0.8
    opts.metric = metric

    n = 100
    y_min, y_max = 0.0, 120.0
    y_range = y_max - y_min
    left_x = -60.0
    wall_x = 20.0

    # ── 1. Curved Boundary — vertical sine wall ──
    poly = [(left_x, y_min), (left_x, y_max), (wall_x, y_max)]
    for i in range(n + 1):
        y = y_max - y_range * i / n
        poly.append((wall_x + 3.0 * math.sin(y * math.pi / 40.0), y))
    ca = ClearedArea(boundary=[])
    ca.cut([poly])
    path, _ = run_segment(ca, (5.0, 0.0), math.pi / 2.0, opts, 140)
    _draw(axes[0], ca, path, (0, 32), (-5, 125), "Curved Boundary")

    # ── 2. Square Wave — vertical ──
    sq_wall = 10.0
    poly = [(left_x, y_min), (left_x, y_max), (sq_wall, y_max)]
    period = 40.0
    amp = 15.0
    half = period / 2.0
    y = y_max
    while y > y_min:
        y_mid = max(y - half, y_min)
        poly.append((sq_wall, y_mid))
        poly.append((sq_wall + amp, y_mid))
        y = y_mid - half
        if y >= y_min:
            poly.append((sq_wall + amp, y))
            poly.append((sq_wall, y))
    ca = ClearedArea(boundary=[])
    ca.cut([poly])
    path, _ = run_segment(ca, (5.0, 0.0), math.pi / 2.0, opts, 250)
    _draw(axes[1], ca, path, (0, 35), (-5, 125), "Square Wave")

    # ── 3. Zig Zag — vertical triangle wave ──
    poly = [(left_x, y_min), (left_x, y_max), (wall_x, y_max)]
    amp = 6.0
    for i in range(n + 1):
        y = y_max - y_range * i / n
        pos = y % period
        t = pos / period
        tri = 2.0 * t if t < 0.5 else 2.0 * (1.0 - t)
        poly.append((wall_x + amp * tri, y))
    ca = ClearedArea(boundary=[])
    ca.cut([poly])
    path, _ = run_segment(ca, (5.0, 0.0), math.pi / 2.0, opts, 140)
    _draw(axes[2], ca, path, (0, 36), (-5, 125), "Zig Zag Pattern")

    # ── 4. Circle ──
    cr = 25.0
    poly = []
    for i in range(n + 1):
        a = 2.0 * math.pi * (1.0 - i / n)
        poly.append((cr * math.cos(a), cr * math.sin(a)))
    ca = ClearedArea(boundary=[])
    ca.cut([poly])
    path, _ = run_segment(ca, (0.0, cr - 0.5), 0.0, opts, 150)
    m = 8
    _draw(axes[3], ca, path, (-cr - m, cr + m), (-cr - m, cr + m), "Circle")

    fig.tight_layout()
    return fig


def generate_wall_following():
    """Wall following using angular engagement (1×4, four shapes)."""
    return _generate_wall_following("angle")


def generate_wall_following_area():
    """Wall following using area engagement (1×4, four shapes)."""
    return _generate_wall_following("area")


def generate_pocket_corner():
    """Stepping through a 90° corner, showing deflection."""
    tool_radius = 3.0
    step_len = 0.6
    target_eng = math.pi * 0.85

    ca = ClearedArea(boundary=[])
    ca.cut([[(-10, -10), (30, -10), (30, 20), (10, 20), (10, 30), (-10, 30)]])

    opts = StepperOptions()
    opts.radius = tool_radius
    opts.step_length = step_len
    opts.target_engagement = target_eng
    opts.max_deflection = 0.8
    path, _status = run_segment(ca, (25, 20), -math.pi / 2, opts, 60)

    fig, ax = plt.subplots(figsize=(8, 7))

    engagements = [_engagement_at(p, ca, tool_radius) for p in path]
    max_eng = max(engagements) if engagements else 1.0
    norm_eng = [e / max_eng for e in engagements]

    for i in range(len(path) - 1):
        seg_xs = [path[i][0], path[i + 1][0]]
        seg_ys = [path[i][1], path[i + 1][1]]
        c = plt.cm.RdYlGn(norm_eng[i])
        ax.plot(seg_xs, seg_ys, color=c, linewidth=2)

    frags = ca.query_window((-20, -20, 50, 50))
    for frag in frags:
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax.fill(fx, fy, "steelblue", alpha=0.2)

    ax.plot(path[0][0], path[0][1], "o", color="green", markersize=8, zorder=5)
    ax.plot(path[-1][0], path[-1][1], "x", color="red", markersize=8, zorder=5)

    ax.set_aspect("equal")
    ax.set_xlim(-5, 35)
    ax.set_ylim(-5, 35)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("Corner Navigation: Solver Deflects to Maintain Engagement")
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_engagement_histogram():
    """Histogram of engagement variance along a curved wall."""
    tool_radius = 3.0
    step_len = 0.5
    target_eng = math.pi * 0.85

    n = 100
    poly = [(-100.0, -100.0), (100.0, -100.0), (100.0, 20.0)]
    for i in range(n + 1):
        t = i / n
        x = 100.0 - 200.0 * t
        y = 20.0 + 5.0 * math.sin(x * math.pi / 50.0)
        poly.append((x, y))

    ca = ClearedArea(boundary=[])
    ca.cut([poly])

    opts = StepperOptions()
    opts.radius = tool_radius
    opts.step_length = step_len
    opts.target_engagement = target_eng
    opts.max_deflection = 0.8
    path, _status = run_segment(ca, (0, 20), 0.0, opts, 200)

    engagements = [_engagement_at(p, ca, tool_radius) for p in path]

    n_bins = min(30, len(set(round(e, 6) for e in engagements)))
    fig, ax = plt.subplots(figsize=(8, 4))
    ax.hist(engagements, bins=max(n_bins, 5), color="steelblue", alpha=0.7)
    ax.axvline(
        target_eng,
        color="red",
        linestyle="--",
        linewidth=2,
        label=f"Target ({target_eng:.2f} rad)",
    )
    ax.set_xlabel("Engagement angle (rad)")
    ax.set_ylabel("Count")
    ax.set_title(
        f"Engagement Distribution: σ = {np.std(engagements):.3f} rad, "
        f"mean = {np.mean(engagements):.3f} rad"
    )
    ax.legend(fontsize=9)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.cut.stepper.md"]
__images__ = [
    {
        "heading": "run_segment",
        "caption": (
            "Wall following along four boundary shapes: "
            "curved, square wave, zig zag, and circle."
        ),
        "function": generate_wall_following,
    },
    {
        "heading": "run_segment",
        "caption": (
            "Wall following using area engagement "
            "(same shapes as angular version)."
        ),
        "function": generate_wall_following_area,
    },
    {
        "heading": "step",
        "caption": (
            "90° corner: the solver deflects the heading to keep "
            "engagement constant around the turn."
        ),
        "function": generate_pocket_corner,
    },
    {
        "heading": "step",
        "caption": (
            "Engagement histogram for 200 steps along a straight "
            "wall. Tight peak near target indicates stable behaviour."
        ),
        "function": generate_engagement_histogram,
    },
]
