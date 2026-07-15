import matplotlib.pyplot as plt
import numpy as np
import streamlit as st

from raygeo.image.dither import (
    apply_bayer_dither,
    apply_floyd_steinberg_dither,
)
from raygeo.image.grayscale import normalize_grayscale
from raygeo.image.srgb import linear_to_srgb, srgb_to_linear
from tools.plot import make_pattern


def page_image():
    st.header("Image Processing")

    c1, c2 = st.columns(2)
    with c1:
        w = st.number_input("Width", 8, 1024, 128, step=8, key="img_w")
        h = st.number_input("Height", 8, 1024, 128, step=8, key="img_h")
    with c2:
        pattern = st.selectbox(
            "Test pattern",
            [
                "Gradient",
                "Checkered",
                "Circle",
                "Random noise",
            ],
        )

    arr = make_pattern(w, h, pattern)

    fig, axes = plt.subplots(1, 2, figsize=(10, 4))

    axes[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes[0].set_title("Original (uint8)")

    linear = srgb_to_linear(arr)
    back = linear_to_srgb(linear)
    axes[1].imshow(back, cmap="gray", vmin=0, vmax=255)
    axes[1].set_title("Round-trip (sRGB -> linear -> sRGB)")

    st.pyplot(fig)

    st.subheader("Dithering")
    dither = st.selectbox("Dither method", ["Floyd-Steinberg", "Bayer 4x4"])
    invert = st.checkbox("Invert", value=False)

    gray = normalize_grayscale(arr).astype(np.uint8)
    if dither == "Floyd-Steinberg":
        dithered = apply_floyd_steinberg_dither(gray, invert)
    else:
        bayer = np.array(
            [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]],
            dtype=np.float32,
        )
        dithered = apply_bayer_dither(gray, bayer, invert, cell_size=1)

    fig2, axes2 = plt.subplots(1, 2, figsize=(10, 4))
    axes2[0].imshow(arr, cmap="gray", vmin=0, vmax=255)
    axes2[0].set_title("Original")
    axes2[1].imshow(dithered, cmap="gray", vmin=0, vmax=1)
    axes2[1].set_title(f"Dithered ({dither})")
    st.pyplot(fig2)
