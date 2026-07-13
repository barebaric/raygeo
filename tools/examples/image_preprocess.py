"""Example for raygeo.image.preprocess sub-module."""

from collections import Counter

import matplotlib.pyplot as plt
import numpy as np

from raygeo.image.preprocess import (
    compute_adaptive_threshold,
    denoise_binary,
    filter_components,
    get_component_areas,
    grayscale_to_binary,
)
from tools.plot import fill_rounded_rect


def generate_otsu():
    """Otsu."""
    w, h = 128, 128
    gray2 = np.full((h, w), 200, dtype=np.uint8)
    gray2[10:60, 10:118] = 70
    yy, xx = np.ogrid[:h, :w]
    circle = (xx - 90) ** 2 + (yy - 96) ** 2 < 900
    gray2[circle] = 130

    binary_otsu = grayscale_to_binary(gray2, auto_threshold=True)
    binary_fixed = grayscale_to_binary(
        gray2, threshold=0.5, auto_threshold=False
    )

    fig, axes = plt.subplots(1, 3, figsize=(12, 4))
    axes[0].imshow(gray2, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original Grayscale")
    axes[1].imshow(binary_otsu, cmap="gray", vmin=0, vmax=1)
    axes[1].set_title("Otsu Threshold")
    axes[2].imshow(binary_fixed, cmap="gray", vmin=0, vmax=1)
    axes[2].set_title("Fixed Threshold (0.5)")
    plt.tight_layout()
    return fig


def generate_component_areas():
    """Component areas."""
    w, h = 128, 128
    binary_mask = np.zeros((h, w), dtype=bool)
    fill_rounded_rect(binary_mask, (10, 10), (50, 50), 5)
    fill_rounded_rect(binary_mask, (70, 10), (120, 40), 4)
    fill_rounded_rect(binary_mask, (10, 70), (35, 95), 3)
    fill_rounded_rect(binary_mask, (75, 60), (115, 115), 6)
    binary = binary_mask.astype(np.uint8)

    areas = get_component_areas(binary)
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
    plt.tight_layout()
    return fig2


def generate_filter_components():
    """Filter components."""
    w, h = 128, 128
    binary_mask2 = np.zeros((h, w), dtype=bool)
    fill_rounded_rect(binary_mask2, (10, 10), (50, 50), 5)
    fill_rounded_rect(binary_mask2, (70, 10), (120, 40), 4)
    fill_rounded_rect(binary_mask2, (75, 60), (115, 115), 6)
    binary_mask2[20:25, 60:65] = True
    binary_mask2[50:58, 90:96] = True
    binary_mask2[100:115, 15:20] = True
    binary2 = binary_mask2.astype(np.uint8)

    filtered = filter_components(binary2, min_area=100)

    fig3, axes3 = plt.subplots(1, 2, figsize=(8, 4))
    axes3[0].imshow(binary2, cmap="gray", vmin=0, vmax=1)
    axes3[0].set_title("Before Filtering")
    axes3[1].imshow(filtered, cmap="gray", vmin=0, vmax=1)
    axes3[1].set_title("After (min_area=100)")
    plt.tight_layout()
    return fig3


def generate_denoise_binary():
    """Denoise binary."""
    w, h = 128, 128
    binary_mask3 = np.zeros((h, w), dtype=bool)
    fill_rounded_rect(binary_mask3, (10, 10), (50, 50), 5)
    fill_rounded_rect(binary_mask3, (70, 10), (120, 40), 4)
    fill_rounded_rect(binary_mask3, (75, 60), (115, 115), 6)
    rng = np.random.default_rng(42)
    noise = rng.random((h, w)) < 0.02
    binary_mask3 |= noise
    binary3 = binary_mask3.astype(np.uint8)

    denoised = denoise_binary(binary3)

    fig4, axes4 = plt.subplots(1, 2, figsize=(8, 4))
    axes4[0].imshow(binary3, cmap="gray", vmin=0, vmax=1)
    axes4[0].set_title("Before Denoising")
    axes4[1].imshow(denoised, cmap="gray", vmin=0, vmax=1)
    axes4[1].set_title("After Denoising")
    plt.tight_layout()
    return fig4


def generate_adaptive_threshold():
    """Adaptive threshold."""
    w, h = 128, 128
    binary4 = np.zeros((h, w), dtype=bool)
    rng2 = np.random.default_rng(42)
    binary4[rng2.random((h, w)) < 0.008] = True
    binary4[20:25, 60:65] = True
    binary4[50:55, 90:95] = True
    binary4[105:110, 20:25] = True
    binary4[40:60, 40:60] = True
    binary4 = binary4.astype(np.uint8)

    areas = get_component_areas(binary4)
    thr = compute_adaptive_threshold(areas)

    counts = Counter(areas)
    area_vals = sorted(counts.keys())
    count_vals = [counts[a] for a in area_vals]

    labels = [str(a) for a in area_vals]

    fig5, (ax5_img, ax5_chart) = plt.subplots(1, 2, figsize=(10, 4))
    ax5_img.imshow(binary4, cmap="gray", vmin=0, vmax=1)
    ax5_img.set_title("Binary Image")
    ax5_img.axis("off")

    bars = ax5_chart.bar(labels, count_vals, color="steelblue", width=0.6)
    split_idx = None
    for i, a in enumerate(area_vals):
        if a < thr:
            bars[i].set_color("tomato")
        elif split_idx is None:
            split_idx = i

    if split_idx is not None:
        ax5_chart.axvline(
            x=split_idx - 0.5,
            color="red",
            linestyle="--",
            linewidth=1.5,
            label=f"Threshold = {thr} px",
        )
    ax5_chart.set_xlabel("Component area (px)")
    ax5_chart.set_ylabel("Count")
    ax5_chart.set_title("Area Distribution")
    ax5_chart.legend(fontsize=9)

    plt.tight_layout()
    return fig5


__docs_target__ = ["raygeo.image.preprocess.md"]
__images__ = [
    {
        "heading": "grayscale_to_binary",
        "caption": "Grayscale to binary via Otsu and fixed threshold",
        "function": generate_otsu,
    },
    {
        "heading": "get_component_areas",
        "caption": "Connected component areas sorted ascending",
        "function": generate_component_areas,
    },
    {
        "heading": "filter_components",
        "caption": "Component filtering by minimum area",
        "function": generate_filter_components,
    },
    {
        "heading": "denoise_binary",
        "caption": "Binary image denoised via adaptive thresholding",
        "function": generate_denoise_binary,
    },
    {
        "heading": "compute_adaptive_threshold",
        "caption": "Adaptive threshold from component area distribution",
        "function": generate_adaptive_threshold,
    },
]
