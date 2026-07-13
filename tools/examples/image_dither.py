"""Example for raygeo.image.dither sub-module."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.image.dither import (
    apply_bayer_dither,
    apply_floyd_steinberg_dither,
    apply_minimum_run_length,
)
from raygeo.image.grayscale import normalize_grayscale
from tools.plot import make_pattern


def _plot_dither(gray, dithered, title):
    fig, axes = plt.subplots(1, 2, figsize=(10, 4))
    axes[0].imshow(gray, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original")
    axes[1].imshow(dithered, cmap="gray", vmin=0, vmax=1)
    axes[1].set_title(title)
    plt.tight_layout()
    return fig


def generate_dither_floyd():
    """Floyd dither."""
    w, h = 128, 128
    arr = make_pattern(w, h, "Gradient")
    gray = normalize_grayscale(arr).astype(np.uint8)

    dithered_fs = apply_floyd_steinberg_dither(gray, False)
    return _plot_dither(gray, dithered_fs, "Floyd-Steinberg")


def generate_dither_bayer():
    """Bayer dither."""
    w, h = 128, 128
    arr = make_pattern(w, h, "Gradient")
    gray = normalize_grayscale(arr).astype(np.uint8)

    bayer = np.array(
        [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
        dtype=np.float32,
    )
    dithered_bayer = apply_bayer_dither(gray, bayer, False, cell_size=1)
    return _plot_dither(gray, dithered_bayer, "Bayer 4x4")


def generate_min_run_len():
    """Min run length."""
    h2, w2 = 5, 20

    binary5 = np.zeros((h2, w2), dtype=np.uint8)
    binary5[0, 1:3] = 1
    binary5[0, 5:12] = 1
    binary5[0, 15:17] = 1
    binary5[1, 0:8] = 1
    binary5[1, 10:19] = 1
    binary5[2, 2:4] = 1
    binary5[2, 6:15] = 1
    binary5[2, 17:19] = 1
    binary5[3, 3:7] = 1
    binary5[3, 9:12] = 1
    binary5[3, 14:16] = 1
    binary5[4, 0:18] = 1

    result = apply_minimum_run_length(binary5.copy(), min_run_length=4)

    fig, axes = plt.subplots(1, 2, figsize=(8, 3))
    axes[0].imshow(binary5, cmap="gray", vmin=0, vmax=1, aspect="auto")
    axes[0].set_title(f"Before ({w2} px wide)")
    axes[0].set_yticks(range(h2))
    axes[0].set_xticks(range(w2))
    axes[1].imshow(result, cmap="gray", vmin=0, vmax=1, aspect="auto")
    axes[1].set_title("After (min_run_length=4)")
    axes[1].set_yticks(range(h2))
    axes[1].set_xticks(range(w2))

    plt.tight_layout()
    return fig


__docs_target__ = ["raygeo.image.dither.md"]
__images__ = [
    {
        "heading": "apply_floyd_steinberg_dither",
        "caption": "Floyd-Steinberg dithering",
        "function": generate_dither_floyd,
    },
    {
        "heading": "apply_bayer_dither",
        "caption": "Bayer 4x4 ordered dithering",
        "function": generate_dither_bayer,
    },
    {
        "heading": "apply_minimum_run_length",
        "caption": "Minimum run length applied to binary image",
        "function": generate_min_run_len,
    },
]
