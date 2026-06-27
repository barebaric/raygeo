"""Visualise the interpolation helpers."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.ops.cut.interp import Interpolation, point_in_valid_area, rotate


def generate_interpolation_bracket():
    """Interpolation bracket: error vs steering angle with min/max samples."""

    def error_fn(a):
        return 0.4 * a**3 + 0.05 * a

    interp = Interpolation()
    min_a = -0.50
    max_a = 0.40
    min_e = error_fn(min_a)
    max_e = error_fn(max_a)
    interp.add(
        error=min_e,
        angle=min_a,
        pos=(0.0, 0.0),
        allow_skip=False,
        is_conventional=False,
    )
    interp.add(
        error=max_e,
        angle=max_a,
        pos=(1.0, 0.0),
        allow_skip=False,
        is_conventional=False,
    )

    fine_angles = np.linspace(-0.7, 0.7, 200)
    curve = error_fn(fine_angles)

    fig, ax = plt.subplots(figsize=(8, 5))

    ax.plot(fine_angles, curve, "steelblue", linewidth=2, label="Error(angle)")
    ax.axhline(0.0, color="gray", linewidth=0.8, linestyle="--")

    ax.scatter(
        [min_a],
        [min_e],
        color="red",
        s=100,
        zorder=5,
        label="min sample",
    )
    ax.scatter(
        [max_a],
        [max_e],
        color="green",
        s=100,
        zorder=5,
        label="max sample",
    )

    ax.annotate(
        f"min  (θ={min_a:.2f}, e={min_e:.3f})",
        (min_a, min_e),
        textcoords="offset points",
        xytext=(10, -15),
        fontsize=9,
        color="red",
    )
    ax.annotate(
        f"max  (θ={max_a:.2f}, e={max_e:.3f})",
        (max_a, max_e),
        textcoords="offset points",
        xytext=(10, 5),
        fontsize=9,
        color="green",
    )

    zero_angle = interp.interpolate()
    ax.axvline(
        zero_angle,
        color="purple",
        linewidth=1.5,
        linestyle=":",
        label=f"interpolated zero (θ={zero_angle:.3f})",
    )
    ax.scatter([zero_angle], [0.0], color="purple", s=80, zorder=5)

    ax.fill_between(
        [min_a, max_a],
        -0.15,
        0.15,
        color="gold",
        alpha=0.15,
        label="bracket range",
    )

    ax.set_xlabel("Steering angle (rad)")
    ax.set_ylabel("Error (cut-area per distance)")
    ax.set_title("Interpolation bracket around the root")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_point_in_valid_area():
    """Valid-area polygon with shell, hole, and points marked valid/invalid."""
    shell = [(0.0, 0.0), (8.0, 0.0), (8.0, 6.0), (0.0, 6.0)]
    # CW = hole
    hole = [(3.0, 1.5), (5.0, 1.5), (5.0, 4.5), (3.0, 4.5)]

    test_pts = [
        (1.0, 1.0, True),  # inside shell, outside hole → valid
        (4.0, 3.0, False),  # inside shell, inside hole → invalid
        (7.0, 5.0, True),  # inside shell, outside hole → valid
        (9.0, 3.0, False),  # outside shell → invalid
    ]

    fig, ax = plt.subplots(figsize=(7, 5))

    shell_arr = np.array(shell + [shell[0]])
    ax.fill(
        shell_arr[:, 0],
        shell_arr[:, 1],
        "lightblue",
        alpha=0.3,
        label="Shell (CCW)",
    )
    ax.plot(shell_arr[:, 0], shell_arr[:, 1], "steelblue", linewidth=2)

    hole_arr = np.array(hole + [hole[0]])
    ax.fill(
        hole_arr[:, 0],
        hole_arr[:, 1],
        "white",
        alpha=0.8,
    )
    ax.plot(
        hole_arr[:, 0],
        hole_arr[:, 1],
        "crimson",
        linewidth=2,
        linestyle="--",
        label="Hole (CW)",
    )

    for px, py, valid in test_pts:
        actual = point_in_valid_area((px, py), [shell, hole])
        color = "limegreen" if actual else "red"
        marker = "o" if actual == valid else "x"
        ax.scatter(
            [px],
            [py],
            color=color,
            s=120,
            zorder=5,
            marker=marker,
            linewidths=2,
        )
        label = "valid" if actual else "invalid"
        ax.annotate(
            label,
            (px, py),
            textcoords="offset points",
            xytext=(8, 5),
            fontsize=8,
            color=color,
        )

    ax.set_xlim(-1, 10)
    ax.set_ylim(-1, 7.5)
    ax.set_aspect("equal")
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    ax.set_title("point_in_valid_area — shell with hole")
    ax.legend(fontsize=8)
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


def generate_rotate_demo():
    """Rotate a vector by various angles."""
    v = (1.0, 0.0)
    angles = [0.0, math.pi / 6, math.pi / 3, math.pi / 2, -math.pi / 4]
    colors = ["black", "red", "green", "blue", "orange"]

    fig, ax = plt.subplots(figsize=(6, 6))

    for angle, color in zip(angles, colors):
        rx, ry = rotate(v, angle)
        ax.arrow(
            0.0,
            0.0,
            rx,
            ry,
            head_width=0.08,
            head_length=0.08,
            fc=color,
            ec=color,
            alpha=0.7,
            label=f"θ={angle:.2f}  →  ({rx:.2f}, {ry:.2f})",
        )

    ax.set_xlim(-1.2, 1.2)
    ax.set_ylim(-1.2, 1.2)
    ax.set_aspect("equal")
    ax.axhline(0.0, color="gray", linewidth=0.5)
    ax.axvline(0.0, color="gray", linewidth=0.5)
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.set_title("rotate — 2D vector rotation")
    ax.legend(fontsize=7, loc="upper right")
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.cut.interp.md"]

__images__ = [
    {
        "heading": "interpolate",
        "caption": (
            "Interpolation bracket showing error vs steering angle."
            " The red and green markers are the min and max bracket"
            " samples; the purple dashed line marks the interpolated"
            " zero-crossing angle."
        ),
        "function": generate_interpolation_bracket,
    },
    {
        "heading": "point_in_valid_area",
        "caption": (
            "Valid-area polygon with a CCW shell (blue) and CW hole"
            " (red dashed). Points are marked green (valid) or red"
            " (invalid) based on"
            " :func:`~raygeo.ops.cut.interp.point_in_valid_area`."
        ),
        "function": generate_point_in_valid_area,
    },
    {
        "heading": "rotate",
        "caption": (
            "Rotation of a unit vector ``(1, 0)`` by various angles"
            " using :func:`~raygeo.ops.cut.interp.rotate`."
        ),
        "function": generate_rotate_demo,
    },
]
