"""Generate image processing example images."""

from collections import Counter

import matplotlib.pyplot as plt
import numpy as np

import raygeo.image as img
from raygeo.image import rasterize_scanlines
from raygeo.ops.raster import ScanMode, rasterize_power_modulation
from raygeo.ops.types import CommandType
from tools.plot import fill_rounded_rect, make_pattern


def _plot_dither(gray, dithered, title):
    fig, axes = plt.subplots(1, 2, figsize=(10, 4))
    axes[0].imshow(gray, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original")
    axes[1].imshow(dithered, cmap="gray", vmin=0, vmax=1)
    axes[1].set_title(title)
    fig.tight_layout()
    return fig


def generate_srgb():
    """sRGB."""
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
    return fig


def generate_dither_floyd():
    """Floyd dither."""
    w, h = 128, 128
    arr = make_pattern(w, h, "Gradient")
    gray = img.normalize_grayscale(arr).astype(np.uint8)

    dithered_fs = img.apply_floyd_steinberg_dither(gray, False)
    return _plot_dither(gray, dithered_fs, "Floyd-Steinberg")


def generate_dither_bayer():
    """Bayer dither."""
    w, h = 128, 128
    arr = make_pattern(w, h, "Gradient")
    gray = img.normalize_grayscale(arr).astype(np.uint8)

    bayer = np.array(
        [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
        dtype=np.float32,
    )
    dithered_bayer = img.apply_bayer_dither(gray, bayer, False, cell_size=1)
    return _plot_dither(gray, dithered_bayer, "Bayer 4x4")


def generate_otsu():
    """Otsu."""
    w, h = 128, 128
    gray2 = np.full((h, w), 200, dtype=np.uint8)
    gray2[10:60, 10:118] = 70
    yy, xx = np.ogrid[:h, :w]
    circle = (xx - 90) ** 2 + (yy - 96) ** 2 < 900
    gray2[circle] = 130

    binary_otsu = img.grayscale_to_binary(gray2, auto_threshold=True)
    binary_fixed = img.grayscale_to_binary(
        gray2, threshold=0.5, auto_threshold=False
    )

    fig, axes = plt.subplots(1, 3, figsize=(12, 4))
    axes[0].imshow(gray2, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original Grayscale")
    axes[1].imshow(binary_otsu, cmap="gray", vmin=0, vmax=1)
    axes[1].set_title("Otsu Threshold")
    axes[2].imshow(binary_fixed, cmap="gray", vmin=0, vmax=1)
    axes[2].set_title("Fixed Threshold (0.5)")
    fig.tight_layout()
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

    filtered = img.filter_components(binary2, min_area=100)

    fig3, axes3 = plt.subplots(1, 2, figsize=(8, 4))
    axes3[0].imshow(binary2, cmap="gray", vmin=0, vmax=1)
    axes3[0].set_title("Before Filtering")
    axes3[1].imshow(filtered, cmap="gray", vmin=0, vmax=1)
    axes3[1].set_title("After (min_area=100)")
    fig3.tight_layout()
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

    denoised = img.denoise_binary(binary3)

    fig4, axes4 = plt.subplots(1, 2, figsize=(8, 4))
    axes4[0].imshow(binary3, cmap="gray", vmin=0, vmax=1)
    axes4[0].set_title("Before Denoising")
    axes4[1].imshow(denoised, cmap="gray", vmin=0, vmax=1)
    axes4[1].set_title("After Denoising")
    fig4.tight_layout()
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

    areas = img.get_component_areas(binary4)
    thr = img.compute_adaptive_threshold(areas)

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

    fig5.tight_layout()
    return fig5


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

    result = img.apply_minimum_run_length(binary5.copy(), min_run_length=4)

    fig6, axes6 = plt.subplots(1, 2, figsize=(8, 3))
    axes6[0].imshow(binary5, cmap="gray", vmin=0, vmax=1, aspect="auto")
    axes6[0].set_title(f"Before ({w2} px wide)")
    axes6[0].set_yticks(range(h2))
    axes6[0].set_xticks(range(w2))
    axes6[1].imshow(result, cmap="gray", vmin=0, vmax=1, aspect="auto")
    axes6[1].set_title("After (min_run_length=4)")
    axes6[1].set_yticks(range(h2))
    axes6[1].set_xticks(range(w2))

    fig6.tight_layout()
    return fig6


def generate_rasterize_scanlines():
    """Rasterize scanlines."""
    img_size = 64
    ppm = 10.0

    gray3 = make_pattern(img_size, img_size, "Radial")
    alpha = np.full((img_size, img_size), 255, dtype=np.uint8)

    ops = rasterize_power_modulation(
        gray3,
        alpha,
        (ppm, ppm),
        0.0,
        0.0,
        line_interval_mm=0.1,
        sample_interval_mm=0.05,
        min_power=0.0,
        max_power=1.0,
        angle=0,
        scan_mode=ScanMode.Segmented,
    )

    rasterized = rasterize_scanlines(
        ops, img_size, img_size, (ppm, ppm), (0.0, 0.0)
    )

    fig7, axes7 = plt.subplots(1, 2, figsize=(14, 6))
    max_mm = img_size / ppm

    ops.preload_state()
    power_cmap = plt.get_cmap("plasma")
    pos = (0.0, 0.0, 0.0)
    for i in range(len(ops)):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            n = max(1, len(ops.scanline_data(i)))
            xs = np.linspace(pos[0], ep[0], n)
            ys = np.linspace(pos[1], ep[1], n)
            pwr = np.frombuffer(ops.scanline_data(i), dtype=np.uint8).astype(
                np.float64
            )
            pn = pwr / 255.0
            for j in range(n - 1):
                axes7[0].plot(
                    xs[j : j + 2],
                    ys[j : j + 2],
                    color=power_cmap(pn[j] if j < len(pn) else 0),
                    linewidth=1.2,
                )
            pos = ep

    axes7[0].set_xlim(0, max_mm)
    axes7[0].set_ylim(0, max_mm)
    axes7[0].set_aspect("equal")
    axes7[0].grid(True, alpha=0.3)
    axes7[0].set_title("Input: Ops scanlines (power-colored)")

    axes7[1].imshow(
        rasterized,
        cmap="plasma",
        vmin=0,
        vmax=255,
        origin="lower",
        extent=[0, max_mm, 0, max_mm],
    )
    axes7[1].set_title("Output: rasterized power-map buffer")
    axes7[1].set_xlim(0, max_mm)
    axes7[1].set_ylim(0, max_mm)
    axes7[1].set_aspect("equal")

    fig7.tight_layout()
    return fig7


__images__ = [
    {
        "heading": "srgb_to_linear",
        "caption": "sRGB to linear round-trip",
        "function": generate_srgb,
    },
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
    {
        "heading": "apply_minimum_run_length",
        "caption": "Minimum run length applied to binary image",
        "function": generate_min_run_len,
    },
    {
        "heading": "rasterize_scanlines",
        "caption": "Scanline ops rasterized into a 2D power-map buffer",
        "function": generate_rasterize_scanlines,
    },
]
