"""Variant A — vertical hook (1080 x 1920, 22.5 s) for Reels, Shorts, and TikTok.

Cold open on the payoff: the LaTeX source above, the formula it draws below, both cut from one
real recording of the formula editor, so the hover highlight is visible in the first frame.
Then the build from palette blocks, the gallery, a beat-cut montage of the app, and the end card.
Sections land on beats of "Chronos" (120 BPM; beats every 0.5 s with media-start 0.125).
Text stays inside the strictest social safe area (TikTok, Reels, and Shorts together): x 120-888
and y 288-1248, narrowing to x 120-780 below y 840. Footage panels may use the full width.
Run from the project root: python3 tools/gen.py
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT.parent))
import kit  # noqa: E402

W, H = 1080, 1920
GROUND = "#0d1117"
CAP = "assets/cap/dark"
hover = kit.events(ROOT, "dark", "eq-hover")
build = kit.events(ROOT, "dark", "eq-build")
gallery = kit.events(ROOT, "dark", "eq-gallery")

HOOK, ANSWER, BUILD, GALLERY, MONTAGE, END, TOTAL = 0.0, 3.5, 7.0, 12.5, 15.5, 18.5, 22.5

# Regions of the 1920 x 1000 capture.
PREVIEW = (972, 450, 632, 330)
SOURCE_TWO_ROWS = (316, 436, 630, 58)
SOURCE_ONE_ROW = (316, 436, 630, 30)
PALETTE = (316, 236, 990, 64)
POPUP = (822, 156, 646, 748)
WINDOW = (0, 0, 1920, 1000)

html, T, audio = [], [], []
a = T.extend

# ---- hook + answer: one take of the hover recording, cropped twice
hover_start = kit.at(hover, "dialog") - 0.15
take = (HOOK, BUILD - HOOK, hover_start, 1)
html.append('<div class="lbl" id="lbl-src" style="top:560px">LaTeX</div>')
html.append(kit.panel("h-src", f"{CAP}/eq-hover.mp4", SOURCE_TWO_ROWS, 1.62, 30, 604, video=take))
html.append('<div class="lbl" id="lbl-out" style="top:730px">what it draws</div>')
html.append(kit.panel("h-out", f"{CAP}/eq-hover.mp4", PREVIEW, 1.62, 28, 774, video=(*take[:3], 2)))
a(kit.show("h-src", HOOK) + kit.show("h-out", HOOK))
a(['tl.set("#lbl-src, #lbl-out", { opacity: 1 }, 0);'])
a(kit.hide("h-src", BUILD) + kit.hide("h-out", BUILD))
a(['tl.set("#lbl-src, #lbl-out", { opacity: 0 }, %.2f);' % BUILD])

# ---- build: palette, source, and formula from the build recording
build_start = kit.at(build, "type") - 0.25
take = (BUILD, GALLERY - BUILD, build_start, 3)
html.append('<div class="lbl" id="lbl-pal" style="top:560px">blocks</div>')
html.append(kit.panel("b-pal", f"{CAP}/eq-build.mp4", PALETTE, 1.04, 25, 604, video=take, radius=14))
html.append(kit.panel("b-src", f"{CAP}/eq-build.mp4", SOURCE_ONE_ROW, 1.62, 30, 704, video=(*take[:3], 4), radius=14))
html.append(kit.panel("b-out", f"{CAP}/eq-build.mp4", PREVIEW, 1.62, 28, 790, video=(*take[:3], 5)))
for pid in ("b-pal", "b-src", "b-out"):
    a(kit.show(pid, BUILD) + kit.hide(pid, GALLERY))
a(['tl.set("#lbl-pal", { opacity: 1 }, %.2f);' % BUILD, 'tl.set("#lbl-pal", { opacity: 0 }, %.2f);' % GALLERY])

# ---- gallery
take = (GALLERY, MONTAGE - GALLERY, kit.at(gallery, "click", target="gallery") - 0.1, 6)
html.append(kit.panel("g-pop", f"{CAP}/eq-gallery.mp4", POPUP, 1.4, (W - 646 * 1.4) / 2, 560, video=take))
a(kit.show("g-pop", GALLERY, "rise") + kit.hide("g-pop", MONTAGE))

# ---- montage: four beat cuts of the app
STILLS = ["01-view-top", "11-split-top", "14-palette-mode", "16-search"]
for i, name in enumerate(STILLS):
    t = MONTAGE + i * 0.75
    html.append(kit.panel(f"m{i}", f"{CAP}/{name}.png", WINDOW, 1000 / 1920, 40, 600, radius=16))
    a(kit.show(f"m{i}", t) + kit.hide(f"m{i}", t + 0.75))

# ---- captions
CAPS = [
    (HOOK, ANSWER, ["What does", "this LaTeX draw?"]),
    (ANSWER, BUILD, ["Point at it.", "It lights up."]),
    (BUILD, GALLERY, ["Build it", "from blocks."]),
    (GALLERY, MONTAGE, ["Or start", "from a classic."]),
    (MONTAGE, END, ["Markdown.", "Native. Fast."]),
]
for i, (t0, t1, lines) in enumerate(CAPS):
    html.append(kit.caption(f"cap{i}", lines))
    # Frame 0 is the thumbnail: the opening caption is already set, not animating in.
    a(kit.caption_in(f"cap{i}", t0) if i else ['tl.set("#cap0", { opacity: 1 }, 0);'])
    a(kit.caption_out(f"cap{i}", t1))

# ---- end card
html.append(f"""<div class="end" id="end">
        <img class="elogo" src="assets/images/logo.png" alt="MD View app icon" />
        <div class="ew">MD View</div>
        <div class="ecta">Free &amp; open source</div>
        <div class="eos">macOS · Windows · Linux</div>
        <div class="eurl">github.com/<br />jigonzalez930209/md-view-rs</div>
      </div>""")
a([f'tl.set("#end", {{ opacity: 1 }}, {END});',
   f'tl.fromTo("#end", {{ clipPath: "inset(100% 0 0 0)" }}, {{ clipPath: "inset(0% 0 0 0)", duration: 0.45, ease: "power3.inOut" }}, {END});',
   f'tl.fromTo(".elogo", {{ scale: 0.6, opacity: 0 }}, {{ scale: 1, opacity: 1, duration: 0.4, ease: "back.out(1.6)" }}, {END + 0.35});',
   f'tl.fromTo(".ew", {{ opacity: 0, clipPath: "inset(0 50% 0 50%)" }}, {{ opacity: 1, clipPath: "inset(0 0% 0 0%)", duration: 0.45, ease: "power3.out" }}, {END + 0.6});',
   f'tl.fromTo(".ecta, .eos, .eurl", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.2, stagger: 0.12 }}, {END + 0.95});'])

css = f"""
    #root {{ background: {GROUND}; }}
    .cap {{ position: absolute; left: 120px; top: 300px; width: 768px; font-size: 80px; font-weight: 900; letter-spacing: -0.035em; line-height: 1.02; color: #ffffff; opacity: 0; }}
    .cap .ln {{ white-space: nowrap; }}
    .cap span {{ display: inline-block; }}
    .cap .ln:last-child span {{ color: {kit.BRAND.replace('0b6bd6', '4c9bff')}; }}
    .lbl {{ position: absolute; left: 120px; font-size: 30px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; color: {kit.ACCENT}; opacity: 0; }}
    .panel {{ box-shadow: 0 24px 60px rgba(0,0,0,0.55); outline: 1px solid #30363d; }}
    .end {{ position: absolute; inset: 0; background: #ffffff; opacity: 0; }}
    .elogo {{ position: absolute; left: 120px; top: 330px; width: 200px; height: 200px; }}
    .ew, .ecta, .eos, .eurl {{ position: absolute; left: 120px; white-space: nowrap; }}
    .ew {{ top: 560px; font-size: 150px; font-weight: 900; letter-spacing: -0.045em; line-height: 1.1; color: #0a1220; }}
    .ecta {{ top: 880px; font-size: 52px; font-weight: 800; color: #0a1220; }}
    .eos {{ top: 960px; font-size: 40px; font-weight: 500; color: #3b4658; }}
    .eurl {{ top: 1150px; width: 660px; font-family: "JetBrains Mono", monospace; font-size: 28px; line-height: 1.3; white-space: normal; color: #0b5cb8; }}
"""
(ROOT / "compositions/film.html").write_text(kit.composition("film", W, H, css, "\n      ".join(html), T))

# ---- sound: music on the beat grid, clicks synced to the recordings
audio.append(f'      <audio id="music" class="clip" data-start="0" data-duration="{TOTAL}" data-media-start="0.125" '
             f'data-track-index="10" data-volume="0.8" data-fade-out="1.5" src="assets/audio/chronos.mp3"></audio>')
audio.append(kit.sfx("s-open", 0.0, "impact-bass-1", 0.45, 1.6))
for i, e in enumerate(e for e in hover if e["kind"] == "hover"):
    t = HOOK + e["video_t"] - hover_start
    if t < BUILD - 0.2:
        audio.append(kit.sfx(f"s-h{i}", t + 0.05, "click-soft", 0.35, 0.2, 12))
for i, e in enumerate(build):
    t = BUILD + e["video_t"] - build_start
    if e["kind"] in ("click", "key") and t < GALLERY:
        audio.append(kit.sfx(f"s-b{i}", t + 0.03, "click", 0.45, 0.2, 13))
    if e["kind"] == "type" and t < GALLERY:
        audio.append(kit.sfx(f"s-t{i}", t, "typing", 0.3, 0.6, 14))
for i, t in enumerate([BUILD, GALLERY, MONTAGE]):
    audio.append(kit.sfx(f"s-w{i}", t - 0.05, "whoosh-short", 0.35, 0.57, 15))
for i in range(1, 4):
    audio.append(kit.sfx(f"s-m{i}", MONTAGE + i * 0.75, "key-press", 0.35, 0.2, 16))
audio.append(kit.sfx("s-end", END + 0.35, "pop", 0.5, 0.7, 17))
body = (f'      <div id="film" data-composition-id="film" data-composition-src="compositions/film.html" data-start="0" '
        f'data-duration="{TOTAL}" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>')
(ROOT / "index.html").write_text(kit.index_html(W, H, TOTAL, body, audio, GROUND))
print("ok")
