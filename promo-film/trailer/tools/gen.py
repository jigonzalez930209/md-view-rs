"""Variant B — product trailer (1920 x 1080, 67.5 s) for YouTube, the website, and the README.

Chaptered and calmer than the social cuts: one large window card (GitHub Light) with a camera,
a numbered chapter title above it, and real recordings for every interaction. 01 Read, 02 Write,
03 Math (gallery, double-click + hover, build), 04 Find, 05 Watch (the pure-Rust video player
playing HEVC, Xvid AVI, and H.264 TS, then the PDF viewer and the CSV grid), 06 Fast (numbers
measured by the E2E run), then the download card. Music: "Chronos" twice, the second copy crossfaded on a bar.
Run from the project root: python3 tools/gen.py
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT.parent))
import kit  # noqa: E402

W, H = 1920, 1080
GROUND = "#eef2f7"
INK = "#0a1220"
CAP = "assets/cap/light"
CARD_S = 0.82
CARD_W, CARD_H = 1920 * CARD_S, 1000 * CARD_S
CARD_X, CARD_Y = (W - CARD_W) / 2, 214
DOC = (323, 101, 1572, 864)

gallery = kit.events(ROOT, "light", "eq-gallery")
hover = kit.events(ROOT, "light", "eq-hover")
build = kit.events(ROOT, "light", "eq-build")

TITLE, READ, WRITE, MATH, FIND, WATCH, FAST, END, TOTAL = 0, 4, 13, 21, 37, 43, 55, 63, 67.5
VERSION = "1.0.0-rc.8"
# video-play.mp4 (recorded by the trailer's capture notes in README.md): seconds into the
# recording where the film cuts in, where the PDF and CSV stills take over, and the clicks.
PLAY_IN, PDF_AT, CSV_AT = 1.2, 52.6, 53.8
PLAY_CLICKS = [1.5, 5.8, 6.8, 9.4, 10.4]
HOVER_AT, BUILD_AT = 26.5, 32.5


def cam(scale, fx, fy):
    """Camera that shows window point (fx, fy) at the card centre, at `scale` x the fitted size."""
    s = CARD_S * scale
    return {"x": round(CARD_W / 2 - fx * s, 1), "y": round(CARD_H / 2 - fy * s, 1), "scale": round(s, 4)}


def js(d):
    return "{ " + ", ".join(f"{k}: {v}" for k, v in d.items()) + " }"


FULL = cam(1.0, 960, 500)
T = []
a = T.extend


def move(c, t, d=0.8):
    a([f'tl.to(".cam", {{ ...{js(c)}, duration: {d}, ease: "power3.inOut" }}, {t});'])


def layer(lid, t0, t1=None):
    a([f'tl.set("#{lid}", {{ opacity: 1 }}, {t0});'])
    if t1 is not None:
        a([f'tl.set("#{lid}", {{ opacity: 0 }}, {t1});'])


def video_layer(lid, src, start, end, media_start, track):
    return (f'<video id="{lid}" class="clip lay vid" data-start="{start:.3f}" data-duration="{end - start:.3f}" '
            f'data-media-start="{media_start:.3f}" data-track-index="{track}" muted playsinline src="{src}"></video>')


def img_layer(lid, src, alt):
    return f'<img id="{lid}" class="lay" src="{src}" alt="{alt}" />'


dx, dy, dw, dh = DOC
layers = [
    img_layer("l-view", f"{CAP}/01-view-top.png", "MD View reading view"),
    f'<div id="l-doc" class="lay"><div class="docclip" style="left:{dx}px; top:{dy}px; width:{dw}px; height:{dh}px">'
    f'<img class="doc" src="{CAP}/doc-tall.png" alt="" /></div></div>',
    img_layer("l-edit", f"{CAP}/07-edit-top.png", "MD View editor"),
    img_layer("l-find", f"{CAP}/08b-find-regex.png", "Regex find"),
    img_layer("l-split", f"{CAP}/11-split-top.png", "Split view"),
    video_layer("l-splitv", f"{CAP}/split-scroll.mp4", 17.3, MATH, 0.333, 1),
    img_layer("l-eqedit", f"{CAP}/eq-02-edit.png", "Equation buttons"),
    video_layer("l-gal", f"{CAP}/eq-gallery.mp4", MATH, HOVER_AT, kit.at(gallery, "click", target="gallery") - 0.9, 2),
    video_layer("l-hov", f"{CAP}/eq-hover.mp4", HOVER_AT, BUILD_AT, kit.at(hover, "dblclick") - 1.2, 3),
    video_layer("l-bld", f"{CAP}/eq-build.mp4", BUILD_AT, FIND, kit.at(build, "type") - 0.3, 4),
    img_layer("l-pal", f"{CAP}/13-palette.png", "Command palette"),
    img_layer("l-palm", f"{CAP}/14-palette-mode.png", "Command palette filtered"),
    img_layer("l-srch", f"{CAP}/16-search.png", "Workspace search"),
    img_layer("l-thm", f"{CAP}/17b-settings-themes.png", "Theme list"),
    video_layer("l-play", f"{CAP}/video-play.mp4", WATCH, PDF_AT, PLAY_IN, 5),
    img_layer("l-pdf", f"{CAP}/viewer-pdf.png", "PDF viewer"),
    img_layer("l-csv", f"{CAP}/viewer-csv.png", "CSV grid"),
]

# ---- chapters: number + title + one line, top-left inside title-safe
CHAPTERS = [
    (READ, WRITE, "01", "Read", "GitHub-flavoured Markdown, LaTeX, and Mermaid, drawn natively."),
    (WRITE, MATH, "02", "Write", "A real code editor: regex find and replace, split view in sync."),
    (MATH, FIND, "03", "Math", "An equation gallery and a formula editor that explains its LaTeX."),
    (FIND, WATCH, "04", "Find", "Every command one keystroke away. Nine themes."),
    (WATCH, FAST, "05", "Watch", "Video with sound, PDF, and spreadsheets. Pure Rust, nothing to install."),
    (FAST, END, "06", "Fast", "Measured in the release test run, not promised."),
]
chapters = []
for i, (t0, t1, num, title, line) in enumerate(CHAPTERS):
    chapters.append(f'<div class="ch" id="ch{i}"><span class="num">{num}</span><span class="tt">{title}</span>'
                    f'<div class="cl">{line}</div></div>')
    a([f'tl.set("#ch{i}", {{ opacity: 1 }}, {t0});',
       f'tl.fromTo("#ch{i} .num, #ch{i} .tt", {{ y: 30, opacity: 0 }}, {{ y: 0, opacity: 1, duration: 0.45, ease: "power3.out", stagger: 0.08 }}, {t0});',
       f'tl.fromTo("#ch{i} .cl", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.3 }}, {t0 + 0.25});',
       f'tl.to("#ch{i}", {{ opacity: 0, duration: 0.2 }}, {t1 - 0.25});'])

# ---- title
# The wordmark is on screen at 0 s: frame 0 is the thumbnail.
a(['tl.fromTo("#t-word", { scale: 1.06 }, { scale: 1, duration: 1.4, ease: "power3.out" }, 0);',
   'tl.fromTo("#t-line span", { opacity: 0, filter: "blur(10px)" }, { opacity: 1, filter: "blur(0px)", duration: 0.25, stagger: 0.08 }, 0.6);',
   'tl.to("#title", { opacity: 0, y: -40, duration: 0.35, ease: "power2.in" }, 3.0);',
   # immediateRender off: the card stays hidden (CSS opacity 0) until it rises at 3.4 s.
   f'tl.fromTo("#card", {{ y: 700, opacity: 1 }}, {{ y: 0, opacity: 1, duration: 0.8, ease: "power3.out", immediateRender: false }}, 3.4);'])

# ---- 01 read: the page scrolls through the real document
layer("l-view", READ - 0.6)
layer("l-doc", READ - 0.6)
for t, y in [(READ + 0.6, 715), (READ + 3.4, 1421), (READ + 6.2, 2551)]:
    a([f'tl.to(".doc", {{ y: {-y}, duration: 1.0, ease: "power2.inOut" }}, {t});'])
move(cam(1.12, 1117, 520), READ + 0.6, 1.0)
# ---- 02 write
layer("l-edit", WRITE)
move(FULL, WRITE, 0.6)
layer("l-find", WRITE + 2.0)
move(cam(1.7, 1430, 260), WRITE + 2.0)
layer("l-split", WRITE + 4.3)
move(FULL, WRITE + 4.3, 0.6)
# ---- 03 math
layer("l-eqedit", MATH)
move(cam(1.25, 1050, 470), MATH + 0.6, 1.0)
move(FULL, HOVER_AT, 0.6)
dialog_in = HOVER_AT + kit.at(hover, "dialog") - (kit.at(hover, "dblclick") - 1.2)
move(cam(1.3, 960, 612), dialog_in, 0.9)
# ---- 04 find
layer("l-pal", FIND)
move(cam(1.25, 960, 470), FIND, 0.6)
layer("l-palm", FIND + 1.0)
layer("l-srch", FIND + 2.0)
move(cam(1.2, 960, 430), FIND + 2.0, 0.5)
move(cam(1.35, 1180, 470), FIND + 4.0, 0.5)
# ---- 05 watch: the recording plays at full size, then the PDF and CSV viewers
layer("l-thm", FIND + 4.0, WATCH)
move(FULL, WATCH, 0.6)
layer("l-pdf", PDF_AT)
move(cam(1.15, 1060, 420), PDF_AT, 0.5)
layer("l-csv", CSV_AT)
move(cam(1.25, 900, 360), CSV_AT, 0.5)
a([f'tl.to("#card", {{ y: 900, duration: 0.6, ease: "power3.in" }}, {FAST - 0.1});'])

# ---- 05 fast: measured numbers
# From the 1.0.0-rc.5 E2E run: median of the per-120-frame p50 (244 windows) and peak RSS.
STATS = [("5.4 ms", "median frame time across the full E2E run"), ("245 MiB", "peak memory with every feature exercised"),
         ("210,000", "lines open and editable in large-file mode"), ("0", "webviews, browsers, or JS engines inside")]
stats = []
for i, (big, small) in enumerate(STATS):
    x, y = 192 + (i % 2) * 780, 330 + (i // 2) * 290
    stats.append(f'<div class="st" id="st{i}" style="left:{x}px; top:{y}px"><div class="sb">{big}</div><div class="ss">{small}</div></div>')
    t = FAST + 0.6 + i * 1.2
    a([f'tl.set("#st{i}", {{ opacity: 1 }}, {t:.2f});',
       f'tl.fromTo("#st{i} .sb", {{ clipPath: "inset(0 100% 0 0)" }}, {{ clipPath: "inset(0 0% 0 0)", duration: 0.5, ease: "power3.out" }}, {t:.2f});',
       f'tl.fromTo("#st{i} .ss", {{ opacity: 0, y: 12 }}, {{ opacity: 1, y: 0, duration: 0.3 }}, {t + 0.25:.2f});',
       f'tl.to("#st{i}", {{ opacity: 0, duration: 0.2 }}, {END - 0.25});'])

# ---- end
a([f'tl.set("#end", {{ opacity: 1 }}, {END});',
   f'tl.fromTo(".elogo", {{ scale: 0.6, opacity: 0 }}, {{ scale: 1, opacity: 1, duration: 0.45, ease: "back.out(1.6)" }}, {END});',
   f'tl.fromTo(".ew", {{ opacity: 0, clipPath: "inset(0 50% 0 50%)" }}, {{ opacity: 1, clipPath: "inset(0 0% 0 0%)", duration: 0.5, ease: "power3.out" }}, {END + 0.3});',
   f'tl.fromTo(".ecta, .eos, .eurl", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.22, stagger: 0.12 }}, {END + 0.7});'])

html = f"""
      <div id="title"><div id="t-word">MD View</div>
        <div id="t-line"><span>A native Markdown editor.</span> <span>No browser inside.</span></div></div>
      {"".join(chapters)}
      <div id="card" style="left:{CARD_X:.1f}px; top:{CARD_Y}px; width:{CARD_W:.1f}px; height:{CARD_H:.1f}px">
        <div class="cam">{"".join(layers)}</div>
      </div>
      {"".join(stats)}
      <div id="end">
        <img class="elogo" src="assets/images/logo.png" alt="MD View app icon" />
        <div class="ew">MD View</div>
        <div class="ecta">Download {VERSION}</div>
        <div class="eos">macOS · Windows · Linux · free &amp; open source</div>
        <div class="eurl">{kit.REPO}</div>
      </div>"""

css = f"""
    #root {{ background: {GROUND}; color: {INK}; }}
    #title {{ position: absolute; left: 192px; top: 330px; }}
    #t-word {{ transform-origin: 0 50%; font-size: 210px; font-weight: 900; letter-spacing: -0.05em; line-height: 1; }}
    #t-line {{ margin-top: 28px; font-size: 54px; font-weight: 700; letter-spacing: -0.02em; color: #3b4658; }}
    #t-line span {{ display: inline-block; }}
    .ch {{ position: absolute; left: 192px; top: 108px; opacity: 0; white-space: nowrap; }}
    .ch .num {{ display: inline-block; font-size: 44px; font-weight: 900; color: {kit.ACCENT}; margin-right: 18px; }}
    .ch .tt {{ display: inline-block; font-size: 44px; font-weight: 900; letter-spacing: -0.02em; }}
    .ch .cl {{ display: inline-block; margin-left: 28px; font-size: 30px; font-weight: 500; color: #3b4658; }}
    #card {{ position: absolute; border-radius: 16px; overflow: hidden; background: #ffffff; opacity: 0;
            box-shadow: 0 30px 80px rgba(10,18,32,0.22), 0 0 0 1px rgba(10,18,32,0.08); }}
    .cam {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; transform-origin: 0 0; }}
    .lay {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; opacity: 0; }}
    .vid {{ opacity: 1; }}
    .docclip {{ position: absolute; overflow: hidden; }}
    .doc {{ position: absolute; left: 0; top: 0; width: 1572px; height: 3744px; }}
    .st {{ position: absolute; width: 720px; opacity: 0; }}
    .sb {{ font-size: 150px; font-weight: 900; letter-spacing: -0.05em; line-height: 1; color: {kit.BRAND}; }}
    .ss {{ margin-top: 12px; font-size: 32px; font-weight: 600; color: #3b4658; }}
    #end {{ position: absolute; inset: 0; opacity: 0; }}
    .elogo {{ position: absolute; left: {W / 2 - 90}px; top: 200px; width: 180px; height: 180px; }}
    .ew, .ecta, .eos, .eurl {{ position: absolute; left: 0; width: {W}px; text-align: center; white-space: nowrap; }}
    .ew {{ top: 410px; font-size: 150px; font-weight: 900; letter-spacing: -0.045em; line-height: 1.1; }}
    .ecta {{ top: 620px; font-size: 52px; font-weight: 800; }}
    .eos {{ top: 700px; font-size: 36px; font-weight: 500; color: #3b4658; }}
    .eurl {{ top: 790px; font-family: "JetBrains Mono", monospace; font-size: 32px; color: #0b5cb8; }}
"""
initial = f'gsap.set(".cam", {js(FULL)}); gsap.set(".doc", {{ y: 0 }});'
(ROOT / "compositions/film.html").write_text(kit.composition("film", W, H, css, html, T, initial))

# ---- sound
audio = [
    f'      <audio id="music" class="clip" data-start="0" data-duration="44" data-media-start="0.125" data-track-index="10" '
    f'data-volume="0.75" data-fade-out="2" src="assets/audio/chronos.mp3"></audio>',
    # The track is 45 s long: the second copy ends with it, a little before the film.
    f'      <audio id="music2" class="clip" data-start="42" data-duration="24.8" data-media-start="20.125" '
    f'data-track-index="11" data-volume="0.75" data-fade-in="2" data-fade-out="2.5" src="assets/audio/chronos.mp3"></audio>',
    kit.sfx("s-title", 0.2, "impact-bass-1", 0.5, 2.0, 12),
]
for i, t in enumerate([c[0] for c in CHAPTERS]):
    audio.append(kit.sfx(f"s-ch{i}", t - 0.05, "whoosh-short", 0.3, 0.57, 13))
for i, (name, log, start, lead) in enumerate([("gal", gallery, MATH, kit.at(gallery, "click", target="gallery") - 0.9),
                                              ("hov", hover, HOVER_AT, kit.at(hover, "dblclick") - 1.2),
                                              ("bld", build, BUILD_AT, kit.at(build, "type") - 0.3)]):
    end = [HOVER_AT, BUILD_AT, FIND][i]
    for j, e in enumerate(log):
        t = start + e["video_t"] - lead
        if e["kind"] in ("click", "dblclick", "key") and t < end:
            audio.append(kit.sfx(f"s-{name}{j}", t + 0.03, "click", 0.4, 0.2, 14))
for i, t in enumerate(PLAY_CLICKS):
    audio.append(kit.sfx(f"s-play{i}", WATCH + t - PLAY_IN + 0.03, "click", 0.4, 0.2, 14))
for i, t in enumerate([PDF_AT, CSV_AT]):
    audio.append(kit.sfx(f"s-view{i}", t, "whoosh-short", 0.25, 0.57, 13))
for i in range(4):
    audio.append(kit.sfx(f"s-st{i}", FAST + 0.6 + i * 1.2, "key-press", 0.4, 0.2, 15))
audio.append(kit.sfx("s-end", END + 0.05, "pop", 0.5, 0.7, 16))
body = (f'      <div id="film" data-composition-id="film" data-composition-src="compositions/film.html" data-start="0" '
        f'data-duration="{TOTAL}" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>')
(ROOT / "index.html").write_text(kit.index_html(W, H, TOTAL, body, audio, GROUND))
print("ok")
