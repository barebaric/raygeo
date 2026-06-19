import matplotlib.pyplot as plt
import numpy as np
import streamlit as st

from raygeo.ops.raster import (
    ScanMode,
    rasterize_mask_lines,
    rasterize_mask_scan,
    rasterize_multi_pass,
    rasterize_power_modulation,
)
from raygeo.ops.types import CommandType
from tools.plot import make_pattern


def page_rasterization():
    st.header("Rasterization")

    c1, c2 = st.columns(2)
    with c1:
        mode = st.selectbox(
            "Rasterization mode",
            [
                "Power Modulation",
                "Mask Scan",
                "Mask Lines",
                "Multi-Pass",
            ],
            key="rast_mode",
        )
        scan_mode = st.selectbox(
            "Scan mode",
            ["Segmented", "FullSweep"],
            key="rast_scan_mode",
        )
    with c2:
        pattern = st.selectbox(
            "Test pattern",
            ["Gradient", "Checkered", "Circle", "Random noise"],
            key="rast_pattern",
        )
        img_size = st.number_input(
            "Image size", 16, 256, 64, step=16, key="rast_size"
        )

    c3, c4, c5 = st.columns(3)
    with c3:
        line_interval = st.slider(
            "Line interval (mm)", 0.05, 1.0, 0.1, 0.05, key="rast_li"
        )
        angle = st.slider("Angle (deg)", 0, 90, 0, 5, key="rast_angle")
    with c4:
        ppm_val = st.number_input(
            "Pixels per mm", 1.0, 50.0, 10.0, 0.5, key="rast_ppm"
        )
    with c5:
        sample_interval = 0.05
        min_power = 0.0
        max_power = 1.0
        num_depth = 3
        z_step = 0.5
        if mode == "Power Modulation":
            sample_interval = st.slider(
                "Sample interval (mm)",
                0.01,
                0.5,
                0.05,
                0.01,
                key="rast_si",
            )
            min_power = st.slider(
                "Min power", 0.0, 1.0, 0.0, 0.1, key="rast_minp"
            )
            max_power = st.slider(
                "Max power", 0.0, 1.0, 1.0, 0.1, key="rast_maxp"
            )
        elif mode == "Multi-Pass":
            num_depth = st.slider("Depth levels", 2, 10, 3, key="rast_depth")
            z_step = st.slider(
                "Z step down", 0.1, 2.0, 0.5, 0.1, key="rast_zstep"
            )

    sm = ScanMode.Segmented if scan_mode == "Segmented" else ScanMode.FullSweep

    gray = make_pattern(img_size, img_size, pattern)

    if mode == "Power Modulation":
        alpha = np.full((img_size, img_size), 255, dtype=np.uint8)
        ops = rasterize_power_modulation(
            gray,
            alpha,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            sample_interval,
            min_power=min_power,
            max_power=max_power,
            angle=float(angle),
            scan_mode=sm,
        )
    elif mode == "Mask Scan":
        mask = (gray > 128).astype(np.uint8)
        ops = rasterize_mask_scan(
            mask,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            angle=float(angle),
            scan_mode=sm,
        )
    elif mode == "Mask Lines":
        mask = (gray > 128).astype(np.uint8)
        ops = rasterize_mask_lines(
            mask,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            angle=float(angle),
            scan_mode=sm,
        )
    else:
        ops = rasterize_multi_pass(
            gray,
            (ppm_val, ppm_val),
            0.0,
            0.0,
            line_interval,
            num_depth,
            z_step,
            angle=float(angle),
            scan_mode=sm,
        )

    fig, axes = plt.subplots(1, 2, figsize=(14, 6))

    axes[0].imshow(gray, cmap="gray", vmin=0, vmax=255, origin="lower")
    axes[0].set_title("Input image")
    axes[0].set_aspect("equal")

    cmap = plt.get_cmap("hot")

    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if abs(pos[0] - ep[0]) > 1e-6 or abs(pos[1] - ep[1]) > 1e-6:
                axes[1].plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    color="gray",
                    linewidth=0.5,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.SCAN_LINE:
            ep = ops.endpoint(i)
            sd = ops.scanline_data(i)
            n = len(sd)
            if n > 0:
                xs = np.linspace(pos[0], ep[0], n)
                ys = np.linspace(pos[1], ep[1], n)
                power_arr = np.frombuffer(sd, dtype=np.uint8).astype(
                    np.float64
                )
                power_norm = power_arr / 255.0
                colors = cmap(power_norm)
                colors[:, 3] = np.clip(power_norm * 2, 0.15, 1.0)
                axes[1].scatter(
                    xs, ys, c=colors, s=max(0.5, 800 / img_size), marker="s"
                )
            pos = ep
        elif ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            axes[1].plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color=(0.9, 0.3, 0.3, 0.9),
                linewidth=1.0,
                solid_capstyle="round",
            )
            pos = ep

    line_count = len(ops.indices_of(CommandType.LINE_TO))
    scan_count = len(
        [
            i
            for i in range(ops.len())
            if ops.command_type(i) == CommandType.SCAN_LINE
        ]
    )
    total = ops.len()

    axes[1].plot(
        [], [], color="gray", linewidth=0.5, linestyle=":", label="Travel"
    )
    axes[1].plot([], [], color=cmap(1.0), linewidth=2, label="Scan (high pwr)")
    axes[1].plot([], [], color=cmap(0.3), linewidth=2, label="Scan (low pwr)")
    axes[1].plot(
        [], [], color=(0.9, 0.3, 0.3, 0.9), linewidth=2, label="Lines"
    )
    axes[1].set_aspect("equal")
    axes[1].grid(True, alpha=0.3)
    axes[1].legend(fontsize=9)
    axes[1].set_title(f"{scan_mode} | {mode} ({angle}\u00b0)")
    fig.tight_layout()
    st.pyplot(fig)

    c1, c2, c3 = st.columns(3)
    c1.metric("Commands", total)
    c2.metric("Scan lines", scan_count)
    c3.metric("Lines", line_count)
