#!/usr/bin/env python3
"""Generate stylized PNG screenshots under docs/images/ for README and guides.

Requires Pillow: pip install pillow
  (or: pip install --target .docgen_pillow pillow && PYTHONPATH=.docgen_pillow python3 ...)

This renders a visual approximation of the TUI, not a live terminal capture.
"""

from __future__ import annotations

import os
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError as e:
    raise SystemExit(
        "Pillow is required: pip install pillow\n"
        "  or: pip install --target .docgen_pillow pillow && PYTHONPATH=.docgen_pillow python3 scripts/generate_doc_screenshots.py"
    ) from e

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "images"

BG = (12, 12, 12)
FG = (230, 230, 230)
CYAN = (0, 200, 220)
YELLOW = (220, 220, 100)
BLUE = (120, 160, 255)
GREEN = (120, 220, 160)
GRAY = (100, 100, 100)
ORANGE = (255, 165, 0)
BLACK = (0, 0, 0)


def _font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    for path in (
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    ):
        if os.path.isfile(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def draw_board(path: Path) -> None:
    w, h = 880, 520
    im = Image.new("RGB", (w, h), BG)
    dr = ImageDraw.Draw(im)
    mono = _font(14)
    small = _font(12)

    today = "2026-04-20"
    header = f" TaskWAL  —  today {today}  |  Done: {today} (local) "
    dr.rectangle([4, 4, w - 5, 28], outline=CYAN, width=1)
    dr.text((10, 8), header, fill=CYAN, font=mono)

    col_w = (w - 24) // 3
    y0 = 36
    col_h = h - y0 - 120
    titles = [("TODO", YELLOW, 2), ("DOING", BLUE, 1), ("DONE", GREEN, 1)]
    rows_todo = [
        (" o [01ABC123] Write API docs", False),
        (" o [01DEF456] Review PR", True),
    ]
    rows_doing = [(" * [01GHI789] Integrate auth", False)]
    rows_done = [(" x [01JKL012] Initial scaffold", False)]

    def col_block(
        x: int,
        title: str,
        border: tuple[int, int, int],
        rows: list[tuple[str, bool]],
        selected: bool,
    ) -> None:
        dr.rectangle([x, y0, x + col_w - 4, y0 + col_h], outline=border, width=2 if selected else 1)
        dr.text((x + 8, y0 + 4), f" {title} ({len(rows)}) ", fill=border, font=small)
        yy = y0 + 28
        for line, is_sel in rows:
            if is_sel:
                bbox = dr.textbbox((x + 10, yy), line, font=mono)
                dr.rectangle([bbox[0] - 2, bbox[1] - 1, bbox[2] + 2, bbox[3] + 1], fill=ORANGE)
                dr.text((x + 10, yy), line, fill=BLACK, font=mono)
            else:
                dr.text((x + 10, yy), line, fill=FG, font=mono)
            yy += 20

    col_block(8, "TODO", titles[0][1], rows_todo, True)
    col_block(8 + col_w, "DOING", titles[1][1], rows_doing, False)
    col_block(8 + 2 * col_w, "DONE", titles[2][1], rows_done, False)

    fy = y0 + col_h + 8
    dr.rectangle([4, fy, w - 5, fy + 32], outline=GRAY, width=1)
    dr.text((10, fy + 8), "> ", fill=GRAY, font=mono)
    dr.text((10, fy + 52), " command (press :) ", fill=GRAY, font=small)
    hint = " s start | d done | b back | a Done view | Tab | g stats | q | : command | Esc "
    dr.text((10, h - 28), hint, fill=GRAY, font=small)

    im.save(path, "PNG")


def draw_stats(path: Path) -> None:
    w, h = 640, 480
    im = Image.new("RGB", (w, h), BG)
    dr = ImageDraw.Draw(im)
    mono = _font(15)
    small = _font(13)

    title = " Stats (all-time) | g / Esc back | q quit "
    dr.rectangle([4, 4, w - 5, 28], outline=CYAN, width=1)
    dr.text((10, 8), title, fill=CYAN, font=small)

    body = """
  Total completed   : 42

  Avg cycle time    : 2.3 d

  Avg lead time     : 4.1 d

  Avg per active day: 3.2 tasks

  Streak (local)    : 5 d

  Recent days:
    2026-04-20 -> 4 done
    2026-04-19 -> 2 done
    2026-04-18 -> 5 done
""".strip(
        "\n"
    )
    y = 44
    for line in body.split("\n"):
        dr.text((8, y), line, fill=FG, font=mono)
        y += 22

    im.save(path, "PNG")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    draw_board(OUT / "board.png")
    draw_stats(OUT / "stats.png")
    print(f"Wrote {OUT / 'board.png'}")
    print(f"Wrote {OUT / 'stats.png'}")


if __name__ == "__main__":
    main()
