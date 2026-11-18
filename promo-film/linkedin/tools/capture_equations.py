#!/usr/bin/env python3
"""Capture the formula editor footage for one theme.

Usage: capture_equations.py <display> <outdir>
Run after `launch.sh <theme> <display> <workdir> tools/docs/equations.md`. Window geometry is
the 1920 x 1000 capture at 150 % UI scale. Each recording writes <name>.json with the pointer
and key events, timed from the first recorded frame, for syncing tags and captions.
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from xdrive import Display  # noqa: E402

disp, out = sys.argv[1:3]
out = Path(out)
out.mkdir(parents=True, exist_ok=True)
d = Display(disp)

# Dialog source pane: first character at x 391, rows at y 448 / 476, 11.72 px per character
# (measured from pixels; the dialog editor is narrower than the document editor).
SRC_X, CHAR, ROWS = 391, 11.72, (448, 476)
# Reading view: the Schrödinger equation; edit mode: equation buttons and the last line.
FORMULA = (1120, 308)
GALLERY_BUTTON, LAST_LINE = (840, 136), (600, 713)
PYTHAGORAS = (930, 785)
FRAC_BUTTON, SQRT_BUTTON = (355, 266), (426, 266)
REST = (1500, 720)


def col(c, row=0):
    return SRC_X + (c + 0.5) * CHAR, ROWS[row]


class Log:
    """Events timed from the first frame; ffmpeg's first frame lands about 0.6 s after start."""

    def __init__(self):
        self.t0 = time.monotonic() + 0.6
        self.events = []

    def mark(self, kind, **data):
        self.events.append(dict(t=round(time.monotonic() - self.t0, 3), kind=kind, **data))

    def save(self, name):
        (out / f"{name}.json").write_text(json.dumps(self.events, indent=1))


def dwell(log, label, x, y, glide=0.35, hold=0.75):
    d.glide(x, y, glide)
    log.mark("hover", label=label, x=x, y=y)
    time.sleep(hold)


d.focus()
d.move(*REST)
d.shot(out / "eq-01-view.png", 1.0)

# ---- double-click a formula in the reading view, then read it part by part
SCHRODINGER = [("ħ", 3, 0), ("∂/∂t", 8, 0), ("t", 31, 0), ("Ψ", 34, 0), ("ħ²", 9, 1), ("m", 17, 1),
               ("∇²", 21, 1), ("V", 34, 1), ("Ψ", 37, 1)]


def hover_take():
    log = Log()
    time.sleep(0.6)
    d.glide(*FORMULA, 0.7)
    time.sleep(0.3)
    log.mark("dblclick", x=FORMULA[0], y=FORMULA[1])
    d.click(*FORMULA, count=2, wait=0.9)
    log.mark("dialog")
    for label, c, row in SCHRODINGER:
        dwell(log, label, *col(c, row))
    log.save("eq-hover")


d.record(out / "eq-hover.mp4", 13.6, hover_take)
d.shot(out / "eq-04-dialog-hover.png", 0.2)
d.glide(*col(8), 0.3)
d.shot(out / "eq-05-hover-frac.png", 0.8)
d.key("Escape", 0.8)

# ---- edit mode, the equation gallery
d.key("ctrl+e", 1.0)
d.click(*LAST_LINE)
d.key("ctrl+End")
d.move(*REST)
d.shot(out / "eq-02-edit.png", 0.6)


def gallery_take():
    log = Log()
    time.sleep(0.5)
    d.glide(*GALLERY_BUTTON, 0.6)
    log.mark("click", target="gallery")
    d.click(*GALLERY_BUTTON, wait=0.9)
    d.glide(*PYTHAGORAS, 0.7)
    time.sleep(0.45)
    log.mark("click", target="pythagoras")
    d.click(*PYTHAGORAS, wait=0.2)
    d.glide(*REST, 0.6)
    log.mark("inserted")
    time.sleep(1.2)
    log.save("eq-gallery")


d.click(*GALLERY_BUTTON, wait=1.0)
d.move(*REST)
d.shot(out / "eq-03-gallery.png", 0.5)
d.key("Escape", 0.6)
d.record(out / "eq-gallery.mp4", 6.4, gallery_take)
d.key("ctrl+z", 0.6)

# ---- build a formula from an empty dialog; the take ends empty, as it began, so it loops
d.click(*LAST_LINE)
d.key("ctrl+End")
d.key("alt+equal", 1.0)
d.move(*REST)
d.shot(out / "eq-06-empty.png", 0.4)
QUADRATIC = [("−b ± …", 14), ("√", 19), ("²", 25), ("a", 34)]


def build_take():
    log = Log()
    time.sleep(0.7)
    log.mark("type", text="x = ")
    d.text("x = ", 0.3)
    d.glide(*FRAC_BUTTON, 0.55)
    log.mark("click", target="fraction")
    d.click(*FRAC_BUTTON, wait=0.45)
    log.mark("type", text="-b \\pm ")
    d.text("-b \\pm ", 0.3)
    d.glide(*SQRT_BUTTON, 0.45)
    log.mark("click", target="root")
    d.click(*SQRT_BUTTON, wait=0.45)
    log.mark("type", text="b^2-4ac")
    d.text("b^2-4ac", 0.35)
    log.mark("key", key="Tab")
    d.key("Tab", 0.35)
    log.mark("type", text="2a")
    d.text("2a", 0.8)
    for label, c in QUADRATIC:
        dwell(log, label, *col(c), glide=0.4, hold=0.85)
    d.glide(*REST, 0.6)
    time.sleep(0.5)
    log.mark("clear")
    d.key("ctrl+a", 0.15)
    d.key("Delete", 0.9)
    log.save("eq-build")


d.record(out / "eq-build.mp4", 17.5, build_take)
d.key("Escape", 0.6)
print("captured", out)
