"""Visualise engagement-angle computation for adaptive clearing."""

import math

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Circle

from raygeo.geo.algo.engagement import compute_engagement


def generate_engagement_vs_distance():
    """Engagement angle, area, and chord depth vs signed distance."""
    radius = 5.0
    ds = np.linspace(-radius, radius, 200)

    angles = []
    areas = []
    depths = []
    for d in ds:
        angle, area, depth = compute_engagement(d, radius)
        angles.append(angle)
        areas.append(area)
        depths.append(depth)

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    # Left: angle and area vs signed distance
    ax1.plot(ds, angles, "b-", linewidth=2, label="Engagement angle θ (rad)")
    ax1_twin = ax1.twinx()
    ax1_twin.plot(ds, areas, "r--", linewidth=2, label="Est. area (mm²)")
    ax1.axvline(0, color="gray", linestyle="--", alpha=0.5)
    ax1.axhline(math.pi, color="gray", linestyle=":", alpha=0.3)
    ax1.set_xlabel(
        "Signed distance d (mm)  [−R = inside cleared, +R = outside]"
    )
    ax1.set_ylabel("Angle (rad)", color="b")
    ax1_twin.set_ylabel("Area (mm²)", color="r")
    ax1.set_title("Engagement vs. Signed Distance from Boundary")
    ax1.grid(True, alpha=0.3)

    # Right: chord depth
    ax2.plot(ds, depths, "g-", linewidth=2)
    ax2.axvline(0, color="gray", linestyle="--", alpha=0.5)
    ax2.set_xlabel("Signed distance d (mm)")
    ax2.set_ylabel("Chord depth (mm)")
    ax2.set_title("Chord Depth (max depth of cut)")
    ax2.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_circle_boundary():
    """Circle at several distances from boundary, showing contact arc."""
    radius = 5.0
    distances = [-radius * 0.5, 0.0, radius * 0.5]

    fig, axes = plt.subplots(1, 3, figsize=(15, 5))

    for ax, d in zip(axes, distances):
        angle, _, _ = compute_engagement(d, radius)

        # Draw the circle.
        circle = Circle(
            (0, 0), radius, fill=False, edgecolor="steelblue", linewidth=2
        )
        ax.add_patch(circle)
        ax.plot(0, 0, "k.", markersize=4)

        # Draw the boundary (vertical line at x = d).
        ax.axvline(d, color="red", linewidth=2, label="Boundary")

        # Shade the contact arc (the portion beyond the boundary).
        if angle > 0 and angle < 2 * math.pi - 1e-12:
            half = angle / 2
            theta = np.linspace(-half, half, 50)
            arc_x = radius * np.cos(theta)
            arc_y = radius * np.sin(theta)
            ax.fill(
                arc_x, arc_y, "tomato", alpha=0.4, label=f"θ = {angle:.2f} rad"
            )

        ax.set_xlim(-radius * 1.5, radius * 1.5)
        ax.set_ylim(-radius * 1.5, radius * 1.5)
        ax.set_aspect("equal")
        ax.set_title(f"d = {d:.1f} mm, θ = {angle:.2f} rad")
        ax.legend(fontsize=7)
        ax.grid(True, alpha=0.3)

    fig.suptitle("Circle Engagement at Different Depths", fontsize=13)
    fig.tight_layout()
    return fig


def generate_engagement_heatmap():
    """Heatmap of engagement angle over a pocket with an island."""
    np.random.seed(42)
    n = 50
    xs = np.linspace(-10, 10, n)
    ys = np.linspace(-10, 10, n)
    radius = 2.0

    # Simulate a pocket with a circular cleared area in the centre.
    angles = np.zeros((n, n))
    for i, x in enumerate(xs):
        for j, y in enumerate(ys):
            d_to_centre = math.hypot(x, y)
            if d_to_centre < 3.0:
                # Inside cleared area
                d = -(3.0 - d_to_centre)
            else:
                d = d_to_centre - 3.0
            angle, _, _ = compute_engagement(d, radius)
            angles[j, i] = angle

    fig, ax = plt.subplots(figsize=(8, 7))
    im = ax.pcolormesh(xs, ys, angles, shading="auto", cmap="RdYlGn")
    cbar = fig.colorbar(im, ax=ax)
    cbar.set_label("Engagement angle (rad)")

    # Mark the cleared area boundary.
    theta = np.linspace(0, 2 * math.pi, 100)
    ax.plot(
        3.0 * np.cos(theta),
        3.0 * np.sin(theta),
        "k--",
        linewidth=1.5,
        label="Cleared boundary",
    )

    ax.set_aspect("equal")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("Engagement Heatmap: Circular Cleared Area")
    ax.legend(fontsize=8)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.engagement.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "Engagement angle, area, and chord depth as a function of"
            " signed distance from the cleared boundary."
        ),
        "function": generate_engagement_vs_distance,
    },
    {
        "heading": "compute_engagement",
        "caption": (
            "Circle at three signed distances from the boundary."
            " Shaded red arc is the contact arc (engagement)."
        ),
        "function": generate_circle_boundary,
    },
    {
        "heading": "compute_engagement",
        "caption": (
            "Engagement heatmap around a circular cleared area."
            " Green = low, red = high engagement."
        ),
        "function": generate_engagement_heatmap,
    },
]
