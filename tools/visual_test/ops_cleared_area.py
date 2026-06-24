import matplotlib.pyplot as plt
import streamlit as st

from raygeo.ops.cleared_area import ClearedArea


def page_adaptive_clearing():
    st.header("Adaptive Clearing  --  ClearedArea")

    c1, c2 = st.columns(2)
    with c1:
        tool_dia = st.number_input(
            "Tool diameter", 0.5, 20.0, 6.0, key="ca_dia"
        )
        step = st.number_input("Step distance", 0.1, 10.0, 1.0, key="ca_step")
    with c2:
        n_steps = st.number_input("Number of steps", 1, 100, 20, key="ca_n")
        st.checkbox("Show full boundary", value=True, key="ca_show")

    tool_radius = tool_dia / 2
    boundary = [(5, 5), (85, 5), (85, 85), (5, 85)]

    ca = ClearedArea()
    for i in range(n_steps):
        x = 10.0 + i * step
        if x > 80.0:
            break
        path = [(x, 10), (x, 80)]
        ca.expand(path, tool_radius)

    remaining = ca.remaining([boundary])

    fig, ax = plt.subplots(figsize=(8, 7))
    bx, by = zip(*(boundary + [boundary[0]]))
    ax.plot(bx, by, "k-", linewidth=1.5, label="Boundary")

    all_frags = ca.query_window((-10, -10, 100, 100))
    for frag in all_frags:
        fx, fy = zip(
            *([(p[0], p[1]) for p in frag] + [(frag[0][0], frag[0][1])])
        )
        ax.fill(fx, fy, "steelblue", alpha=0.3)
        ax.plot(fx, fy, "steelblue", linewidth=1, alpha=0.6)

    for poly in remaining:
        px, py = zip(
            *([(p[0], p[1]) for p in poly] + [(poly[0][0], poly[0][1])])
        )
        ax.fill(
            px,
            py,
            "tomato",
            alpha=0.4,
            label="Remaining" if poly is remaining[0] else None,
        )
        ax.plot(px, py, "tomato", linewidth=1.5)

    ax.set_aspect("equal")
    ax.set_xlim(0, 90)
    ax.set_ylim(0, 90)
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=9)
    n_frags = len(ca.query_window((-10, -10, 100, 100)))
    ax.set_title(
        f"Cleared: {ca.total_area():.1f} mm^2  |  Fragments: {n_frags}"
    )
    st.pyplot(fig)

    col1, col2, col3 = st.columns(3)
    col1.metric("Tool radius", f"{tool_radius:.2f} mm")
    col2.metric("Cleared area", f"{ca.total_area():.2f} mm^2")
    col3.metric("Boundary area", "6400 mm^2")
