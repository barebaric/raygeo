"""Visualisations for barcode geometry rendering."""

import matplotlib.pyplot as plt

from raygeo.barcode import Symbology, encode
from raygeo.barcode.geometry import BarcodeStyle
from tools.examples.barcode_common import fill_symbol


def _single(encoded, style, title):
    figure, axes = plt.subplots(figsize=(7, 3.5))
    fill_symbol(axes, encoded, style, title)
    figure.tight_layout()
    return figure


def generate_qr():
    return _single(
        encode("Rayforge", Symbology.QR),
        BarcodeStyle(module_mm=1.0),
        "QR Code — filled outlines with module holes",
    )


def generate_code128_text():
    return _single(
        encode("SN-0042", Symbology.CODE128),
        BarcodeStyle(module_mm=0.5, bar_height_mm=15.0, show_text=True),
        "Code 128 with human-readable text",
    )


def generate_ean13_text():
    return _single(
        encode("590123412345", Symbology.EAN13),
        BarcodeStyle(module_mm=0.5, bar_height_mm=18.0, show_text=True),
        "EAN-13 with guard bars reaching into the digits",
    )


def generate_inverted():
    return _single(
        encode("Rayforge", Symbology.QR),
        BarcodeStyle(module_mm=0.6, invert=True),
        "Inverted QR code with a dark quiet-zone border",
    )


__docs_target__ = ["raygeo.barcode.geometry.md"]
__images__ = [
    {
        "heading": "to_geometry",
        "caption": (
            "A QR code as closed outlines with holes, ready for filling"
        ),
        "function": generate_qr,
    },
    {
        "heading": "to_geometry",
        "caption": "Code 128 with human-readable text below the bars",
        "function": generate_code128_text,
    },
    {
        "heading": "to_geometry",
        "caption": (
            "EAN-13 with its usual digit layout and extended guard bars"
        ),
        "function": generate_ean13_text,
    },
    {
        "heading": "to_geometry",
        "caption": "An inverted QR code with a dark quiet-zone border",
        "function": generate_inverted,
    },
]
