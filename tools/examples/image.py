"""Generate image processing example images."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.figure import Figure

from raygeo.geo import Geometry, Matrix
from raygeo.geo.shape.text import FontConfig, text_to_geometry
from raygeo.image import rasterize_scanlines
from raygeo.image.render import geometry_to_image
from raygeo.image.scan import ScanMode
from raygeo.ops import Ops
from raygeo.ops.types import CommandType
from tools.plot import make_pattern


class _PixelFigure(Figure):
    """Figure subclass whose savefig saves raw pixel buffer as PNG."""

    def __init__(self, data: np.ndarray):
        super().__init__()
        self._pixel_data = data

    def savefig(self, fname, **kw):
        plt.imsave(fname, self._pixel_data[::-1])


def generate_rasterize_scanlines():
    """Rasterize scanlines."""
    img_size = 64
    ppm = 10.0

    gray3 = make_pattern(img_size, img_size, "Radial")
    alpha = np.full((img_size, img_size), 255, dtype=np.uint8)

    ops = Ops.from_power_modulated_image(
        gray3,
        alpha,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval_mm=0.1,
        sample_interval_mm=0.05,
        min_power=0.0,
        max_power=1.0,
        angle=0,
        scan_mode=ScanMode.SEGMENTED,
    )

    rasterized = rasterize_scanlines(
        ops, img_size, img_size, (ppm, ppm), (0.0, 0.0)
    ).to_numpy()

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))
    max_mm = img_size / ppm

    ops.preload_state()
    power_cmap = plt.get_cmap("plasma")
    pos = (0.0, 0.0, 0.0)
    for i in range(len(ops)):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            n = max(1, len(ops.scanline_data(i)))
            xs = np.linspace(pos[0], ep[0], n)
            ys = np.linspace(pos[1], ep[1], n)
            pwr = np.frombuffer(ops.scanline_data(i), dtype=np.uint8).astype(
                np.float64
            )
            pn = pwr / 255.0
            for j in range(n - 1):
                axes[0].plot(
                    xs[j : j + 2],
                    ys[j : j + 2],
                    color=power_cmap(pn[j] if j < len(pn) else 0),
                    linewidth=1.2,
                )
            pos = ep

    axes[0].set_xlim(0, max_mm)
    axes[0].set_ylim(0, max_mm)
    axes[0].set_aspect("equal")
    axes[0].grid(True, alpha=0.3)
    axes[0].set_title("Input: Ops scanlines (power-colored)")

    axes[1].imshow(
        rasterized,
        cmap="plasma",
        vmin=0,
        vmax=255,
        origin="lower",
        extent=[0, max_mm, 0, max_mm],
    )
    axes[1].set_title("Output: rasterized power-map buffer")
    axes[1].set_xlim(0, max_mm)
    axes[1].set_ylim(0, max_mm)
    axes[1].set_aspect("equal")

    plt.tight_layout()
    return fig


def generate_geometry_to_image():
    """Rasterise vector geometry (strokes + fills) into an RGBA image.

    Demonstrates ``geometry_to_image`` by creating a grid of filled
    rectangles with a stroke border, placing text labels, and returning
    the rendered pixel buffer as a matplotlib figure.
    """
    mm = 80.0, 70.0
    dpi = 300

    strokes = Geometry()
    fills = Geometry()

    # 3x3 grid of filled squares with stroke outlines, symmetrically centred
    cell = 14.0
    gap = 2.0
    grid = 3.0 * cell + 2.0 * gap  # 46 mm
    mx = (mm[0] - grid) / 2.0  # 17 mm horizontal margin
    # Y-up: text near bottom (y=6), grid above it (y=18 ... 64)
    grid_y0 = 18.0
    for row in range(3):
        for col in range(3):
            x = mx + col * (cell + gap)
            y = grid_y0 + row * (cell + gap)
            fills.move_to(x, y, 0.0)
            fills.line_to(x + cell, y, 0.0)
            fills.line_to(x + cell, y + cell, 0.0)
            fills.line_to(x, y + cell, 0.0)
            fills.close_path()
            strokes.move_to(x, y, 0.0)
            strokes.line_to(x + cell, y, 0.0)
            strokes.line_to(x + cell, y + cell, 0.0)
            strokes.line_to(x, y + cell, 0.0)
            strokes.line_to(x, y, 0.0)

    # Text label centred below the grid
    font = FontConfig("sans-serif", 14.0)
    label = text_to_geometry("Raygeo", font)
    if label is not None and not label.is_empty():
        label.transform(Matrix.scale(1, -1))
        rect = label.rect()
        lw = rect[2] - rect[0]
        label.transform(
            Matrix.translation(
                mm[0] / 2 - lw / 2,
                8.0,
            )
        )
        strokes.extend(label)

    rendered = geometry_to_image(strokes, fills, mm, dpi=dpi)

    # Return a Figure subclass whose savefig saves the raw pixel buffer as
    # PNG -- avoids the resampling that imshow always introduces when the
    # Figure's axes don't exactly match pixel count.
    # geometry_to_image returns Y-down (row 0 = top).  Flip to Y-up for
    # the doc image (lower-left = origin).
    return _PixelFigure(rendered)


__docs_target__ = ["raygeo.image.md"]
__images__ = [
    {
        "heading": "rasterize_scanlines",
        "caption": "Scanline ops rasterized into a 2D power-map buffer",
        "function": generate_rasterize_scanlines,
    },
]
