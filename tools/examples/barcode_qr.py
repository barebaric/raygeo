"""Visualisations for QR Code encoding."""

import matplotlib.pyplot as plt

from raygeo.barcode import QrErrorCorrection
from raygeo.barcode.geometry import BarcodeStyle
from raygeo.barcode.qr import encode
from tools.examples.barcode_common import fill_symbol


def generate_encode():
    text = "SN-0042 Rayforge"
    levels = [
        (QrErrorCorrection.L, "L"),
        (QrErrorCorrection.M, "M"),
        (QrErrorCorrection.Q, "Q"),
        (QrErrorCorrection.H, "H"),
    ]
    figure, axes = plt.subplots(1, 4, figsize=(12, 4))
    for axis, (level, label) in zip(axes, levels):
        encoded = encode(text, level)
        fill_symbol(
            axis,
            encoded,
            BarcodeStyle(module_mm=1.0),
            f"{label}: {encoded.width} × {encoded.height}",
        )
    figure.tight_layout()
    return figure


__docs_target__ = ["raygeo.barcode.qr.md"]
__images__ = [
    {
        "heading": "encode",
        "caption": "The four QR error-correction levels for the same content",
        "function": generate_encode,
    },
]
