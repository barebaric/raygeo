"""Example for raygeo.image.pack sub-module."""

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Patch

from raygeo.image.pack import Packer


def _u_shape() -> np.ndarray:
    mask = np.zeros((8, 8), dtype=bool)
    mask[0:7, 0:3] = True
    mask[0:7, 5:8] = True
    mask[5:7, 0:8] = True
    return mask


def _disc() -> np.ndarray:
    yy, xx = np.ogrid[-5:6, -5:6]
    return xx**2 + yy**2 <= 25


def generate_packing():
    """Pack footprints."""
    width, height = 64, 26
    canvas = np.zeros((height, width), dtype=np.uint8)
    canvas[2:7, 40:54] = 1
    canvas[18:23, 6:14] = 1

    l_shape = np.zeros((10, 9), dtype=bool)
    l_shape[:, :4] = True
    l_shape[6:, :] = True

    items = [
        ("L-bracket", [l_shape, np.rot90(l_shape)]),
        ("plate", [np.ones((8, 12), dtype=bool)]),
        (
            "rail",
            [
                np.ones((4, 18), dtype=bool),
                np.ones((18, 4), dtype=bool),
            ],
        ),
        ("spacer", [np.ones((6, 6), dtype=bool)]),
        ("U-bracket", [_u_shape(), np.rot90(_u_shape())]),
        ("disc", [_disc()]),
        (
            "bar",
            [
                np.ones((3, 20), dtype=bool),
                np.ones((20, 3), dtype=bool),
            ],
        ),
        ("pad", [np.ones((5, 7), dtype=bool)]),
        (
            "clip",
            [
                np.ones((4, 10), dtype=bool),
                np.ones((10, 4), dtype=bool),
            ],
        ),
        ("tab", [np.ones((3, 6), dtype=bool)]),
    ]

    packer = Packer(canvas)
    colormap = plt.get_cmap("tab10")
    image = np.ones((height, width, 3), dtype=float)
    image[canvas != 0] = (0.85, 0.85, 0.85)

    legend = []
    placed = 0
    for index, (name, variants) in enumerate(items):
        result = packer.place([variant.view(np.uint8) for variant in variants])
        if result is None:
            continue
        variant_index, y, x = result
        mask = variants[variant_index]
        color = colormap(index % 10)[:3]
        region = image[y : y + mask.shape[0], x : x + mask.shape[1]]
        region[mask] = color
        placed += 1
        legend.append(
            Patch(
                facecolor=color,
                edgecolor="none",
                label=f"{placed}. {name}",
            )
        )

    fig, ax = plt.subplots(figsize=(9, 4))
    ax.imshow(image, extent=(0, width, height, 0))
    ax.set_title("Greedy first-fit packing with variant selection")
    ax.axis("off")
    legend.append(Patch(facecolor=(0.85, 0.85, 0.85), label="blocked"))
    ax.legend(
        handles=legend,
        loc="upper left",
        bbox_to_anchor=(1.01, 1.0),
        frameon=False,
        fontsize=8,
    )
    plt.tight_layout()
    return fig


__docs_target__ = ["raygeo.image.pack.md"]
__images__ = [
    {
        "heading": None,
        "caption": "Greedy first-fit packing with variant selection",
        "function": generate_packing,
    },
]
