"""Generate text-to-geometry example images."""

import matplotlib.pyplot as plt

from raygeo.geo.shape.text import FontConfig, text_to_geometry
from tools.plot import plot_geometry


def generate_text_to_geometry():
    """Glyph outlines for a text string."""
    text = "Hello"
    cfg = FontConfig("sans-serif", 48.0, bold=False, italic=False)
    geo = text_to_geometry(text, cfg)
    assert geo is not None

    fig, ax = plt.subplots(figsize=(10, 3))
    ax.set_aspect("equal")
    ax.set_title(f"Glyph outlines: {text!r}", fontsize=14)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")
    plot_geometry(
        ax,
        geo,
        color="steelblue",
        linewidth=1.5,
        show_points=True,
        label="Glyph",
    )
    ax.legend(loc="upper right")
    fig.tight_layout()
    return fig


def generate_get_text_width():
    """Text advance width and cursor position."""
    text = "RayGeo"
    cfg = FontConfig("sans-serif", 36.0)
    geo = text_to_geometry(text, cfg)
    assert geo is not None

    width = cfg.get_text_width(text)
    pos_2 = cfg.get_text_position(text, 2)

    fig, ax = plt.subplots(figsize=(10, 2.8))
    ax.set_aspect("equal")
    ax.set_title(f"Text width and cursor position: {text!r}", fontsize=14)
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")

    plot_geometry(ax, geo, color="steelblue", linewidth=1.5, label="Glyph")

    ax.axvline(
        pos_2,
        color="orange",
        linestyle="-",
        linewidth=1.2,
        label=f"pos @ idx 2 ({pos_2:.1f})",
    )
    ax.axvline(
        width,
        color="purple",
        linestyle="--",
        linewidth=1.0,
        label=f"width ({width:.1f})",
    )

    ax.legend(loc="upper right", fontsize=9)
    fig.tight_layout()
    return fig


def generate_get_font_metrics():
    """Font metrics showing ascent, descent, and height."""
    text = "ABC abc Î|"
    cfg = FontConfig("sans-serif", 48.0)
    geo = text_to_geometry(text, cfg)
    assert geo is not None

    ascent, descent, height = cfg.get_font_metrics()

    fig, ax = plt.subplots(figsize=(8, 4))
    ax.set_aspect("equal")
    ax.set_title(
        f"Font metrics: ascent={ascent:.1f}, "
        f"descent={descent:.1f}, height={height:.1f}",
        fontsize=14,
    )
    ax.set_xlabel("X (mm)")
    ax.set_ylabel("Y (mm)")

    plot_geometry(ax, geo, color="steelblue", linewidth=1.5, label="Glyph")

    ax.axhline(
        0, color="gray", linestyle="--", linewidth=0.8, label="Baseline"
    )
    ax.axhline(
        ascent,
        color="tomato",
        linestyle=":",
        linewidth=0.8,
        label=f"Ascent ({ascent:.1f})",
    )
    ax.axhline(
        descent,
        color="seagreen",
        linestyle=":",
        linewidth=0.8,
        label=f"Descent ({descent:.1f})",
    )

    ax.set_ylim(descent * 1.15, ascent * 1.15)
    ax.legend(loc="lower right", fontsize=9)
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.shape.text.md"]
__images__ = [
    {
        "heading": "text_to_geometry",
        "caption": "Glyph outlines rendered as vector geometry",
        "function": generate_text_to_geometry,
    },
    {
        "heading": "get_text_width",
        "caption": "Text advance width and cursor position markers",
        "function": generate_get_text_width,
    },
    {
        "heading": "get_font_metrics",
        "caption": "Ascent, descent, and height above the baseline",
        "function": generate_get_font_metrics,
    },
]
