"""Generate image processing example images."""

import matplotlib.pyplot as plt
import numpy as np

import raygeo.image as img
from tools.plot import make_pattern


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
    bayer = np.array(
        [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
        dtype=np.float32,
    )
    dithered_bayer = img.apply_bayer_dither(gray, bayer, False, cell_size=1)

    fig2, axes2 = plt.subplots(1, 3, figsize=(14, 4))
    axes2[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes2[0].set_title("Original")
    axes2[1].imshow(dithered_fs, cmap="gray", vmin=0, vmax=1)
    axes2[1].set_title("Floyd-Steinberg")
    axes2[2].imshow(dithered_bayer, cmap="gray", vmin=0, vmax=1)
    axes2[2].set_title("Bayer 4x4")

    fig2.tight_layout()
    path2 = output_dir / "image-processing-dither.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "image-processing-dither.png",
            "caption": "Dithering modes: Floyd-Steinberg and Bayer 4x4",
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
