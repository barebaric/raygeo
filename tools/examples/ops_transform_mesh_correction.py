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


def _bed(xx, yy):
    return (
        AMPLITUDE
        * np.sin(2 * np.pi * xx / SIZE)
        * np.cos(2 * np.pi * yy / SIZE)
    )


def _bed_heights():
    line = np.arange(0.0, SIZE + GRID_STEP / 2, GRID_STEP)
    xx, yy = np.meshgrid(line, line)
    return _bed(xx, yy)


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


def _model_surface(samples=60):
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
    ops.mesh_correction(0.0, 0.0, GRID_STEP, GRID_STEP, _bed_heights())
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


def _plot_height_map(ax, model):
    mx, my, mz = model
    ax.plot_surface(mx, my, mz, cmap="viridis", alpha=0.85, linewidth=0)
    gx, gy = np.meshgrid(
        np.arange(0.0, SIZE + GRID_STEP / 2, GRID_STEP),
        np.arange(0.0, SIZE + GRID_STEP / 2, GRID_STEP),
    )
    heights = _bed_heights()
    ax.plot_wireframe(gx, gy, heights, color="gray", alpha=0.5, linewidth=0.8)
    ax.scatter(gx, gy, heights + 0.05, color="crimson", s=16)
    ax.set_zlim(-AMPLITUDE * 1.8, AMPLITUDE * 1.8)
    ax.set_title(
        f"Probed height map ({len(gx)}x{len(gx)} probes as dots, "
        "their control mesh as wireframe)",
        fontsize=11,
    )


def generate_mesh_correction_example():
    """Warp a flat toolpath onto a probed bed height map."""
    original = _build_toolpath()
    corrected = original.copy()
    corrected.mesh_correction(0.0, 0.0, GRID_STEP, GRID_STEP, _bed_heights())
    model = _model_surface()

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
    _plot_height_map(ax3, model)

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
]
