import pytest

from raygeo.barcode import (
    QrErrorCorrection,
    Symbology,
    encode,
    min_quiet_zone,
)
from raygeo.barcode.code128 import encode as encode_code128
from raygeo.barcode.ean13 import encode as encode_ean13
from raygeo.barcode.qr import encode as encode_qr


class TestQr:
    def test_square_and_version_formula(self):
        encoded = encode_qr("https://rayforge.org")
        assert encoded.symbology == Symbology.QR
        assert encoded.width == encoded.height
        assert (encoded.width - 17) % 4 == 0

    def test_finder_pattern(self):
        row = encode_qr("x").modules[0]
        assert tuple(row[:7]) == (True,) * 7
        assert not row[7]

    def test_higher_error_correction_needs_more_modules(self):
        text = "SN-0042 " * 6
        low = encode_qr(text, QrErrorCorrection.L)
        high = encode_qr(text, QrErrorCorrection.H)
        assert high.width > low.width

    def test_rejects_empty(self):
        with pytest.raises(ValueError):
            encode_qr("")

    def test_rejects_oversized_content(self):
        with pytest.raises(ValueError):
            encode_qr("x" * 4000)


class TestCode128:
    def test_text_width_and_start_stop(self):
        encoded = encode_code128("Ab")
        assert encoded.symbology == Symbology.CODE128
        assert encoded.width == 57
        assert encoded.height == 1
        assert encoded.text == "Ab"
        start = "11010010000"
        assert tuple(encoded.modules[0][:11]) == tuple(
            bit == "1" for bit in start
        )
        assert encoded.modules[0][-1] is True

    def test_single_character_width_includes_checksum_and_stop(self):
        assert encode_code128("A").width == 46

    def test_long_digit_run_uses_code_c(self):
        assert encode_code128("123456").width == 68

    def test_odd_digit_run_finishes_in_code_b(self):
        assert encode_code128("12345").width == 79

    def test_switches_to_code_c_for_a_long_run(self):
        encoded = encode_code128("X123456")
        assert encoded.width == 90
        assert encoded.text == "X123456"

    def test_rejects_non_ascii_and_empty(self):
        with pytest.raises(ValueError):
            encode_code128("é")
        with pytest.raises(ValueError):
            encode_code128("")


class TestEan13:
    def test_adds_the_check_digit(self):
        encoded = encode_ean13("590123412345")
        assert encoded.symbology == Symbology.EAN13
        assert encoded.text == "5901234123457"
        assert encoded.width == 95
        assert encoded.height == 1

    def test_verifies_a_supplied_check_digit(self):
        assert encode_ean13("4006381333931").text == "4006381333931"
        with pytest.raises(ValueError, match="check digit"):
            encode_ean13("4006381333932")

    def test_rejects_wrong_input(self):
        for text in ("12345", "12345678901a", "12345678901234"):
            with pytest.raises(ValueError):
                encode_ean13(text)

    def test_guards(self):
        encoded = encode_ean13("590123412345")
        row = encoded.modules[0]
        assert [row[i] for i in (0, 1, 2)] == [True, False, True]
        assert [row[i] for i in range(45, 50)] == [
            False,
            True,
            False,
            True,
            False,
        ]
        assert {0, 2, 46, 48, 92, 94} <= set(encoded.guards)


def test_encode_dispatches():
    assert encode("590123412345", Symbology.EAN13).width == 95
    assert encode("A", Symbology.CODE128).height == 1
    assert encode("A", Symbology.QR).height > 1


def test_min_quiet_zone_per_symbology():
    assert min_quiet_zone(Symbology.QR) == 4
    assert min_quiet_zone(Symbology.CODE128) == 10
    assert min_quiet_zone(Symbology.EAN13) == 9


def test_enum_values_round_trip():
    for symbology in Symbology.QR, Symbology.CODE128, Symbology.EAN13:
        assert Symbology.from_value(symbology.value) == symbology
    for level in (
        QrErrorCorrection.L,
        QrErrorCorrection.M,
        QrErrorCorrection.Q,
        QrErrorCorrection.H,
    ):
        assert QrErrorCorrection.from_value(level.value) == level
    with pytest.raises(ValueError):
        Symbology.from_value("unknown")
    with pytest.raises(ValueError):
        QrErrorCorrection.from_value("X")


def test_enums_pickle_and_copy():
    import copy
    import pickle

    for value in (Symbology.QR, QrErrorCorrection.H):
        assert pickle.loads(pickle.dumps(value)) == value
        assert copy.deepcopy(value) == value
