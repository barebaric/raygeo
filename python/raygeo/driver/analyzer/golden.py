"""Golden-file harness: listings captured once, compared forever.

A golden test runs a deterministic scenario (a session against a
firmware emulator), records the traffic, and compares the decoded
listing against a committed fixture.  Captures from real hardware
can be dropped in as fixtures the same way: replay them here and
protocol changes that alter the wire behavior fail loudly.

Fixtures are two files per scenario: ``<name>.listing`` (the
compared, timestamp-free listing) and ``<name>.jsonl`` (the full
capture with timings, for humans and for replay).  Regenerate or
create them with ``RAYDRIVER_REGEN_GOLDEN=1``.
"""

from __future__ import annotations

import difflib
import os
from pathlib import Path

from raygeo.driver.analyzer.capture import Capture
from raygeo.driver.analyzer.decode import Decoder, get_decoder, render_listing

REGEN = os.environ.get("RAYDRIVER_REGEN_GOLDEN", "") == "1"


def assert_capture_matches_golden(
    name: str,
    capture: Capture,
    *,
    golden_dir,
    decoder: Decoder | None = None,
) -> None:
    """Assert *capture*'s listing equals the golden fixture *name*.

    Missing fixtures are written on first run; with
    ``RAYDRIVER_REGEN_GOLDEN=1`` they are rewritten instead of
    compared.
    """
    decoder = decoder or get_decoder(capture.protocol)
    listing = render_listing(capture, decoder, with_times=False)
    base = Path(golden_dir) / name
    listing_path = base.with_name(base.name + ".listing")
    jsonl_path = base.with_name(base.name + ".jsonl")
    if REGEN or not listing_path.exists():
        listing_path.parent.mkdir(parents=True, exist_ok=True)
        listing_path.write_text(listing, encoding="utf-8")
        capture.save(jsonl_path)
        return
    expected = listing_path.read_text(encoding="utf-8")
    if listing != expected:
        diff = "\n".join(
            list(
                difflib.unified_diff(
                    expected.splitlines(),
                    listing.splitlines(),
                    "golden",
                    "actual",
                    lineterm="",
                )
            )[:40]
        )
        raise AssertionError(
            f"decoded traffic for {name!r} changed:\n{diff}\n\n"
            "If this change is intended, regenerate the fixtures "
            "with RAYDRIVER_REGEN_GOLDEN=1 and review the diff."
        )
