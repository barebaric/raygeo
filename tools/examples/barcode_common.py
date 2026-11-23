"""Shared plotting helpers for the barcode documentation examples."""

import matplotlib.pyplot as plt
from matplotlib.patches import PathPatch
from matplotlib.path import Path

from raygeo.barcode.geometry import BarcodeStyle, to_geometry


def fill_symbol(axes, encoded, style, title, color="#1d1d1d"):
    """Draw an encoded symbol as a filled compound path."""
    geometry = to_geometry(encoded, style)
    polygons = geometry.to_polygons(0.01)

    vertices = []
    codes = []
    for polygon in polygons:
        if len(polygon) < 3:
            continue
        vertices.extend([*polygon, polygon[0]])
        codes.extend(
            [Path.MOVETO]
            + [Path.LINETO] * (len(polygon) - 1)
            + [Path.CLOSEPOLY]
        )

    path = Path(vertices, codes)
    axes.add_patch(
        PathPatch(path, facecolor=color, edgecolor="none", linewidth=0)
    )
    x0, y0, x1, y1 = geometry.rect()
    axes.set_xlim(x0 - 1.0, x1 + 1.0)
    axes.set_ylim(y0 - 1.0, y1 + 1.0)
    axes.set_aspect("equal")
    axes.axis("off")
    axes.set_title(title, fontsize=11)


def panel(figure, axes, encoded, title, module_mm=0.5):
    """Fill one panel with a symbol using a readable default style."""
    bar_height = 0.0 if len(encoded.modules) > 1 else 15.0
    style = BarcodeStyle(
        module_mm=module_mm,
        bar_height_mm=bar_height,
        show_text=bar_height > 0.0,
    )
    fill_symbol(axes, encoded, style, title)


def close(figure):
    figure.tight_layout()
    return figure


def new_figure(width, height):
    return plt.subplots(figsize=(width, height))
