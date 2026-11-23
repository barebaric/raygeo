"""Visualisations for Code 128 encoding."""

import matplotlib.pyplot as plt

from raygeo.barcode.code128 import encode
from raygeo.barcode.geometry import BarcodeStyle
from tools.examples.barcode_common import fill_symbol


def generate_encode():
    entries = [
        ("Ab", "Code set B: text"),
        ("123456", "Code set C: digit pairs"),
        ("X123456", "Switch to C for a long digit run"),
    ]
    figure, axes = plt.subplots(3, 1, figsize=(10, 7))
    for axis, (text, label) in zip(axes, entries):
        encoded = encode(text)
        fill_symbol(
            axis,
            encoded,
            BarcodeStyle(
                module_mm=0.5,
                bar_height_mm=14.0,
                show_text=True,
            ),
            f"{label} — {encoded.width} modules",
        )
    figure.tight_layout()
    return figure


__docs_target__ = ["raygeo.barcode.code128.md"]
__images__ = [
    {
        "heading": "encode",
        "caption": "Code set B, code set C, and an automatic B-to-C switch",
        "function": generate_encode,
    },
]
