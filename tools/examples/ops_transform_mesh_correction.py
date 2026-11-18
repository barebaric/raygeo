"""Generate mesh_correction example images."""

import math

import matplotlib.pyplot as plt
import numpy as np

from raygeo.ops import Ops
from tools.plot import plot_ops_3d

SIZE = 100.0
AMPLITUDE = 1.2
GRID_STEP = 25.0
SEGMENT = 4.0
HILL_AMPLITUDE = 2.0
HILL_SIGMA = 32.0
HILL_ZLIM = HILL_AMPLITUDE * 1.3


def _bed(xx, yy):
    return (
        AMPLITUDE
        * np.sin(2 * np.pi * xx / SIZE)
        * np.cos(2 * np.pi * yy / SIZE)
    )


def _hill(xx, yy):
    return HILL_AMPLITUDE * np.exp(
        -(((xx - SIZE / 2) ** 2 + (yy - SIZE / 2) ** 2) / (2 * HILL_SIGMA**2))
    )


def _probe_grid():
    gx, gy = np.meshgrid(
        np.arange(0.0, SIZE + GRID_STEP / 2, GRID_STEP),
        np.arange(0.0, SIZE + GRID_STEP / 2, GRID_STEP),
    )
    return gx, gy


def _bed_heights():
    return _bed(*_probe_grid())


def _hill_heights():
    return _hill(*_probe_grid())


def _build_toolpath():
    """Circle job with raster fill, pre-flattened to short segments.

    Importers deliver geometry in this form (curves are linearized
    before Ops are built), and the short segments let the plot
    resolve the Z warp the correction applies along every move.
    """
    ops = Ops()
    ops.set_feed_rate(3000)

    def circle(cx, cy, radius):
        n = max(12, int(2 * math.pi * radius / SEGMENT))
        for k in range(n + 1):
            a = 2 * math.pi * k / n
            x = cx + radius * math.cos(a)
            y = cy + radius * math.sin(a)
            if k == 0:
                ops.move_to(x, y, 0)
            else:
                ops.line_to(x, y, 0)

    radius = 36.0
    circle(50.0, 50.0, radius)
    y = 18.0
    row = 0
    while y <= 82.0:
        half = math.sqrt(radius**2 - (y - 50.0) ** 2) - 3.0
        if half > 2.0:
            n = max(2, int(math.ceil(2 * half / SEGMENT)) + 1)
            xs = np.linspace(50.0 - half, 50.0 + half, n)
            if row % 2:
                xs = xs[::-1]
            ops.move_to(float(xs[0]), y, 0)
            for x in xs[1:]:
                ops.scan_to(float(x), y, 0, power_values=[220] * 2)
        y += 4.0
        row += 1
    circle(50.0, 50.0, 8.0)
    return ops


def _build_split_toolpath():
    """Two long diagonal cuts across the probe grid.

    Each runs straight between its corrected ends, so without
    subdivision it misses the hill in the middle of the bed.
    """
    ops = Ops()
    ops.set_feed_rate(3000)
    ops.move_to(5.0, 5.0, 0.0)
    ops.line_to(95.0, 95.0, 0.0)
    ops.move_to(5.0, 20.0, 0.0)
    ops.line_to(95.0, 80.0, 0.0)
    return ops


def _model_surface(heights, samples=60):
    """The surface the machine actually applies: the bilinear
    interpolation of the probe grid, evaluated by running
    mesh_correction on a fine inspection grid of survey lines."""
    line = np.linspace(0.0, SIZE, samples)
    xx, yy = np.meshgrid(line, line)
    ops = Ops()
    for j in range(samples):
        ops.move_to(float(xx[j, 0]), float(yy[j, 0]), 0)
        for i in range(1, samples):
            ops.line_to(float(xx[j, i]), float(yy[j, i]), 0)
    ops.mesh_correction(0.0, 0.0, GRID_STEP, GRID_STEP, heights)
    z = np.empty((samples, samples))
    for j in range(samples):
        for i in range(samples):
            z[j, i] = ops.endpoint(j * samples + i)[2]
    return xx, yy, z


def _plot_path(ax, ops, title, model):
    plot_ops_3d(ax, ops, mark_cut_start=False)
    xx, yy, z = model
    ax.plot_surface(xx, yy, z, color="slategray", alpha=0.25, linewidth=0)
    ax.set_xlim(-5.0, SIZE + 5.0)
    ax.set_ylim(-5.0, SIZE + 5.0)
    ax.set_zlim(-AMPLITUDE * 1.8, AMPLITUDE * 1.8)
    ax.set_title(title, fontsize=11)


