#!/usr/bin/env python3
"""Measure preview snapshots against GitHub's own render.

Usage:
    python3 -I scripts/github-parity/measure.py <github-ref.png> <math-ref.png> \
        [<github-ref16.png> [<parity-ref.png>]]

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
GITHUB_SCALE = "crates/mdview-render/tests/snapshots/preview-github-scale.png"
PARITY = "crates/mdview-render/tests/snapshots/github-parity.png"
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


def sections(path):
    """Heading-rule positions, section heights, and the alert bar of one render."""
    image = Image.open(path).convert("RGB")
    px = image.load()
    width, height = image.size
    background = px[width - 40, 10]

    def diff(color):
        return sum(abs(a - b) for a, b in zip(color, background))

    rules = [
        y
        for y in range(1, height - 1)
        if all(8 < diff(px[x, y]) < 120 for x in range(125, 635, 5))
        and diff(px[300, y - 1]) < 8
        and diff(px[300, y + 1]) < 8
    ]
    bar = [y for y in range(height) if px[122, y][2] > 150 and px[122, y][0] < 100]
    bar_runs = [len(run) for run in groups(bar) if len(run) > 8]
    return [b - a for a, b in zip(rules, rules[1:])], bar_runs


def ink_bands(path, x0=118, x1=642):
    """Vertical ink bands (top, height) of the content column: lines, rules, boxes."""
    image = Image.open(path).convert("RGB")
    px = image.load()
    width, height = image.size
    background = px[x0 + 2, height - 30]
    rows = [
        y
        for y in range(20, height - 20)
        if any(sum(abs(a - b) for a, b in zip(px[x, y], background)) > 40 for x in range(x0, x1, 2))
    ]
    runs = []
    for y in rows:
        if runs and y - runs[-1][1] <= 2:
            runs[-1][1] = y
        else:
            runs.append([y, y])
    return [(top, bottom - top + 1) for top, bottom in runs if bottom - top + 1 >= 3]


def compare_bands(reference, preview):
    github = ink_bands(reference)
    ours = ink_bands(preview)
    print(f"ink bands: GitHub {len(github)}, preview {len(ours)}")
    print(" #  GitHub(top,h)   preview(top,h)  top delta")
    for index, (left, right) in enumerate(zip(github, ours)):
        top_left = left[0] - github[0][0]
        top_right = right[0] - ours[0][0]
        print(f"{index:2d}  {top_left:5d},{left[1]:4d}     {top_right:5d},{right[1]:4d}     {top_right - top_left:+d}")


def main():
    github, math = sys.argv[1], sys.argv[2]
    print("list marker offsets (px), GitHub:", markers(github, 290, 540))
    print("list marker offsets (px), preview:", markers(PREVIEW, 265, 520))
    print("display formula (w, h), GitHub:", ink(math, (200, 112, 480, 190)))
    print("display formula (w, h), preview:", ink(MEDIA, (200, 105, 560, 185)))
    if len(sys.argv) > 3:
        # GitHub's real scale: a 16 px body against preview-github-scale.png.
        print("section heights / alert height (px), GitHub:", sections(sys.argv[3]))
        print("section heights / alert height (px), preview:", sections(GITHUB_SCALE))
    if len(sys.argv) > 4:
        # Every common element (fixtures/github-parity.md) at GitHub's 16 px scale.
        compare_bands(sys.argv[4], PARITY)


if __name__ == "__main__":
    main()
