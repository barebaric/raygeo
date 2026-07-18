"""Generate apply_multipass example images."""

import matplotlib.pyplot as plt

from raygeo.ops import Ops
from raygeo.ops.types import CommandType, SectionType


def generate_multipass():
    ops = Ops()
    ops.set_power(1.0)
    ops.ops_section_start(SectionType.VECTOR_OUTLINE, "wp1")
    ops.move_to(25, 25, 0)
    ops.line_to(75, 25, 0)
    ops.line_to(75, 75, 0)
    ops.line_to(25, 75, 0)
    ops.close_path()
    ops.ops_section_end(SectionType.VECTOR_OUTLINE)

    n_passes = 4
    z_step = 2.0
    orig = ops.copy()
    ops.apply_multipass(n_passes, z_step)

    fig = plt.figure(figsize=(14, 6))

    ax1 = fig.add_subplot(1, 2, 1)
    orig.preload_state()
    pos = (0.0, 0.0, 0.0)
    for i in range(orig.len()):
        ct = orig.command_type(i)
        if ct == CommandType.MOVE_TO:
            pos = orig.endpoint(i)
            continue
        if ct == CommandType.LINE_TO:
            ep = orig.endpoint(i)
            ax1.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                color="steelblue",
                linewidth=2.5,
                solid_capstyle="round",
            )
            pos = ep
    ax1.set_aspect("equal")
    ax1.set_xlim(0, 100)
    ax1.set_ylim(0, 100)
    ax1.grid(True, alpha=0.3)
    ax1.set_title("Original (1 pass)", fontsize=12)

    ax2 = fig.add_subplot(1, 2, 2, projection="3d")
    ops.preload_state()
    pos = (0.0, 0.0, 0.0)
    colors = ["steelblue", "forestgreen", "darkorange", "tomato"]
    z_colors = {}
    for i in range(ops.len()):
        ct = ops.command_type(i)
        if ct == CommandType.MOVE_TO:
            ep = ops.endpoint(i)
            if pos != ep:
                ax2.plot(
                    [pos[0], ep[0]],
                    [pos[1], ep[1]],
                    [pos[2], ep[2]],
                    color="gray",
                    linewidth=0.7,
                    linestyle=":",
                )
            pos = ep
            continue
        if ct == CommandType.LINE_TO:
            ep = ops.endpoint(i)
            z = pos[2]
            if z not in z_colors:
                z_colors[z] = colors[len(z_colors) % len(colors)]
            ax2.plot(
                [pos[0], ep[0]],
                [pos[1], ep[1]],
                [pos[2], ep[2]],
                color=z_colors[z],
                linewidth=2,
            )
            pos = ep

    for z_val, color in z_colors.items():
        pass_num = int(abs(z_val) / z_step) if z_step > 0 else 0
        ax2.plot(
            [], [], [], color=color, linewidth=2, label=f"Pass {pass_num}"
        )
    ax2.plot(
        [], [], [], color="gray", linewidth=0.7, linestyle=":", label="Travel"
    )
    ax2.set_xlabel("X")
    ax2.set_ylabel("Y")
    ax2.set_zlabel("Z")
    ax2.set_title(
        f"MultiPass ({n_passes} passes, Z step={z_step})", fontsize=12
    )
    ax2.legend(fontsize=8)
    ax2.view_init(elev=25, azim=-40)

    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.ops.md"]
__images__ = [
    {
        "heading": "apply_multipass",
        "caption": "Multi-pass with Z stepping",
        "function": generate_multipass,
    },
]
