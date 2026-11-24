"""Tests for the native CorelDRAW reader."""

import io
import zipfile
from pathlib import Path

import pytest

from raygeo.formats.coreldraw import (
    CdrError,
    CdrFormat,
    CdrParseError,
    CdrUnsupportedFormat,
    detect_format,
    parse,
)

ASSETS = Path(__file__).parent / "assets"

CIRCLE_BOUNDS = (1.7930, 3.0430, 6.7069, 7.9569)
SQUARE_BOUNDS = (2.4764, 3.9216, 6.0236, 7.0784)


def _riff_drawing(form: bytes, body: bytes = b"") -> bytes:
    payload = form + body
    return b"RIFF" + len(payload).to_bytes(4, "little") + payload


def _zip_drawing(member: str, data: bytes) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr(member, data)
    return buffer.getvalue()


class TestDetectFormat:
    @pytest.mark.parametrize(
        "version",
        [b" ", b"5", b"9", b"A", b"C", b"H", b"J", b"O", b"P", b"R", b"U"],
    )
    def test_riff_versions(self, version):
        data = _riff_drawing(b"CDR" + version)
        assert detect_format(data) is CdrFormat.CDR

    def test_template(self):
        assert detect_format(_riff_drawing(b"CDTQ")) is CdrFormat.CDR

    def test_cmx(self):
        assert detect_format(_riff_drawing(b"CMX1")) is CdrFormat.CMX
        riffx = b"RIFX" + (8).to_bytes(4, "little") + b"CMX1"
        assert detect_format(riffx) is CdrFormat.CMX

    def test_waldo(self):
        assert detect_format(b"WL\xb5\x00\x00rest") is CdrFormat.CDR

    def test_zip_drawing(self):
        data = _zip_drawing("content/riffData.cdr", _riff_drawing(b"CDRC"))
        assert detect_format(data) is CdrFormat.CDR

    @pytest.mark.parametrize("name", ["circle.cdr", "square.cdr", "pages.cdr"])
    def test_corpus_samples(self, name):
        assert detect_format((ASSETS / name).read_bytes()) is CdrFormat.CDR

    @pytest.mark.parametrize(
        "data",
        [
            b"",
            b"RIFF",
            b"PK\x03\x04",
            b"\x89PNG\r\n\x1a\n" + b"\x00" * 16,
            b"<svg xmlns='http://www.w3.org/2000/svg'/>",
            _riff_drawing(b"WAVE"),
            _riff_drawing(b"CDR\x00"),
        ],
    )
    def test_other_data_is_rejected(self, data):
        assert detect_format(data) is None

    def test_zip_without_drawing_is_rejected(self):
        data = _zip_drawing("word/document.xml", b"PK")
        assert detect_format(data) is None


class TestParse:
    def test_circle_riff(self):
        document = parse((ASSETS / "circle.cdr").read_bytes())

        assert document.page_width == pytest.approx(8.5)
        assert document.page_height == pytest.approx(11.0)
        assert document.page_count == 1
        assert document.issues == []
        assert document.units_to_mm == pytest.approx(25.4)

        assert len(document.layers) == 1
        name, geometry = document.layers[0]
        assert name == "Layer 1"
        assert len(geometry) == 6
        kinds = {type(command).__name__ for command in geometry.data}
        assert kinds == {"Move", "Line", "Bezier"}
        assert geometry.rect() == pytest.approx(CIRCLE_BOUNDS, abs=0.01)

    def test_square_zip(self):
        document = parse((ASSETS / "square.cdr").read_bytes())

        assert document.page_width == pytest.approx(8.5)
        assert document.page_height == pytest.approx(11.0)
        assert document.page_count == 1
        assert document.issues == []

        assert len(document.layers) == 1
        name, geometry = document.layers[0]
        assert name == "Layer 1"
        kinds = {type(command).__name__ for command in geometry.data}
        assert kinds == {"Move", "Line"}
        assert geometry.rect() == pytest.approx(SQUARE_BOUNDS, abs=0.001)

    def test_text_only_pages(self):
        document = parse((ASSETS / "pages.cdr").read_bytes())

        assert document.page_count == 2
        assert document.layers == []
        assert len(document.issues) == 1
        assert "text object(s)" in document.issues[0]

    def test_zip_and_riff_agree(self, tmp_path):
        data = (ASSETS / "circle.cdr").read_bytes()
        archive = tmp_path / "circle.cdr"
        archive.write_bytes(_zip_drawing("content/riffData.cdr", data))

        from_zip = parse(archive.read_bytes())

        assert from_zip.page_width == pytest.approx(8.5)
        assert from_zip.layers[0][1].rect() == pytest.approx(
            CIRCLE_BOUNDS, abs=0.01
        )

    def test_not_a_drawing(self):
        with pytest.raises(CdrUnsupportedFormat):
            parse(b"\x89PNG\r\n\x1a\n" + b"\x00" * 16)

    def test_cmx_is_reported(self):
        with pytest.raises(CdrParseError) as error:
            parse(_riff_drawing(b"CMX1"))
        assert "CMX" in str(error.value)

    def test_waldo_is_reported(self):
        with pytest.raises(CdrParseError) as error:
            parse(b"WL\xb5\x00\x00rest")
        assert "WL" in str(error.value)

    def test_damaged_zip_is_reported(self):
        with pytest.raises(CdrParseError):
            parse(b"PK\x03\x04" + b"\x00" * 40)

    def test_errors_share_a_base_class(self):
        assert issubclass(CdrUnsupportedFormat, CdrError)
        assert issubclass(CdrParseError, CdrError)
