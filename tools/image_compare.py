"""Perceptual image comparison for doc regeneration.

When matplotlib renders the same figure across versions or platforms,
the resulting PNG bytes often differ slightly (anti-aliasing, font
hinting, library upgrades) even though the image looks identical.

This module provides a fast byte-level check followed by a perceptual
fallback so that ``make docs`` only rewrites images that actually
changed, keeping ``git status`` clean.
"""

import io
from pathlib import Path

import numpy as np
from PIL import Image

# Mean absolute per-channel difference (0-255 scale) below which two
# images are considered identical. Anti-aliasing drift typically stays
# well under 1.0; real content changes produce values in the tens.
MAX_MEAN_DIFF: float = 1.0

# Maximum fraction of pixels allowed to differ by more than
# ``pixel_tolerance`` in any single channel. Anti-aliasing may shift a
# thin edge by a few levels, but the affected pixel count is tiny.
MAX_CHANGED_FRACTION: float = 0.001

# Per-channel difference (0-255) below which a pixel is considered
# unchanged. Anything above counts toward ``MAX_CHANGED_FRACTION``.
PIXEL_TOLERANCE: int = 3


def _load_array(source: Path | bytes) -> np.ndarray:
    """Load an image (from a path or raw bytes) as an int16 RGBA array."""
    if isinstance(source, (str, Path)):
        with Image.open(source) as im:
            return np.asarray(im.convert("RGBA"), dtype=np.int16)
    with Image.open(io.BytesIO(source)) as im:
        return np.asarray(im.convert("RGBA"), dtype=np.int16)


def bytes_identical(existing_path: Path, new_bytes: bytes) -> bool:
    """Exact byte-level equality check (fast path)."""
    try:
        with open(existing_path, "rb") as f:
            return f.read() == new_bytes
    except OSError:
        return False


def visually_similar(
    existing_path: Path,
    new_bytes: bytes,
    *,
    max_mean_diff: float = MAX_MEAN_DIFF,
    max_changed_fraction: float = MAX_CHANGED_FRACTION,
    pixel_tolerance: int = PIXEL_TOLERANCE,
) -> bool:
    """Return True if two images are visually indistinguishable.

    Two independent criteria must both hold:

    1. The mean absolute per-channel difference is below
       ``max_mean_diff`` (catches global drift).
    2. The fraction of pixels differing by more than ``pixel_tolerance``
       in any channel is below ``max_changed_fraction`` (catches local
       changes that a low mean might mask).

    Images with different dimensions are never considered similar.
    Any decode failure returns False so the caller falls back to
    overwriting.
    """
    try:
        existing = _load_array(existing_path)
        new = _load_array(new_bytes)
    except Exception:
        return False

    if existing.shape != new.shape:
        return False

    diff = np.abs(existing - new)
    if diff.mean() >= max_mean_diff:
        return False

    per_pixel_max = diff.max(axis=-1)
    changed = per_pixel_max > pixel_tolerance
    return bool(changed.mean() < max_changed_fraction)


def looks_identical(existing_path: Path, new_bytes: bytes) -> bool:
    """Byte-level then perceptual comparison.

    Returns True when the existing file should be left untouched.
    """
    return bytes_identical(existing_path, new_bytes) or visually_similar(
        existing_path, new_bytes
    )