def _plot_probe_grid(ax, heights):
    gx, gy = _probe_grid()
    ax.plot_wireframe(gx, gy, heights, color="gray", alpha=0.5, linewidth=0.7)


def _move_count(ops):
    return sum(
        1 for i in range(ops.len()) if ops.is_cutting(i) or ops.is_travel(i)
    )


def _plot_cutting_moves(ax, ops):
    prev = None
    for i in range(ops.len()):
        end = ops.endpoint(i)
        if ops.is_cutting(i) and prev is not None:
            ax.plot(
                [prev[0], end[0]],
                [prev[1], end[1]],
                [prev[2], end[2]],
                color="#111111",
                linewidth=2.0,
                alpha=0.9,
            )
        prev = end


def _mark_command_ends(ax, ops):
    xs, ys, zs = [], [], []
    for i in range(ops.len()):
        if not ops.is_cutting(i):
            continue
        x, y, z = ops.endpoint(i)
        xs.append(x)
        ys.append(y)
        zs.append(z)
    ax.scatter(xs, ys, zs, color="crimson", s=18, depthshade=False)


def _plot_height_map(ax, model, heights, zlim=AMPLITUDE * 1.8):
    mx, my, mz = model
    ax.plot_surface(mx, my, mz, cmap="viridis", alpha=0.85, linewidth=0)
    gx, gy = _probe_grid()
    ax.plot_wireframe(gx, gy, heights, color="gray", alpha=0.5, linewidth=0.8)
    ax.scatter(gx, gy, heights + 0.05, color="crimson", s=16)
    ax.set_zlim(-zlim, zlim)
    ax.set_title(
        f"Probed height map ({len(gx)}x{len(gx)} probes as dots, "
        "their control mesh as wireframe)",
        fontsize=11,
    )


def _plot_split_path(ax, ops, title, model, heights, zlim):
    _plot_path(ax, ops, title, model)
    _plot_probe_grid(ax, heights)
    _plot_cutting_moves(ax, ops)
    _mark_command_ends(ax, ops)
    ax.set_zlim(-zlim, zlim)
    ax.view_init(elev=20, azim=-55)


def generate_mesh_correction_example():
    """Warp a flat toolpath onto a probed bed height map."""
    heights = _bed_heights()
    original = _build_toolpath()
    corrected = original.copy()
    corrected.mesh_correction(0.0, 0.0, GRID_STEP, GRID_STEP, heights)
    model = _model_surface(heights)

    fig = plt.figure(figsize=(22, 6.5))
    ax1 = fig.add_subplot(131, projection="3d")
    ax2 = fig.add_subplot(132, projection="3d")
    ax3 = fig.add_subplot(133, projection="3d")

    _plot_path(
        ax1, original, "Before: flat toolpath, wavy bed underneath", model
    )
    _plot_path(
        ax2,
        corrected,
        "After mesh_correction: path rides the height map",
        model,
    )
    _plot_height_map(ax3, model, heights)

    fig.tight_layout()
    return fig


def generate_mesh_correction_subdivide_example():
    """Show how subdivide makes long cuts follow the height map."""
    heights = _hill_heights()
    model = _model_surface(heights)
    original = _build_split_toolpath()

    plain = original.copy()
    plain.mesh_correction(0.0, 0.0, GRID_STEP, GRID_STEP, heights)

    split = original.copy()
    split.mesh_correction(
        0.0, 0.0, GRID_STEP, GRID_STEP, heights, subdivide=True
    )

    fig = plt.figure(figsize=(22, 6.5))
    ax1 = fig.add_subplot(131, projection="3d")
    ax2 = fig.add_subplot(132, projection="3d")
    ax3 = fig.add_subplot(133, projection="3d")

    _plot_height_map(ax1, model, heights, zlim=HILL_ZLIM)
    _plot_split_path(
        ax2,
        plain,
        f"subdivide=False: {_move_count(plain)} moves; each cut runs "
        "straight between its corrected ends",
        model,
        heights,
        HILL_ZLIM,
    )
    _plot_split_path(
        ax3,
        split,
        f"subdivide=True: {_move_count(split)} moves; each cut is "
        "split where it crosses a grid line",
        model,
        heights,
        HILL_ZLIM,
    )

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "mesh_correction",
        "caption": (
            "A flat toolpath warped onto a bed height map so the "
            "focal point tracks the wavy surface"
        ),
        "function": generate_mesh_correction_example,
    },
    {
        "heading": "mesh_correction",
        "caption": (
            "Splitting long cuts at grid lines makes Z follow the "
            "hill that a straight cut misses"
        ),
        "function": generate_mesh_correction_subdivide_example,
    },
]
