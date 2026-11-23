"""Visualisations for the top-level barcode API."""

import matplotlib.pyplot as plt

from raygeo.barcode import (
    QrErrorCorrection,
    Symbology,
    encode,
    min_quiet_zone,
)
from raygeo.barcode.geometry import BarcodeStyle
from tools.examples.barcode_common import fill_symbol, panel


def generate_overview():
    return _three_symbols("The three symbologies")


def generate_encode():
    figure, axes = plt.subplots(1, 3, figsize=(11, 4))
    entries = [
        ("Rayforge", Symbology.QR, QrErrorCorrection.Q, "QR, Q"),
        ("123456", Symbology.CODE128, QrErrorCorrection.M, "Code 128"),
        ("400638133393", Symbology.EAN13, QrErrorCorrection.M, "EAN-13"),
    ]
    for axis, (text, symbology, level, label) in zip(axes, entries):
        encoded = encode(text, symbology, level)
        title = f"{label}: {encoded.width} × {encoded.height} modules"
        panel(figure, axis, encoded, title, module_mm=1.0)
    figure.tight_layout()
    return figure


def generate_min_quiet_zone():
    figure, axes = plt.subplots(1, 3, figsize=(11, 4))
    entries = [
        (encode("Rayforge", Symbology.QR), Symbology.QR, "QR"),
        (encode("SN-0042", Symbology.CODE128), Symbology.CODE128, "Code 128"),
        (encode("590123412345", Symbology.EAN13), Symbology.EAN13, "EAN-13"),
    ]
    for axis, (encoded, symbology, label) in zip(axes, entries):
        fill_symbol(
            axis,
            encoded,
            BarcodeStyle(
                module_mm=0.6,
                bar_height_mm=12.0,
                show_text=symbology != Symbology.QR,
                invert=True,
            ),
            f"{label}: {min_quiet_zone(symbology)} modules",
        )
    figure.tight_layout()
    return figure


def _three_symbols(title):
    figure, axes = plt.subplots(1, 3, figsize=(11, 4))
    figure.suptitle(title, fontsize=14)
    panel(
        figure,
        axes[0],
        encode("Rayforge", Symbology.QR, QrErrorCorrection.M),
        "QR Code",
        module_mm=1.0,
    )
    panel(
        figure,
        axes[1],
        encode("SN-0042", Symbology.CODE128),
        "Code 128",
    )
    panel(
        figure,
        axes[2],
        encode("590123412345", Symbology.EAN13),
        "EAN-13",
    )
    figure.tight_layout()
    return figure


__docs_target__ = ["raygeo.barcode.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "QR Code, Code 128 and EAN-13, encoded and rendered by raygeo"
        ),
        "function": generate_overview,
    },
    {
        "heading": "encode",
        "caption": (
            "encode() dispatches to the symbology module and returns modules"
        ),
        "function": generate_encode,
    },
    {
        "heading": "min_quiet_zone",
        "caption": (
            "Inverted symbols with the minimum quiet zone per symbology"
        ),
        "function": generate_min_quiet_zone,
    },
]
