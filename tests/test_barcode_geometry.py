import pytest

from raygeo.barcode.code128 import encode as encode_code128
from raygeo.barcode.ean13 import encode as encode_ean13
from raygeo.barcode.geometry import BarcodeStyle, to_geometry
from raygeo.barcode.qr import encode as encode_qr
from raygeo.geo.shape.polygon import get_polygon_signed_area


def _loops(encoded, style):
    return to_geometry(encoded, style).to_polygons(0.001)


def _bbox(geometry):
    return geometry.rect()


def test_qr_outline_has_holes():
    geometry = to_geometry(encode_qr("x"), BarcodeStyle(module_mm=1.0))
    assert _bbox(geometry) == pytest.approx((0.0, 0.0, 21.0, 21.0))
    areas = [
        get_polygon_signed_area(loop)
        for loop in _loops(encode_qr("x"), BarcodeStyle(module_mm=1.0))
    ]
    assert min(areas) < 0.0
    assert sum(areas) == pytest.approx(228.0)


def test_invert_adds_a_dark_quiet_zone():
    geometry = to_geometry(
        encode_qr("x"), BarcodeStyle(module_mm=1.0, invert=True)
    )
    assert _bbox(geometry) == pytest.approx((0.0, 0.0, 29.0, 29.0))
    areas = [
        get_polygon_signed_area(loop)
        for loop in _loops(
            encode_qr("x"), BarcodeStyle(module_mm=1.0, invert=True)
        )
    ]
    assert sum(areas) == pytest.approx(29.0 * 29.0 - 228.0)


def test_one_dimensional_bars_get_the_bar_height():
    encoded = encode_code128("SN-0042")
    plain = to_geometry(
        encoded, BarcodeStyle(module_mm=0.5, bar_height_mm=12.0)
    )
    assert _bbox(plain) == pytest.approx((0.0, 0.0, 50.5, 12.0))

    with_text = to_geometry(
        encoded,
        BarcodeStyle(module_mm=0.5, bar_height_mm=12.0, show_text=True),
    )
    x0, _y0, x1, y1 = _bbox(with_text)
    assert (x0, x1) == pytest.approx((0.0, 50.5))
    assert y1 > 12.0


def test_ean_text_uses_the_usual_layout():
    encoded = encode_ean13("590123412345")
    geometry = to_geometry(
        encoded,
        BarcodeStyle(module_mm=0.33, bar_height_mm=15.0, show_text=True),
    )
    x0, y0, x1, y1 = _bbox(geometry)
    assert y0 == pytest.approx(0.33, abs=0.05)
    assert y1 > 15.0
    assert x0 < 0.0
    assert x1 == pytest.approx(95.0 * 0.33)


def test_qr_ignores_bar_height_and_text():
    geometry = to_geometry(
        encode_qr("x"),
        BarcodeStyle(
            module_mm=1.0,
            bar_height_mm=99.0,
            show_text=True,
        ),
    )
    assert _bbox(geometry) == pytest.approx((0.0, 0.0, 21.0, 21.0))


def test_quiet_zone_defaults_to_the_symbology_minimum():
    geometry = to_geometry(
        encode_code128("A"),
        BarcodeStyle(module_mm=0.5, bar_height_mm=12.0, invert=True),
    )
    assert _bbox(geometry) == pytest.approx((0.0, 0.0, 33.0, 22.0))


def test_invalid_style_values_raise():
    encoded = encode_qr("x")
    with pytest.raises(ValueError):
        to_geometry(encoded, BarcodeStyle(module_mm=0.0))
    with pytest.raises(ValueError):
        to_geometry(encoded, BarcodeStyle(bar_height_mm=-1.0))
    with pytest.raises(ValueError):
        to_geometry(encoded, BarcodeStyle(quiet_zone=1001))


def test_contours_are_oriented_by_nesting():
    areas = [
        get_polygon_signed_area(loop)
        for loop in _loops(encode_qr("x"), BarcodeStyle(module_mm=1.0))
    ]
    assert min(areas) < 0.0 < max(areas)
    assert sum(areas) > 0.0
