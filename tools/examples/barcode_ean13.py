"""Visualisations for EAN-13 encoding."""

import matplotlib.pyplot as plt

from raygeo.barcode.ean13 import encode
from raygeo.barcode.geometry import BarcodeStyle
from tools.examples.barcode_common import fill_symbol


def generate_encode():
    encoded = encode("590123412345")
    figure, axes = plt.subplots(figsize=(8, 3.5))
    fill_symbol(
        axes,
        encoded,
        BarcodeStyle(
            module_mm=0.5,
            bar_height_mm=18.0,
            show_text=True,
        ),
        f"EAN-13 {encoded.text} — guard bars reach into the digits",
    )
    figure.tight_layout()
    return figure


__docs_target__ = ["raygeo.barcode.ean13.md"]
__images__ = [
    {
        "heading": "encode",
        "caption": "Twelve digits, with the check digit computed and printed",
        "function": generate_encode,
    },
]
