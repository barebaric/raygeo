"""Example for raygeo.image.srgb sub-module."""

import matplotlib.pyplot as plt

from raygeo.image.srgb import linear_to_srgb, srgb_to_linear
from tools.plot import make_pattern


def generate_srgb():
    """sRGB."""
    w, h = 128, 128
    arr = make_pattern(w, h, "Gradient")

    fig, axes = plt.subplots(1, 2, figsize=(10, 4))
    axes[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original (uint8)")
    linear = srgb_to_linear(arr)
    back = linear_to_srgb(linear)
    axes[1].imshow(back, cmap="gray", vmin=0, vmax=255)
    axes[1].set_title("Round-trip (sRGB -> linear -> sRGB)")

    plt.tight_layout()
    return fig


__docs_target__ = ["raygeo.image.srgb.md"]
__images__ = [
    {
        "heading": "srgb_to_linear",
        "caption": "sRGB to linear round-trip",
        "function": generate_srgb,
    },
]
