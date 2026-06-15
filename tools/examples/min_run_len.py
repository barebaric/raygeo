"""Generate minimum run length example images."""

import matplotlib.pyplot as plt
import numpy as np

import raygeo.image as img


def generate_examples(output_dir):
    images = []
    h, w = 5, 20

    binary = np.zeros((h, w), dtype=np.uint8)
    binary[0, 1:3] = 1
    binary[0, 5:12] = 1
    binary[0, 15:17] = 1
    binary[1, 0:8] = 1
    binary[1, 10:19] = 1
    binary[2, 2:4] = 1
    binary[2, 6:15] = 1
    binary[2, 17:19] = 1
    binary[3, 3:7] = 1
    binary[3, 9:12] = 1
    binary[3, 14:16] = 1
    binary[4, 0:18] = 1

    result = img.apply_minimum_run_length(binary.copy(), min_run_length=4)

    fig, axes = plt.subplots(1, 2, figsize=(8, 3))
    axes[0].imshow(binary, cmap="gray", vmin=0, vmax=1, aspect="auto")
    axes[0].set_title(f"Before ({w} px wide)")
    axes[0].set_yticks(range(h))
    axes[0].set_xticks(range(w))
    axes[1].imshow(result, cmap="gray", vmin=0, vmax=1, aspect="auto")
    axes[1].set_title("After (min_run_length=4)")
    axes[1].set_yticks(range(h))
    axes[1].set_xticks(range(w))

    fig.tight_layout()
    path = output_dir / "image-processing-min-run-len.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "image-processing-min-run-len.png",
            "caption": "Minimum run length applied to binary image",
        }
    )

    return {
        "title": "Minimum Run Length",
        "description": (
            "Remove binary runs shorter than a given minimum length."
        ),
        "images": images,
    }
