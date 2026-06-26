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


def generate_wall_following():
    """Tool stepping along a curved wall, maintaining constant engagement."""
    tool_radius = 3.0
    step_len = 1.0
    target_eng = math.pi

    # Cleared area with a curved top edge (sine wave).
    n = 100
    poly = [(-100.0, -100.0), (100.0, -100.0), (100.0, 20.0)]
    for i in range(n + 1):
        x = 100.0 - 200.0 * i / n
        y = 20.0 + 2.0 * math.sin(x * math.pi / 40.0)
        poly.append((x, y))

    ca = ClearedArea(boundary=[])
    ca.cut([poly])

    opts = StepperOptions()
    opts.radius = tool_radius
    opts.step_length = step_len
    opts.target_engagement = target_eng
    opts.max_deflection = 0.8
    path, _status = run_segment(ca, (0, 20), 0.0, opts, 80)

    fig, (ax1, ax2) = plt.subplots(
        1, 2, figsize=(14, 5), gridspec_kw={"width_ratios": [3, 1]}
    )

    # ── Left: path overlaid on geometry ──
    engagements = [_engagement_at(p, ca, tool_radius) for p in path]
    max_eng = max(engagements) if engagements else 1.0
    norm_eng = [e / max_eng for e in engagements]

    for frag in ca.query_window((-110, -110, 110, 40)):
        fx = [p[0] for p in frag] + [frag[0][0]]
        fy = [p[1] for p in frag] + [frag[0][1]]
        ax1.fill(fx, fy, "steelblue", alpha=0.2)
        ax1.plot(fx, fy, "steelblue", linewidth=1, alpha=0.5)

    for i in range(len(path) - 1):
        seg_xs = [path[i][0], path[i + 1][0]]
        seg_ys = [path[i][1], path[i + 1][1]]
        c = plt.cm.RdYlGn(norm_eng[i])
        ax1.plot(seg_xs, seg_ys, color=c, linewidth=2)

    # Every 10th tool position.
    for i in range(0, len(path), 10):
        c = Circle(
            path[i],
            tool_radius,
            fill=False,
            edgecolor=plt.cm.RdYlGn(norm_eng[i]),
            linewidth=1,
            linestyle="--",
        )
        ax1.add_patch(c)

    ax1.set_aspect("equal")
    ax1.set_xlim(-10, 85)
    ax1.set_ylim(12, 32)
    ax1.set_xlabel("X (mm)")
    ax1.set_ylabel("Y (mm)")
    ax1.set_title(
        "Wall Following: Curved Boundary\n"
        "(path colour = engagement, green = on-target)"
    )
    ax1.grid(True, alpha=0.3)

    # ── Right: engagement histogram ──
    target_line_label = f"Target ({target_eng:.2f})"
    ax2.hist(engagements, bins=20, color="steelblue", alpha=0.7)
    ax2.axvline(
        target_eng,
        color="red",
        linestyle="--",
        linewidth=2,
        label=target_line_label,
    )
    ax2.set_xlabel("Engagement angle (rad)")
    ax2.set_ylabel("Count")
    mean_eng = np.mean(engagements)
    std_eng = np.std(engagements)
    ax2.set_title(f"Engagement  |  μ = {mean_eng:.3f}  σ = {std_eng:.3f}")
    ax2.legend(fontsize=8)
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


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
        "heading": None,
        "caption": (
            "Tool stepping parallel to a straight wall. "
            "Path colour = engagement (green = on target)."
        ),
        "function": generate_wall_following,
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
