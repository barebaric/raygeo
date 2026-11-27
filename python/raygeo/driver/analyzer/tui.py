"""Interactive terminal monitor for live captures.

A thin curses front end over :mod:`raygeo.driver.analyzer.capture` and
:mod:`raygeo.driver.analyzer.decode`: it tails the recorder's capture
while the session runs, decodes frames with the protocol decoder,
and saves the capture on demand.  Everything testable lives in the
modules behind it.

Keys: ``q`` quit, ``p`` pause/resume the view, ``s`` save, ``f``
cycle the direction filter (all/tx/rx), arrows/PgUp/PgDn/Home/End
scroll.  The view follows the tail unless you scroll up.
"""

from __future__ import annotations

import asyncio
import curses
from collections.abc import Awaitable, Callable

from raygeo.driver.analyzer.capture import RX, TX, Recorder
from raygeo.driver.analyzer.decode import Decoder, format_frame

_FILTERS = ("all", "tx", "rx")


class AnalyzerTui:
    """Curses monitor over a :class:`Recorder` capture."""

    def __init__(
        self,
        recorder: Recorder,
        decoder: Decoder,
        *,
        source: str = "",
        save_path: str | None = None,
    ):
        self.recorder = recorder
        self.decoder = decoder
        self.decoder.reset()
        self.source = source
        self.save_path = save_path
        self.paused = False
        self.filter = "all"
        self.status = ""
        self._follow = True
        self._top = 0

    # -- view helpers ------------------------------------------------

    def _visible_frames(self):
        return [
            frame
            for frame in self.recorder.capture.frames
            if self.filter == "all"
            or (self.filter == "tx" and frame.direction == TX)
            or (self.filter == "rx" and frame.direction == RX)
        ]

    def _rows(self, width: int) -> list[str]:
        rows = []
        frames = self.recorder.capture.frames
        for index, frame in enumerate(frames):
            if self.filter == "tx" and frame.direction != TX:
                continue
            if self.filter == "rx" and frame.direction != RX:
                continue
            row = format_frame(index, frame, self.decoder)
            rows.append(row[:width] if width else row)
        return rows

    # -- events ------------------------------------------------------

    def handle_key(self, key: int, height: int) -> bool:
        """Process one key; returns False when the TUI should quit."""
        visible = max(len(self._visible_frames()), 1)
        if key in (ord("q"), 27):
            return False
        if key == ord("p"):
            self.paused = not self.paused
        elif key == ord("s"):
            self.save()
        elif key == ord("f"):
            current = _FILTERS.index(self.filter)
            self.filter = _FILTERS[(current + 1) % len(_FILTERS)]
            self._top = 0
            self._follow = True
        elif key == curses.KEY_UP:
            self._top = max(0, self._top - 1)
            self._follow = False
        elif key == curses.KEY_DOWN:
            self._top += 1
            self._follow = False
        elif key == curses.KEY_PPAGE:
            self._top = max(0, self._top - (height - 2))
            self._follow = False
        elif key == curses.KEY_NPAGE:
            self._top += height - 2
            self._follow = False
        elif key == curses.KEY_HOME:
            self._top = 0
            self._follow = False
        elif key == curses.KEY_END:
            self._follow = True
        if not self._follow:
            self._top = min(self._top, max(visible - 1, 0))
        return True

    def save(self) -> None:
        if self.save_path:
            self.recorder.capture.save(self.save_path)
            self.status = f"saved {self.save_path}"

    # -- rendering ---------------------------------------------------

    def draw(self, stdscr) -> None:
        stdscr.erase()
        height, width = stdscr.getmaxyx()
        capture = self.recorder.capture
        header = (
            f" {self.source or capture.source}  "
            f"[{capture.protocol}]  {len(capture.frames)} frames  "
            f"filter:{self.filter}"
            f"{'  PAUSED' if self.paused else ''}  "
            f"{'  ' + self.status if self.status else ''}"
        )
        stdscr.addnstr(0, 0, header, width - 1, curses.A_REVERSE)
        rows = self._rows(width)
        if self._follow:
            self._top = max(len(rows) - (height - 2), 0)
        for line_index, row in enumerate(
            rows[self._top : self._top + height - 2]
        ):
            stdscr.addnstr(line_index + 1, 0, row, width - 1)
        footer = " q quit | p pause | s save | f filter | arrows scroll"
        stdscr.addnstr(height - 1, 0, footer, width - 1, curses.A_REVERSE)
        stdscr.refresh()

    async def run(
        self,
        stdscr,
        *,
        work: (
            Awaitable[object] | Callable[[], Awaitable[object]] | None
        ) = None,
        refresh: float = 0.05,
    ):
        """Run the monitor until quit.

        *work* is an optional coroutine (or zero-argument callable
        returning one) to run alongside; it is cancelled and awaited
        on quit, so the session tears down before the loop closes."""
        stdscr.nodelay(True)
        task = None
        if work is not None:
            coroutine = work() if callable(work) else work
            task = asyncio.ensure_future(coroutine)
        try:
            while True:
                height, _ = stdscr.getmaxyx()
                while True:
                    key = stdscr.getch()
                    if key == -1:
                        break
                    if key == curses.KEY_RESIZE:
                        pass
                    elif not self.handle_key(key, height):
                        return
                self.draw(stdscr)
                await asyncio.sleep(refresh)
        finally:
            if task is not None:
                task.cancel()
                await asyncio.gather(task, return_exceptions=True)


def run_tui(
    recorder: Recorder,
    decoder: Decoder,
    *,
    source: str = "",
    save_path: str | None = None,
    work: (Awaitable[object] | Callable[[], Awaitable[object]] | None) = None,
) -> None:
    """Run :class:`AnalyzerTui` under curses (needs a terminal)."""
    tui = AnalyzerTui(recorder, decoder, source=source, save_path=save_path)

    async def amain(stdscr):
        await tui.run(stdscr, work=work)

    def main(stdscr):
        asyncio.run(amain(stdscr))

    curses.wrapper(main)
