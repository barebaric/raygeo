"""Generate image preprocess example images."""

import matplotlib.pyplot as plt
import numpy as np

import raygeo.image as img
from tools.plot import fill_rounded_rect


def generate_examples(output_dir):
    images = []
    w, h = 128, 128

    # --- grayscale_to_binary ---
    # Bright bg (200) with a dark rectangle (70) and a mid-gray circle (130).
    # Otsu picks threshold ~130 so the circle is foreground.
    # Fixed 0.5 → 127 truncates, so the circle (130) becomes background.
    gray = np.full((h, w), 200, dtype=np.uint8)
    gray[10:60, 10:118] = 70
    yy, xx = np.ogrid[:h, :w]
    circle = (xx - 90) ** 2 + (yy - 96) ** 2 < 900
    gray[circle] = 130

    binary_otsu = img.grayscale_to_binary(gray, auto_threshold=True)
    binary_fixed = img.grayscale_to_binary(
        gray, threshold=0.5, auto_threshold=False
    )

    fig, axes = plt.subplots(1, 3, figsize=(12, 4))
    axes[0].imshow(gray, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original Grayscale")
    axes[1].imshow(binary_otsu, cmap="gray", vmin=0, vmax=1)
    axes[1].set_title("Otsu Threshold")
    axes[2].imshow(binary_fixed, cmap="gray", vmin=0, vmax=1)
    axes[2].set_title("Fixed Threshold (0.5)")
    fig.tight_layout()
    path = output_dir / "image-processing-otsu.png"
    fig.savefig(path, dpi=150)
    plt.close(fig)
    images.append(
        {
            "path": "image-processing-otsu.png",
            "caption": "Grayscale to binary via Otsu and fixed threshold",
        }
    )

    # --- get_component_areas ---
    binary_mask = np.zeros((h, w), dtype=bool)
    fill_rounded_rect(binary_mask, (10, 10), (50, 50), 5)
    fill_rounded_rect(binary_mask, (70, 10), (120, 40), 4)
    fill_rounded_rect(binary_mask, (10, 70), (35, 95), 3)
    fill_rounded_rect(binary_mask, (75, 60), (115, 115), 6)
    binary = binary_mask.astype(np.uint8)

    areas = img.get_component_areas(binary)
    areas_text = "\n".join(f"  {a} px" for a in areas)

    fig2, axes2 = plt.subplots(1, 2, figsize=(8, 4))
    axes2[0].imshow(binary, cmap="gray", vmin=0, vmax=1)
    axes2[0].set_title("Binary Components")
    axes2[0].axis("off")
    axes2[1].axis("off")
    axes2[1].text(
        0.1,
        0.5,
        f"Sorted component areas:\n\n{areas_text}",
        fontsize=12,
        verticalalignment="center",
        fontfamily="monospace",
    )
    axes2[1].set_title("Pixel Areas")
    fig2.tight_layout()
    path2 = output_dir / "image-processing-component-areas.png"
    fig2.savefig(path2, dpi=150)
    plt.close(fig2)
    images.append(
        {
            "path": "image-processing-component-areas.png",
            "caption": "Connected component areas sorted ascending",
        }
    )

    # --- filter_components ---
    binary_mask2 = np.zeros((h, w), dtype=bool)
    fill_rounded_rect(binary_mask2, (10, 10), (50, 50), 5)
    fill_rounded_rect(binary_mask2, (70, 10), (120, 40), 4)
    fill_rounded_rect(binary_mask2, (75, 60), (115, 115), 6)
    binary_mask2[20:25, 60:65] = True
    binary_mask2[50:58, 90:96] = True
    binary_mask2[100:115, 15:20] = True
    binary2 = binary_mask2.astype(np.uint8)

    filtered = img.filter_components(binary2, min_area=100)

    fig3, axes3 = plt.subplots(1, 2, figsize=(8, 4))
    axes3[0].imshow(binary2, cmap="gray", vmin=0, vmax=1)
    axes3[0].set_title("Before Filtering")
    axes3[1].imshow(filtered, cmap="gray", vmin=0, vmax=1)
    axes3[1].set_title("After (min_area=100)")
    fig3.tight_layout()
    path3 = output_dir / "image-processing-filter-components.png"
    fig3.savefig(path3, dpi=150)
    plt.close(fig3)
    images.append(
        {
            "path": "image-processing-filter-components.png",
            "caption": "Connected components filtered by minimum area",
        }
    )

    return {
        "title": "Image Preprocessing",
        "description": (
            "Preprocessing functions including grayscale-to-binary conversion "
            "via Otsu thresholding, connected component analysis, "
            "and component filtering."
        ),
        "images": images,
    }
