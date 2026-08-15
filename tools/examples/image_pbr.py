"""Example for raygeo.image.pbr sub-module."""

import matplotlib.pyplot as plt
import numpy as np

from raygeo.image.pbr import generate_brdf_lut


def generate_brdf_lut_img():
    """Split-sum BRDF LUT."""
    lut = generate_brdf_lut(size=128)

    # Render the two LUT channels side by side as a single RGB image:
    # red = Fresnel scale, green = Fresnel bias.
    rgb = np.dstack(
        [
            lut[..., 0],
            lut[..., 1],
            np.zeros_like(lut[..., 0]),
        ]
    )
    rgb = np.clip(rgb, 0.0, 1.0)

    fig, axes = plt.subplots(1, 3, figsize=(15, 4))
    axes[0].imshow(rgb)
    axes[0].set_title("Split-sum BRDF LUT (scale=R, bias=G)")
    axes[0].set_xlabel("NdotV")
    axes[0].set_ylabel("Roughness")
    axes[1].imshow(lut[..., 0], cmap="magma", vmin=0, vmax=1)
    axes[1].set_title("Fresnel scale")
    axes[1].set_xlabel("NdotV")
    axes[1].set_ylabel("Roughness")
    axes[2].imshow(lut[..., 1], cmap="magma", vmin=0, vmax=1)
    axes[2].set_title("Fresnel bias")
    axes[2].set_xlabel("NdotV")
    axes[2].set_ylabel("Roughness")
    plt.tight_layout()
    return fig


__docs_target__ = ["raygeo.image.pbr.md"]
__images__ = [
    {
        "heading": "generate_brdf_lut",
        "caption": "Split-sum BRDF integration LUT for IBL specular",
        "function": generate_brdf_lut_img,
    },
]
