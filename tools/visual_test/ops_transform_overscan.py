import matplotlib.pyplot as plt
import streamlit as st

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, RasterMode, SectionType


def page_overscan():
    st.header("Overscan")

    preset = st.selectbox(
        "Preset",
        [
            "Horizontal raster lines",
            "Bidirectional raster",
            "Diagonal line",
            "Variable power scanline",
            "Mixed raster + vector",
        ],
        key="os_preset",
    )

    dist = st.slider(
        "Overscan distance (mm)", 0.0, 20.0, 5.0, 0.5, key="os_dist"
    )

    ops = Ops()
    ops.set_power(1.0)

    if preset == "Horizontal raster lines":
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.move_to(10, 10, 0)
        ops.line_to(90, 10, 0)
        ops.move_to(10, 20, 0)
        ops.line_to(90, 20, 0)
        ops.move_to(10, 30, 0)
        ops.line_to(90, 30, 0)
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )
    elif preset == "Bidirectional raster":
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.move_to(10, 10, 0)
        ops.line_to(90, 10, 0)
        ops.move_to(90, 20, 0)
        ops.line_to(10, 20, 0)
        ops.move_to(10, 30, 0)
        ops.line_to(90, 30, 0)
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )
    elif preset == "Diagonal line":
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.move_to(10, 10, 0)
        ops.line_to(70, 70, 0)
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )
    elif preset == "Variable power scanline":
        pv = bytearray(range(0, 256, 4))
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.move_to(10, 50, 0)
        ops.scan_to(90, 50, 0, power_values=pv)
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )
    elif preset == "Mixed raster + vector":
        ops.move_to(5, 5, 0)
        ops.line_to(95, 95, 0)
        ops.ops_section_start(
            SectionType.RASTER_FILL,
            "wp1",
            raster_mode=RasterMode.VARIABLE_POWER,
        )
        ops.move_to(10, 20, 0)
        ops.line_to(80, 20, 0)
        ops.move_to(10, 30, 0)
        ops.line_to(80, 30, 0)
        ops.ops_section_end(
            SectionType.RASTER_FILL, raster_mode=RasterMode.VARIABLE_POWER
        )

    orig = ops.copy()
    orig_lines = len(ops.indices_of(CommandType.LINE_TO))
    orig_scans = len(
        [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
    )

    ops.apply_overscan(dist)

    result_lines = len(ops.indices_of(CommandType.LINE_TO))
    result_scans = len(
        [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
    )

    fig, ax = plt.subplots(figsize=(12, 8))

    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep
        elif ct == CommandType.SCAN_LINE:
            ep = orig.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="tomato",
                linewidth=5,
                alpha=0.35,
                solid_capstyle="round",
            )
            pos = ep

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep
        elif ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            ax.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="forestgreen",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep

    ax.plot([], [], color="tomato", linewidth=5, alpha=0.35, label="Original")
    ax.plot([], [], color="forestgreen", linewidth=2.5, label="With overscan")
    ax.plot([], [], color="gray", linewidth=0.7, linestyle=":", label="Travel")
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Lines", f"{orig_lines} -> {result_lines}")
    c2.metric("Scan lines", f"{orig_scans} -> {result_scans}")
    c3.metric("Overscan", f"{dist:.1f} mm")
