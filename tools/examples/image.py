"""Generate image processing example images."""

__images__ = [
    {
        "stem": "image-processing-srgb",
        "caption": "sRGB to linear round-trip",
        "doc": "raygeo.image.md",
        "heading": "srgb_to_linear",
    },
    {
        "stem": "image-processing-dither-floyd",
        "caption": "Floyd-Steinberg dithering",
        "doc": "raygeo.image.md",
        "heading": "apply_floyd_steinberg_dither",
    },
    {
        "stem": "image-processing-dither-bayer",
        "caption": "Bayer 4x4 ordered dithering",
        "doc": "raygeo.image.md",
        "heading": "apply_bayer_dither",
    },
]

import matplotlib.pyplot as plt
import numpy as np

import raygeo.image as img
from tools.plot import make_pattern


def _plot_dither(gray, dithered, title):
    fig, axes = plt.subplots(1, 2, figsize=(10, 4))
    axes[0].imshow(gray, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original")
    axes[1].imshow(dithered, cmap="gray", vmin=0, vmax=1)
    axes[1].set_title(title)
    fig.tight_layout()
    return fig


def generate_examples(output_dir):
    images = []
    w, h = 128, 128
    arr = make_pattern(w, h, "Gradient")

    fig, axes = plt.subplots(1, 2, figsize=(10, 4))
    axes[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original (uint8)")
    linear = img.srgb_to_linear(arr)
    back = img.linear_to_srgb(linear)
    axes[1].imshow(back, cmap="gray", vmin=0, vmax=255)
    axes[1].set_title("Round-trip (sRGB -> linear -> sRGB)")

    fig.tight_layout()
    path = output_dir / "image-processing-srgb.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "image-processing-srgb.png",
            "caption": "sRGB to linear round-trip",
        }
    )

    gray = img.normalize_grayscale(arr).astype(np.uint8)

    dithered_fs = img.apply_floyd_steinberg_dither(gray, False)
    fig2 = _plot_dither(gray, dithered_fs, "Floyd-Steinberg")
    path2 = output_dir / "image-processing-dither-floyd.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "image-processing-dither-floyd.png",
            "caption": "Floyd-Steinberg dithering",
        }
    )

    bayer = np.array(
        [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
        dtype=np.float32,
    )
    dithered_bayer = img.apply_bayer_dither(gray, bayer, False, cell_size=1)
    fig3 = _plot_dither(gray, dithered_bayer, "Bayer 4x4")
    path3 = output_dir / "image-processing-dither-bayer.png"
    fig3.savefig(path3, dpi=150)
    plt.close(fig3)
    images.append(
        {
            "path": "image-processing-dither-bayer.png",
            "caption": "Bayer 4x4 ordered dithering",
        }
    )

    return {
        "title": "Image Processing",
        "description": (
            "Image processing features including sRGB/linear conversion "
            "and dithering."
        ),
        "images": images,
    }
