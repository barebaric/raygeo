"""Example for raygeo.image.render sub-module."""

from tools.examples.image import generate_geometry_to_image

__docs_target__ = ["raygeo.image.render.md"]

__images__ = [
    {
        "heading": "geometry_to_image",
        "caption": "Vector geometry rasterised into an RGBA pixel buffer",
        "function": generate_geometry_to_image,
    },
]
