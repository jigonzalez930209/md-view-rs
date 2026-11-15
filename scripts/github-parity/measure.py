#!/usr/bin/env python3
"""Measure preview snapshots against GitHub's own render.

Usage:
    python3 -I scripts/github-parity/measure.py <github-ref.png> <math-ref.png>

The reference screenshots are produced as described in docs/github-parity.md: the
render_snapshots.rs and media_snapshots.rs documents are rendered by the GitHub
Markdown API, wrapped in github-markdown-css 5.8.1 (and MathJax 3.2.2 for math) at a
520 px, 14 px column, and captured with headless Chrome at 760 px.

The script prints, for both renders, the vertical offset between each list marker
and the text of its first line, and the size of the display and inline formulas.
Requires Pillow.
"""

import sys

from PIL import Image

PREVIEW = "crates/mdview-render/tests/snapshots/preview-light.png"
MEDIA = "crates/mdview-render/tests/snapshots/media-light.png"


def groups(rows):
    out = []
    for y in rows:
        if out and y - out[-1][-1] <= 1:
            out[-1].append(y)
        else:
            out.append([y])
    return out


def markers(path, y0, y1):
    """Marker center minus the text x-height band center, per list row."""
    image = Image.open(path).convert("L")
    px = image.load()
    rows = [y for y in range(y0, y1) if any(px[x, y] < 235 for x in range(125, 142))]
    result = []
    for blob in groups(rows):
        center = (blob[0] + blob[-1]) / 2
        density = {
            y: sum(px[x, y] < 170 for x in range(148, 260))
            for y in range(int(center) - 14, int(center) + 14)
        }
        peak = max(density.values()) or 1
        band = [y for y, value in density.items() if value >= peak * 0.35]
        result.append(round(center - (band[0] + band[-1]) / 2, 1))
    return result


def ink(path, box):
    image = Image.open(path).convert("L").crop(box)
    px = image.load()
    width, height = image.size
    points = [(x, y) for y in range(height) for x in range(width) if px[x, y] < 140]
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    return max(xs) - min(xs) + 1, max(ys) - min(ys) + 1


def main():
    github, math = sys.argv[1], sys.argv[2]
    print("list marker offsets (px), GitHub:", markers(github, 290, 540))
    print("list marker offsets (px), preview:", markers(PREVIEW, 265, 520))
    print("display formula (w, h), GitHub:", ink(math, (200, 112, 480, 190)))
    print("display formula (w, h), preview:", ink(MEDIA, (200, 105, 560, 185)))


if __name__ == "__main__":
    main()
