import matplotlib.pyplot as plt
import streamlit as st

from raygeo.geo.algo.minkowski2d import get_polygon_minkowski_sum_convex
from tools.plot import plot_polygon


def page_minkowski():
    st.header("Minkowski Sum")
    st.write("Compute the Minkowski sum of two convex polygons.")

    c1, c2 = st.columns(2)
    with c1:
        st.subheader("Polygon A (triangle)")
        a_size = st.number_input("Triangle size", 5.0, 100.0, 40.0, key="mk_a")
        a_off = st.number_input(
            "Triangle offset", -50.0, 50.0, 0.0, key="mk_ao"
        )
    with c2:
        st.subheader("Polygon B (square)")
        b_size = st.number_input("Square size", 5.0, 100.0, 20.0, key="mk_b")
        b_off = st.number_input("Square offset", -50.0, 50.0, 0.0, key="mk_bo")

    tri = [
        (a_off, a_off),
        (a_off + a_size, a_off),
        (a_off + a_size / 2, a_off + a_size * 0.875),
    ]
    sq = [
        (b_off, b_off),
        (b_off + b_size, b_off),
        (b_off + b_size, b_off + b_size),
        (b_off, b_off + b_size),
    ]

    result = get_polygon_minkowski_sum_convex(tri, sq)

    fig, ax = plt.subplots(figsize=(8, 8))
    plot_polygon(ax, tri, "steelblue", "Triangle (A)", linewidth=2.5)
    plot_polygon(ax, sq, "tomato", "Square (B)", linewidth=2.5)
    for poly in result:
        plot_polygon(
            ax, poly, "limegreen", "A ⊕ B (Minkowski sum)", linewidth=2.5
        )
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=11)

    all_pts = tri + sq
    for p in result:
        all_pts.extend(p)
    xs = [p[0] for p in all_pts]
    ys = [p[1] for p in all_pts]
    if xs:
        xmin, xmax = min(xs), max(xs)
        ymin, ymax = min(ys), max(ys)
        margin = max(xmax - xmin, ymax - ymin) * 0.25 + 20
        ax.set_xlim(xmin - margin, xmax + margin)
        ax.set_ylim(ymin - margin, ymax + margin)

    fig.tight_layout()
    st.pyplot(fig)

    if result:
        st.success(f"Minkowski sum produced {len(result)} polygon(s)")
    else:
        st.warning("No result — polygons may not be convex")
