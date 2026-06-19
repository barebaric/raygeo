"""Generate concave hull example images."""

__images__ = [
    {
        "stem": "concave-hull",
        "caption": "Concave vs convex hull",
        "doc": "raygeo.geo.algo.hull.md",
        "heading": "get_concave_hull",
    },
]

import matplotlib.pyplot as plt
import numpy as np

from raygeo.geo.algo import hull
from tools.plot import fill_rounded_rect, plot_geometry


def generate_examples(output_dir):
    images = []
    height = 200
    width = 200
    gravity = 0.5

    presets = [
        ("Two squares", _make_two_squares(height, width)),
        ("Hourglass", _make_hourglass(height, width)),
        ("L-shape", _make_lshape(height, width)),
        ("Three dots", _make_three_dots(height, width)),
    ]

    fig, axes = plt.subplots(2, 2, figsize=(12, 12))
    axes_flat = axes.flatten()

    for ax, (title, img) in zip(axes_flat, presets):
        convex_geo = hull.get_enclosing_hull(img)
        concave_geo = hull.get_concave_hull(img, gravity=gravity)

        ax.imshow(
            img,
            origin="upper",
            cmap="Blues",
            alpha=0.3,
            extent=(0, width, height, 0),
        )

        if convex_geo is not None:
            plot_geometry(
                ax, convex_geo, color="tomato", label="Convex", linewidth=1.5
            )
        if concave_geo is not None:
            plot_geometry(
                ax,
                concave_geo,
                color="forestgreen",
                label="Concave",
                linewidth=2,
            )

        ax.set_aspect("equal")
        ax.grid(True, alpha=0.3)
        ax.legend(fontsize=9)
        ax.set_title(title)

    fig.tight_layout()
    path = output_dir / "concave-hull.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "concave-hull.png",
            "caption": "Concave vs convex hull on various shapes",
        }
    )

    return {
        "title": "Concave Hull (Shrink-Wrap)",
        "description": (
            "Compute convex and concave hulls (shrink-wrap) from binary "
            "images, with per-component hull extraction."
        ),
        "images": images,
    }


def _make_two_squares(h, w):
    img = np.zeros((h, w), dtype=bool)
    img[30:70, 30:70] = True
    img[130:170, 130:170] = True
    return img


def _make_hourglass(h, w):
    img = np.zeros((h, w), dtype=bool)
    r = 8
    fill_rounded_rect(img, (60, 30), (140, 70), r)
    fill_rounded_rect(img, (80, 110), (120, 150), r)
    fill_rounded_rect(img, (60, 110), (140, 170), r)
    return img


def _make_lshape(h, w):
    img = np.zeros((h, w), dtype=bool)
    img[30:170, 30:70] = True
    img[30:100, 70:170] = True
    return img


def _make_three_dots(h, w):
    img = np.zeros((h, w), dtype=bool)
    for cy, cx in [(50, 50), (50, 150), (150, 100)]:
        yy, xx = np.ogrid[:h, :w]
        mask = (xx - cx) ** 2 + (yy - cy) ** 2 <= 400
        img[mask] = True
    return img
