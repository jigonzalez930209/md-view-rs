"""Variant C — seamless loop (1080 x 1080) for X, LinkedIn autoplay, and the GitHub social preview.

One continuous real take of the formula editor (GitHub Light): blocks from the palette, the
LaTeX source, and the formula it draws, stacked as three crops of the same recording, with one
line of text. The take begins and ends on an empty dialog with the pointer at rest; the cut
points are chosen by pixel match (including the source caret's blink phase), so the last frame
flows into the first. No music, so the sound loops too; clicks and typing follow the take.
Run from the project root: python3 tools/gen.py
"""
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT.parent))
import kit  # noqa: E402

W = H = 1080
GROUND = "#f6f8fa"
VIDEO = "assets/cap/light/eq-build.mp4"
build = kit.events(ROOT, "light", "eq-build")
FPS = 30


def loop_points():
    """Start and end frames of the take whose images match best (an empty dialog at both ends)."""
    raw = subprocess.run(["ffmpeg", "-loglevel", "error", "-i", str(ROOT / VIDEO), "-vf",
                          "crop=1320:640:300:160,scale=330:160", "-f", "rawvideo", "-pix_fmt", "gray", "-"],
                         capture_output=True, check=True).stdout
    frames = np.frombuffer(raw, np.uint8).reshape(-1, 160, 330).astype(int)
    first, clear = kit.at(build, "type"), kit.at(build, "clear")
    starts = range(int((first - 0.9) * FPS), int((first - 0.15) * FPS))
    ends = range(int((clear + 0.6) * FPS), min(len(frames) - 1, int((clear + 2.0) * FPS)))
    diff, start, end = min((np.abs(frames[s] - frames[e]).mean(), s, e) for s in starts for e in ends)
    print(f"loop {start / FPS:.3f}-{end / FPS:.3f} s, mean frame difference {diff:.3f}")
    return start / FPS, end / FPS


START, END = loop_points()
TOTAL = round(END - START, 3)
take = (0.0, TOTAL, START, 1)

PALETTE = (316, 236, 990, 64)
SOURCE = (316, 436, 630, 30)
PREVIEW = (972, 450, 632, 330)
html = [
    '<div id="brand"><img src="assets/images/logo.png" alt="MD View app icon" /><span class="bn">MD View</span>'
    '<span class="bt">formula editor</span></div>',
    kit.panel("p-pal", VIDEO, PALETTE, 0.86, (W - 990 * 0.86) / 2, 228, video=take, radius=12),
    kit.panel("p-src", VIDEO, SOURCE, 1.36, (W - 630 * 1.36) / 2, 312, video=(*take[:3], 2), radius=12),
    kit.panel("p-out", VIDEO, PREVIEW, 1.36, (W - 632 * 1.36) / 2, 384, video=(*take[:3], 3), radius=18),
    '<div id="line"><span>Point at the LaTeX.</span> <span class="hi">See what it draws.</span></div>',
]
css = f"""
    #root {{ background: {GROUND}; }}
    .panel {{ opacity: 1; box-shadow: 0 18px 50px rgba(10,18,32,0.14), 0 0 0 1px rgba(10,18,32,0.08); }}
    #brand {{ position: absolute; left: 110px; top: 116px; display: flex; align-items: center; gap: 16px; white-space: nowrap; }}
    #brand img {{ width: 56px; height: 56px; }}
    #brand .bn {{ font-size: 42px; font-weight: 900; letter-spacing: -0.03em; color: #0a1220; }}
    #brand .bt {{ font-size: 32px; font-weight: 600; color: #57606a; }}
    #line {{ position: absolute; left: 110px; top: 880px; width: 860px; font-size: 44px; font-weight: 800; letter-spacing: -0.02em; line-height: 1.15; color: #0a1220; }}
    #line .hi {{ color: {kit.BRAND}; }}
"""
# Everything is on screen from the first frame and nothing enters or leaves: the take is the motion.
(ROOT / "compositions/film.html").write_text(kit.composition("film", W, H, css, "\n      ".join(html), []))

audio = []
for i, e in enumerate(build):
    t = e["video_t"] - START
    if not 0 <= t < TOTAL:
        continue
    if e["kind"] in ("click", "key"):
        audio.append(kit.sfx(f"s-c{i}", t + 0.03, "click", 0.45, 0.2, 12))
    elif e["kind"] == "type":
        audio.append(kit.sfx(f"s-t{i}", t, "typing", 0.3, min(0.6, TOTAL - t), 13))
    elif e["kind"] == "clear":
        audio.append(kit.sfx(f"s-x{i}", t + 0.15, "whoosh-short", 0.25, round(min(0.57, TOTAL - t - 0.15), 3), 14))
body = (f'      <div id="film" data-composition-id="film" data-composition-src="compositions/film.html" data-start="0" '
        f'data-duration="{TOTAL}" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>')
(ROOT / "index.html").write_text(kit.index_html(W, H, TOTAL, body, audio, GROUND))
print("ok")
