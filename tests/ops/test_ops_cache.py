"""Tests for the ops-layer cache contract.

The ops layer owns the ``Cacheable`` trait that each assembler
(and transformer, encoder) implements. These tests exercise the
contract directly: calling ``cache_key`` / ``restore_cache`` /
``store_cache`` on an ``Assembler`` wrapper.

- ``ContourSpec`` opts out (all three return ``None``).
- Constructing ``CacheKey`` and ``AssemblyOutput`` works.
"""

from raygeo.geo import Geometry
from raygeo.ops import Ops
from raygeo.ops.assembly import Assembler
from raygeo.ops.assembly.contour import ContourSpec
from raygeo.ops.cache import AssemblyOutput, CacheKey
from raygeo.ops.part import Part


def _make_part() -> Part:
    g = Geometry()
    g.move_to(0, 0)
    g.line_to(10, 0)
    g.line_to(10, 10)
    g.line_to(0, 10)
    g.line_to(0, 0)
    return Part(geometry=g, size_mm=(10.0, 10.0))


# ── ContourSpec opts out of caching ───────────────────────────────


def test_contour_cache_key_returns_none():
    """ContourSpec.cache_key returns None (opts out)."""
    spec = ContourSpec()
    asm = Assembler(spec)
    assert asm.cache_key(_make_part(), "any-tag") is None


def test_contour_restore_cache_returns_none():
    """ContourSpec.restore_cache returns None unconditionally."""
    spec = ContourSpec()
    asm = Assembler(spec)
    dummy = AssemblyOutput(Ops(), False, None)
    assert asm.restore_cache(dummy) is None


def test_contour_store_cache_returns_none():
    """ContourSpec.store_cache returns None unconditionally."""
    spec = ContourSpec()
    asm = Assembler(spec)
    assert asm.store_cache(Ops(), False, None, _make_part()) is None


# ── CacheKey construction ─────────────────────────────────────────


def test_cache_key_round_trip():
    """CacheKey preserves tag and payload_hash."""
    k = CacheKey("my-tag", 123456789)
    assert k.tag == "my-tag"
    assert k.payload_hash == 123456789


def test_cache_key_repr():
    """CacheKey repr includes tag and hash."""
    k = CacheKey("k1", 42)
    r = repr(k)
    assert "k1" in r
    assert "42" in r


# ── AssemblyOutput construction ───────────────────────────────────


def test_assembly_output_defaults():
    """AssemblyOutput constructor defaults is_scalable to False."""
    ao = AssemblyOutput(Ops())
    assert ao.is_scalable is False
    assert ao.source_dimensions is None
    assert ao.cleared_fragments is None


def test_assembly_output_with_fragments():
    """AssemblyOutput with cleared fragments round-trips
    correctly."""
    frags = [[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]]
    ao = AssemblyOutput(Ops(), True, (100.0, 200.0), frags)
    assert ao.is_scalable is True
    assert ao.source_dimensions == (100.0, 200.0)
    assert ao.cleared_fragments == frags
