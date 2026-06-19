import matplotlib.pyplot as plt
import numpy as np
import streamlit as st
from matplotlib.colors import to_hex

from raygeo.geo.algo import hull
from raygeo.svg import svg_string_to_geometries
from tools.plot import (
    auto_limits,
    fill_rounded_rect,
    plot_geometry,
    rasterize_geometries_to_mask,
)

EXAMPLE_SVG = (
    '<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">\n'
    '  <rect x="10" y="10" width="30" height="30" />\n'
    '  <circle cx="70" cy="70" r="20" />\n'
    '  <path d="M 10 70 L 40 90 L 30 50 Z" />\n'
    "</svg>"
)


def page_concave_hull():
    st.header("Concave Hull (Shrink-Wrap)")

    preset = st.selectbox(
        "Shape",
        [
            "Two squares",
            "Hourglass",
            "L-shape",
            "Circle",
            "Three dots",
            "Upload SVG",
        ],
        key="ch_shape",
    )

    height = st.slider("Resolution", 200, 1000, 500, 50, key="ch_res")
    width = height

    gravity = st.slider("Gravity", 0.0, 1.0, 0.1, 0.05, key="ch_grav")

    img = np.zeros((height, width), dtype=bool)
    svg_geoms = []

    if preset == "Upload SVG":
        svg_source = st.radio(
            "SVG source", ["Upload file", "Paste SVG text"], key="ch_svg_src"
        )
        svg_str = ""
        if svg_source == "Upload file":
            uploaded = st.file_uploader(
                "Choose an SVG file", type=["svg"], key="ch_svg_file"
            )
            if uploaded is not None:
                svg_str = uploaded.read().decode("utf-8")
        else:
            svg_str = st.text_area(
                "SVG markup",
                EXAMPLE_SVG,
                height=200,
                key="ch_svg_text",
            )

        if svg_str.strip():
            try:
                svg_geoms = svg_string_to_geometries(svg_str)
                if svg_geoms:
                    img = rasterize_geometries_to_mask(
                        svg_geoms, width, height
                    )
                else:
                    st.warning("No geometries found in SVG")
            except Exception as e:
                st.error(f"Failed to parse SVG: {e}")
    elif preset == "Two squares":
        img[30:70, 30:70] = True
        img[130:170, 130:170] = True
    elif preset == "Hourglass":
        r = 8
        fill_rounded_rect(img, (60, 30), (140, 70), r)
        fill_rounded_rect(img, (80, 110), (120, 150), r)
        fill_rounded_rect(img, (60, 110), (140, 170), r)
    elif preset == "L-shape":
        img[30:170, 30:70] = True
        img[30:100, 70:170] = True
    elif preset == "Circle":
        yy, xx = np.ogrid[:height, :width]
        mask = (xx - 100) ** 2 + (yy - 100) ** 2 <= 2500
        img[mask] = True
    elif preset == "Three dots":
        for cy, cx in [(50, 50), (50, 150), (150, 100)]:
            yy, xx = np.ogrid[:height, :width]
            mask = (xx - cx) ** 2 + (yy - cy) ** 2 <= 400
            img[mask] = True

    convex_geo = hull.get_enclosing_hull(img)
    concave_geo = hull.get_concave_hull(img, gravity=gravity)
    per_component = hull.get_hulls_from_image(img)

    fig, ax = plt.subplots(figsize=(8, 8))

    ax.imshow(
        img,
        origin="upper",
        cmap="Blues",
        alpha=0.3,
        extent=(0, width, height, 0),
    )

    if convex_geo is not None:
        plot_geometry(
            ax,
            convex_geo,
            color="tomato",
            label="Convex hull",
            linewidth=1.5,
        )

    if concave_geo is not None:
        plot_geometry(
            ax,
            concave_geo,
            color="forestgreen",
            label="Concave hull",
            linewidth=2,
        )

    for i, g in enumerate(per_component):
        plot_geometry(
            ax,
            g,
            color="dodgerblue",
            label="Per-component" if i == 0 else None,
            linewidth=1,
            show_points=True,
        )

    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Gravity", f"{gravity:.2f}")
    if convex_geo is not None and concave_geo is not None:
        c2.metric("Convex area", f"{convex_geo.area():.1f}")
        c3.metric("Concave area", f"{concave_geo.area():.1f}")
    c3.metric("Components", f"{len(per_component)}")

    if preset == "Upload SVG" and svg_geoms:
        st.subheader("Parsed SVG geometries")
        c = plt.get_cmap("tab10")
        fig2, ax2 = plt.subplots(figsize=(6, 6))
        for i, g in enumerate(svg_geoms):
            plot_geometry(
                ax2,
                g,
                color=to_hex(c(i / 10)),
                label=f"Path {i}",
                linewidth=1.5,
            )
        xmin, xmax, ymin, ymax = auto_limits(svg_geoms)
        ax2.set_xlim(xmin, xmax)
        ax2.set_ylim(ymin, ymax)
        ax2.set_aspect("equal")
        ax2.grid(True, alpha=0.3)
        ax2.legend(fontsize=8)
        fig2.tight_layout()
        st.pyplot(fig2)
