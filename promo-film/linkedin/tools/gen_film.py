"""Generate the LinkedIn film (1080 x 1350): compositions/app.html, compositions/end.html, index.html.

Layout: a 225 px band on top for captions and numbers, then two full-width screens stacked
with no gap, GitHub Light above and GitHub Dark below (each the 1920 x 1000 capture at
0.5625). Both screens share one camera, one scroll and one timeline, so they never drift.
All app geometry is in capture pixels (1920 x 1000 window at 150 % UI scale).
Run from the project root: python3 tools/gen_film.py
"""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VERSION = "1.0.0-rc.8"
METRICS = json.loads((ROOT / "tools/metrics.json").read_text())


def median(values):
    ordered = sorted(values)
    return ordered[len(ordered) // 2]


MIB = 1.048576  # MiB -> MB
FIRST_MS = METRICS["first_frame_ms"]
COLD_RAM = round(median(METRICS["startup_peak_rss_mib"]) * MIB)
COLD_CPU = median(METRICS["startup_cpu_pct_one_core_first_2s"])
IDLE_RAM = round(median(METRICS["idle_rss_mib_after_10s"]) * MIB)
IDLE_CPU = median(METRICS["idle_cpu_pct_one_core_10_to_15s"])

W, H = 1080, 1350
BAND = 225
CARD_W, CARD_H = 1080, 562.5
CARDS = {"light": (0, BAND), "dark": (0, BAND + CARD_H)}
FPS = 30

# ---------------------------------------------------------------- timing
LAUNCH_AT = 0.5          # exec, in film time; the wait is shown ten times slower
SLOW = 10
PARTIAL_AT = LAUNCH_AT + (FIRST_MS - 33) / 1000 * SLOW   # the frame before the painted one
SHOWN_AT = LAUNCH_AT + FIRST_MS / 1000 * SLOW
STATS_AT, IDLE_AT, TOUR_AT = 2.5, 4.5, 7.5
D = TOUR_AT - 3.0        # the tour below was timed from 3.0
EQ_AT = 25.5 + D
WATCH_AT = 39.0 + D      # 43.5
VID_B_AT = WATCH_AT + 3.0
PDF_AT = WATCH_AT + 5.0
CSV_AT = WATCH_AT + 7.0
END_AT = WATCH_AT + 9.0  # 52.5
TOTAL = END_AT + 5.0


def video_time(theme, name, kind):
    """Time of the first logged event of `kind` in a capture's video (see tools/sync_events.py)."""
    events = json.loads((ROOT / f"assets/cap/{theme}/{name}.json").read_text())
    return next(e["video_t"] for e in events if e["kind"] == kind)


FLIPS = {}


def flip(theme, name, fid, start, duration, media_start, width):
    """A recording as a flipbook of JPEG frames, shown by the timeline instead of <video>.

    The renderer never finished initializing this composition with <video> elements, so the
    recordings play as frames: deterministic, seekable, and frame-exact between the themes.
    Frames are extracted at display resolution into assets/frames/ (generated, not committed).
    """
    folder = ROOT / "assets/frames"
    folder.mkdir(parents=True, exist_ok=True)
    for old in folder.glob(f"{theme}-{fid}-*.jpg"):
        old.unlink()
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-ss", f"{media_start:.3f}", "-i",
                    str(ROOT / f"assets/cap/{theme}/{name}.mp4"), "-t", f"{duration:.3f}", "-vf",
                    f"fps={FPS},scale={width}:-2", "-q:v", "3", str(folder / f"{theme}-{fid}-%03d.jpg")], check=True)
    count = len(list(folder.glob(f"{theme}-{fid}-*.jpg")))
    steps = []
    for i in range(count):
        t = start + i / FPS
        steps.append(f'tl.set("#{fid}-{theme}-{i}", {{ opacity: 1 }}, {t:.3f});')
        if i:
            steps.append(f'tl.set("#{fid}-{theme}-{i - 1}", {{ opacity: 0 }}, {t:.3f});')
    steps.append(f'tl.set("#{fid}-{theme}-{count - 1}", {{ opacity: 0 }}, {start + duration:.3f});')
    FLIPS[(theme, fid)] = steps
    return "".join(f'<img class="win fb" id="{fid}-{theme}-{i}" src="assets/frames/{theme}-{fid}-{i + 1:03d}.jpg" alt="" />'
                   for i in range(count))


DOC = (323, 101, 1572, 864)  # doc pane in the window (x, y, w, h)
FRAMES = {  # scale, focus x, focus y (window px)
    "full": (CARD_W / 1920, 960, 500), "side": (0.85, 300, 420), "doc": (0.85, 1117, 500),
    "find": (1.0, 1430, 300), "pal": (0.72, 960, 470), "search": (0.75, 960, 430), "set": (0.7, 960, 500),
    "gallery": (0.66, 1145, 530), "dialog": (0.74, 960, 612),
}


def cam(name):
    """Camera transform for a framing, clamped so the window always covers the screen."""
    s, fx, fy = FRAMES[name]
    x = min(0.0, max(CARD_W - 1920 * s, CARD_W / 2 - fx * s))
    y = min(0.0, max(CARD_H - 1000 * s, CARD_H / 2 - fy * s))
    return dict(x=round(x, 2), y=round(y, 2), scale=s)


def js(d):
    return "{ " + ", ".join(f"{k}: {v}" for k, v in d.items()) + " }"


def card(k):
    cx, cy = CARDS[k]
    dx, dy, dw, dh = DOC
    cap = f"assets/cap/{k}"
    return f"""
      <div class="card" id="card-{k}" style="left:{cx}px; top:{cy}px">
        <div class="cam" id="cam-{k}">
          <div class="g-view">
            <img class="win" src="{cap}/01-view-top.png" alt="MD View reading view, {k} theme" />
            <div class="docclip" style="left:{dx}px; top:{dy}px; width:{dw}px; height:{dh}px">
              <div class="doc">
                <img src="{cap}/doc-tall.png" alt="" style="width:1572px; height:3744px" />
              </div>
            </div>
          </div>
          <div class="grp g-edit">
            <img class="win e-07" src="{cap}/07-edit-top.png" alt="MD View editor, {k} theme" />
            <img class="win e-08" src="{cap}/08b-find-regex.png" alt="Regex find, {k} theme" />
            <img class="win e-09" src="{cap}/09-replace.png" alt="Find and replace, {k} theme" />
          </div>
          <div class="grp g-split">
            <img class="win" src="{cap}/11-split-top.png" alt="MD View split view, {k} theme" />
            {flip(k, "split-scroll", "sp", 21.0 + D, 4.5, 0.333, 1080)}
          </div>
          <div class="grp g-eq">
            <img class="win q-02" src="{cap}/eq-02-edit.png" alt="Equation buttons in the formatting bar, {k} theme" />
            <img class="win q-03" src="{cap}/eq-03-gallery.png" alt="Equation gallery, {k} theme" />
            {flip(k, "eq-hover", "eh", EQ_AT + 2.0, 4.0, video_time(k, "eq-hover", "dialog") - 0.2, 1420)}
          </div>
          <div class="g-cmd">
            <img class="win c-13" src="{cap}/13-palette.png" alt="Command palette, {k} theme" />
            <img class="win c-14" src="{cap}/14-palette-mode.png" alt="Command palette filtered, {k} theme" />
            <img class="win c-16" src="{cap}/16-search.png" alt="Workspace search, {k} theme" />
            <img class="win c-17" src="{cap}/17-settings.png" alt="Settings, {k} theme" />
            <img class="win c-17b" src="{cap}/17b-settings-themes.png" alt="Theme list, {k} theme" />
          </div>
          <div class="grp g-watch">
            {flip(k, "video-play", "va", WATCH_AT, VID_B_AT - WATCH_AT, 1.5, 1080)}
            {flip(k, "video-play", "vb", VID_B_AT, PDF_AT - VID_B_AT, 6.95, 1080)}
          </div>
          <div class="grp g-pdf"><img class="win" src="{cap}/viewer-pdf.png" alt="PDF viewer, {k} theme" /></div>
          <div class="grp g-csv"><img class="win" src="{cap}/viewer-csv.png" alt="CSV grid, {k} theme" /></div>
          <div class="g-launch">
            <img class="win l-1" src="{cap}/launch-1.png" alt="" />
            <img class="win l-2" src="{cap}/launch-2.png" alt="MD View, first painted frame, {k} theme" />
          </div>
        </div>
      </div>"""


CAPS = [
    (TOUR_AT, 5.5 + D, ["Your folder is", "the workspace."]),
    (5.5 + D, 8.0 + D, ["GitHub tables", "and task lists."]),
    (8.0 + D, 10.5 + D, ["LaTeX, rendered", "as real math."]),
    (10.5 + D, 13.0 + D, ["Mermaid diagrams,", "no browser."]),
    (13.0 + D, 15.0 + D, ["Alerts, quotes", "and footnotes."]),
    (15.0 + D, 17.5 + D, ["A real editor", "underneath."]),
    (17.5 + D, 20.5 + D, ["Regex find", "and replace."]),
    (20.5 + D, EQ_AT, ["Source and preview,", "scrolling in sync."]),
    (EQ_AT, EQ_AT + 2.0, ["Classic formulas,", "one click."]),
    (EQ_AT + 2.0, EQ_AT + 4.0, ["Point at the LaTeX,", "see what it draws."]),
    (EQ_AT + 4.0, EQ_AT + 6.0, ["Edit any equation", "in place."]),
    (31.5 + D, 34.0 + D, ["Every command,", "one keystroke."]),
    (34.0 + D, 36.5 + D, ["Search the", "whole workspace."]),
    (36.5 + D, WATCH_AT, ["Nine themes.", "Light or dark."]),
    (WATCH_AT, VID_B_AT, ["Video with sound,", "built in."]),
    (VID_B_AT, PDF_AT, ["MP4, AVI, TS, HEVC…", "no FFmpeg."]),
    (PDF_AT, END_AT, ["PDFs and", "spreadsheets, too."]),
]
cap_html = "".join(
    f'<div class="cap" id="cap{i}">' + "".join('<div class="ln">' + " ".join(f"<span>{w}</span>" for w in line.split(" ")) + "</div>"
                                              for line in lines) + "</div>"
    for i, (_, _, lines) in enumerate(CAPS))



stats_html = f"""
      <div class="band" id="launch">
        <div class="lbl">Cold start <em>· shown 10× slower</em></div>
        <div class="big"><span id="ms">0</span> ms</div>
      </div>
      <div class="band" id="stats">
        <div class="col" id="col-cold">
          <div class="lbl">Cold start</div>
          <div class="val"><span id="cold-ram">0</span> MB <small>RAM</small></div>
          <div class="val"><span id="cold-cpu">0</span> <small>CPU</small></div>
        </div>
        <div class="col" id="col-idle">
          <div class="lbl">Loaded, idle</div>
          <div class="val"><span id="idle-ram">0</span> MB <small>RAM</small></div>
          <div class="val"><span id="idle-cpu">0</span> <small>CPU</small></div>
        </div>
        <div class="note">Ryzen 9 6900HX · Linux · CPU = share of one core</div>
      </div>"""

T = []
a = T.append


def count(selector, start, duration, end, fmt):
    """Count a number up frame by frame (deterministic: one set per frame)."""
    steps = max(1, round(duration * FPS))
    for i in range(steps + 1):
        f = i / steps
        e = 1 - (1 - f) ** 3
        a(f'tl.set("{selector}", {{ textContent: "{fmt(end * e)}" }}, {start + duration * f:.3f});')


def blur_in(selector, t):
    a(f'tl.fromTo("{selector}", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.18, ease: "power2.out", stagger: 0.06 }}, {t:.2f});')


def blur_out(selector, t):
    a(f'tl.to("{selector}", {{ opacity: 0, filter: "blur(8px)", duration: 0.12, ease: "power2.in" }}, {t - 0.12:.2f});')


# ---- launch: black screens, the real first frames at their measured times (slowed 10x)
count("#ms", LAUNCH_AT, SHOWN_AT - LAUNCH_AT, FIRST_MS, lambda v: f"{v:.0f}")
a(f'tl.set(".l-1", {{ opacity: 1 }}, {PARTIAL_AT:.3f});')
a(f'tl.set(".l-2", {{ opacity: 1 }}, {SHOWN_AT:.3f});')
a('tl.fromTo("#launch .big", { scale: 1 }, { scale: 1.06, duration: 0.12, ease: "power2.out", yoyo: true, repeat: 1 }, '
  f'{SHOWN_AT:.3f});')
blur_out("#launch", STATS_AT)
a(f'tl.set(".g-launch", {{ opacity: 0 }}, {STATS_AT});')
# ---- footprint: cold start, then loaded and idle
a(f'tl.set("#stats", {{ opacity: 1 }}, {STATS_AT});')
blur_in("#col-cold > div, #stats .note", STATS_AT)
count("#cold-ram", STATS_AT + 0.1, 0.6, COLD_RAM, lambda v: f"{v:.0f}")
count("#cold-cpu", STATS_AT + 0.1, 0.6, COLD_CPU, lambda v: f"{v:.1f} %")
blur_in("#col-idle > div", IDLE_AT)
count("#idle-ram", IDLE_AT + 0.1, 0.6, IDLE_RAM, lambda v: f"{v:.0f}")
count("#idle-cpu", IDLE_AT + 0.1, 0.6, IDLE_CPU, lambda v: f"{v:.1f} %")
blur_out("#stats", TOUR_AT)
# ---- captions
for i, (t0, t1, _) in enumerate(CAPS):
    a(f'tl.set("#cap{i}", {{ opacity: 1 }}, {t0});')
    blur_in(f"#cap{i} span", t0)
    if i < len(CAPS) - 1:
        blur_out(f"#cap{i}", t1)


def move(name, t, d=0.6, ease="power3.inOut"):
    a(f'tl.to(".cam", {{ ...{js(cam(name))}, duration: {d}, ease: "{ease}" }}, {t:.2f});')


def wipe(group, t):
    """A clean top-to-bottom wipe to the next screen; no line, no flash."""
    a(f'tl.fromTo("{group}", {{ clipPath: "inset(0 0 100% 0)" }}, {{ clipPath: "inset(0 0 0% 0)", duration: 0.4, ease: "power2.inOut" }}, {t:.2f});')


# ---- tour (timed from 3.0 in the earlier cut, moved by D)
move("side", TOUR_AT)
move("doc", 5.5 + D)
for t, y in [(5.5, 715), (8.0, 1421), (10.5, 2551), (13.0, 2880)]:
    a(f'tl.to(".doc", {{ y: {-y}, duration: 0.8, ease: "power2.inOut" }}, {t + D:.2f});')
wipe(".g-edit", 15.0 + D)
move("full", 15.0 + D)
move("find", 17.5 + D)
a(f'tl.set(".e-08", {{ opacity: 1 }}, {17.5 + D});')
a(f'tl.set(".e-09", {{ opacity: 1 }}, {19.0 + D});')
wipe(".g-split", 20.5 + D)
move("full", 20.5 + D)
wipe(".g-eq", EQ_AT)
move("full", EQ_AT)
a(f'tl.set(".q-03", {{ opacity: 1 }}, {EQ_AT + 0.6});')
move("gallery", EQ_AT + 0.6)
move("dialog", EQ_AT + 2.0, 0.5)
C = 31.5 + D
a(f'tl.set(".g-cmd", {{ opacity: 1 }}, {C});')
a(f'tl.set(".c-13", {{ opacity: 1 }}, {C});')
move("pal", C, 0.5)
a(f'tl.set(".c-14", {{ opacity: 1 }}, {C + 1.0});')
a(f'tl.set(".c-16", {{ opacity: 1 }}, {C + 2.5});')
move("search", C + 2.5, 0.5)
a(f'tl.set(".c-17", {{ opacity: 1 }}, {C + 5.0});')
move("set", C + 5.0, 0.5)
a(f'tl.set(".c-17b", {{ opacity: 1 }}, {C + 6.0});')
# ---- watch: video, then PDF and CSV
wipe(".g-watch", WATCH_AT)
move("full", WATCH_AT)
wipe(".g-pdf", PDF_AT)
wipe(".g-csv", CSV_AT)
blur_out(f"#cap{len(CAPS) - 1}", END_AT - 0.1)

CARD_HTML = {k: card(k) for k in ("light", "dark")}
for steps in FLIPS.values():
    T.extend(steps)
app_js = "\n      ".join(T)

app = f"""<template>
  <style>
    #root {{ position: absolute; inset: 0; background: #0b6bd6; overflow: hidden; font-family: Inter, sans-serif; }}
    #app-stage {{ position: absolute; inset: 0; overflow: hidden; }}
    .card {{ position: absolute; width: {CARD_W}px; height: {CARD_H}px; overflow: hidden; background: #000000; }}
    .cam {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; transform-origin: 0 0; }}
    .g-view, .grp, .g-cmd, .g-launch {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; }}
    .g-launch {{ background: #000000; }}
    .win {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; }}
    .fb, .l-1, .l-2 {{ opacity: 0; }}
    .docclip {{ position: absolute; overflow: hidden; }}
    .doc {{ position: absolute; left: 0; top: 0; width: 1572px; height: 3744px; }}
    .doc img {{ position: absolute; left: 0; top: 0; }}
    .grp {{ clip-path: inset(0 0 100% 0); }}
    .g-cmd {{ opacity: 0; }}
    .e-08, .e-09, .c-13, .c-14, .c-16, .c-17, .c-17b, .q-03 {{ opacity: 0; }}
    .cap {{ position: absolute; left: 60px; top: 44px; width: 960px; font-size: 66px; font-weight: 900; letter-spacing: -0.03em; line-height: 1.04; color: #ffffff; }}
    .cap .ln {{ white-space: nowrap; }}
    .cap span {{ display: inline-block; }}
    .band {{ position: absolute; left: 0; top: 0; width: {W}px; height: {BAND}px; color: #ffffff; }}
    .lbl {{ font-size: 28px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; color: #ffffff; }}
    .lbl em {{ font-style: normal; font-weight: 600; letter-spacing: 0.02em; text-transform: none; color: #ffffff; }}
    #launch .lbl {{ position: absolute; left: 60px; top: 40px; }}
    #launch .big {{ position: absolute; left: 60px; top: 76px; font-size: 112px; font-weight: 900; letter-spacing: -0.04em; line-height: 1; font-variant-numeric: tabular-nums; transform-origin: 0 50%; }}
    #stats {{ opacity: 0; }}
    .col {{ position: absolute; top: 22px; width: 480px; }}
    #col-cold {{ left: 60px; }}
    #col-idle {{ left: 560px; }}
    .val {{ font-size: 52px; font-weight: 900; letter-spacing: -0.03em; line-height: 1.08; font-variant-numeric: tabular-nums; white-space: nowrap; }}
    .val small {{ font-size: 28px; font-weight: 700; letter-spacing: 0.04em; color: #ffffff; }}
    .col .lbl {{ margin-bottom: 6px; }}
    .note {{ position: absolute; left: 60px; top: 182px; font-size: 28px; font-weight: 500; color: #ffffff; white-space: nowrap; }}
  </style>
  <div id="root" data-composition-id="app" data-width="{W}" data-height="{H}">
    <div id="app-stage" data-layout-allow-overflow data-layout-allow-overlap data-layout-allow-occlusion>
      {CARD_HTML['light']}
      {CARD_HTML['dark']}
      {stats_html}
      {cap_html}
    </div>
  </div>
  <script>
    (() => {{
      const tl = gsap.timeline({{ paused: true }});
      gsap.set(".cam", {js(cam("full"))});
      gsap.set(".doc", {{ y: 0 }});
      gsap.set(".cap", {{ opacity: 0 }});
      {app_js}
      window.__timelines["app"] = tl;
    }})();
  </script>
</template>
"""
(ROOT / "compositions").mkdir(exist_ok=True)
(ROOT / "compositions/app.html").write_text(app)

# ---------------------------------------------------------------- END
LX, LY = CARDS["light"]
DX, DY = CARDS["dark"]
ICON = 180
ix = W / 2 - ICON / 2
seam = LY + CARD_H
end = f"""<template>
  <style>
    #root {{ position: absolute; inset: 0; background: #0b6bd6; overflow: hidden; font-family: Inter, sans-serif; }}
    #end-stage {{ position: absolute; inset: 0; overflow: hidden; }}
    .ec {{ position: absolute; width: {CARD_W}px; height: {CARD_H}px; }}
    .ec-l {{ left: {LX}px; top: {LY}px; background: #ffffff; }}
    .ec-d {{ left: {DX}px; top: {DY}px; background: #0d1117; }}
    .ewhite {{ position: absolute; inset: 0; background: #ffffff; clip-path: inset(0 0 100% 0); }}
    .elogo {{ position: absolute; left: {ix}px; top: {seam - ICON / 2}px; width: {ICON}px; height: {ICON}px; opacity: 0; }}
    .et {{ position: absolute; left: 0; width: {W}px; text-align: center; white-space: nowrap; color: #0a1220; }}
    .ew {{ top: 690px; font-size: 132px; font-weight: 900; letter-spacing: -0.045em; line-height: 1.1; }}
    .ecta {{ top: 862px; font-size: 38px; font-weight: 700; letter-spacing: -0.01em; line-height: 1.2; }}
    .eos {{ top: 914px; font-size: 34px; font-weight: 500; line-height: 1.2; color: #3b4658; }}
    .eurl {{ top: 988px; font-family: "JetBrains Mono", monospace; font-size: 29px; line-height: 1.3; color: #0b5cb8; }}
  </style>
  <div id="root" data-composition-id="end" data-width="{W}" data-height="{H}">
    <div id="end-stage" data-layout-allow-overlap>
      <div class="ec ec-l"></div>
      <div class="ec ec-d"></div>
      <div class="ewhite"></div>
      <img class="elogo" src="assets/images/logo.png" alt="MD View app icon" />
      <div class="et ew">MD View</div>
      <div class="et ecta">Download {VERSION}</div>
      <div class="et eos">macOS · Windows · Linux · free &amp; open source</div>
      <div class="et eurl">github.com/jigonzalez930209/md-view-rs</div>
    </div>
  </div>
  <script>
    (() => {{
      const tl = gsap.timeline({{ paused: true }});
      gsap.set(".ew, .ecta, .eos, .eurl", {{ opacity: 0 }});
      // The two screens fold into the two halves of the icon (top half light, bottom half dark).
      tl.fromTo(".ec-l", {{ clipPath: "inset(0px 0px 0px 0px round 0px 0px 0px 0px)" }},
        {{ clipPath: "inset({CARD_H - ICON / 2:.1f}px {CARD_W - (ix - LX) - ICON:.0f}px 0px {ix - LX:.0f}px round 40px 40px 0px 0px)", duration: 0.7, ease: "power3.inOut" }}, 0.1);
      tl.fromTo(".ec-d", {{ clipPath: "inset(0px 0px 0px 0px round 0px 0px 0px 0px)" }},
        {{ clipPath: "inset(0px {CARD_W - (ix - DX) - ICON:.0f}px {CARD_H - ICON / 2:.1f}px {ix - DX:.0f}px round 0px 0px 40px 40px)", duration: 0.7, ease: "power3.inOut" }}, 0.1);
      // The ground wipes to white; the icon takes the place of the two halves.
      tl.to(".ewhite", {{ clipPath: "inset(0 0 0% 0)", duration: 0.5, ease: "power2.inOut" }}, 0.85);
      tl.set(".elogo", {{ opacity: 1 }}, 1.1);
      tl.set(".ec", {{ opacity: 0 }}, 1.1);
      tl.to(".elogo", {{ y: {470 - (seam - ICON / 2):.0f}, duration: 0.55, ease: "power3.inOut" }}, 1.45);
      tl.fromTo(".ew", {{ opacity: 0, clipPath: "inset(0 50% 0 50%)" }}, {{ opacity: 1, clipPath: "inset(0 0% 0 0%)", duration: 0.45, ease: "power3.out" }}, 1.8);
      tl.fromTo(".ecta", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.18, ease: "power2.out" }}, 2.1);
      tl.fromTo(".eos", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.18, ease: "power2.out" }}, 2.22);
      tl.fromTo(".eurl", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.18, ease: "power2.out" }}, 2.34);
      window.__timelines["end"] = tl;
    }})();
  </script>
</template>
"""
(ROOT / "compositions/end.html").write_text(end)

# ---------------------------------------------------------------- COVER (the first frame)
INTRO = 1.5  # the cover holds for three beats; every later time in index.html moves by it
LIGHT_COVER = cam("gallery")
cover = f"""<template>
  <style>
    #root {{ position: absolute; inset: 0; background: #0b6bd6; overflow: hidden; font-family: Inter, sans-serif; }}
    #cover-stage {{ position: absolute; inset: 0; overflow: hidden; }}
    .cv {{ position: absolute; left: 0; width: {CARD_W}px; height: {CARD_H}px; overflow: hidden; background: #000000; }}
    .cv img {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; transform-origin: 0 0; }}
    .cv-l {{ top: {BAND}px; }}
    .cv-d {{ top: {BAND + CARD_H}px; }}
    .cv-l img {{ transform: translate({LIGHT_COVER['x']}px, {LIGHT_COVER['y']}px) scale({LIGHT_COVER['scale']}); }}
    .cv-d img {{ transform: scale({CARD_W / 1920}); }}
    .icon {{ position: absolute; left: 60px; top: 48px; width: 128px; height: 128px; }}
    .name {{ position: absolute; left: 218px; top: 40px; font-size: 92px; font-weight: 900; letter-spacing: -0.045em; line-height: 1; color: #ffffff; white-space: nowrap; }}
    .tag {{ position: absolute; left: 222px; top: 140px; font-size: 36px; font-weight: 700; letter-spacing: -0.01em; line-height: 1.1; color: #ffffff; white-space: nowrap; }}
    .ver {{ position: absolute; right: 60px; top: 58px; padding: 8px 18px; border-radius: 999px; background: #ffffff; color: #0b5cb8; font-size: 30px; font-weight: 800; line-height: 1.1; white-space: nowrap; }}
  </style>
  <div id="root" data-composition-id="cover" data-width="{W}" data-height="{H}">
    <div id="cover-stage" data-layout-allow-overflow>
      <div class="cv cv-l"><img src="assets/cap/light/eq-03-gallery.png" alt="MD View equation gallery, light theme" /></div>
      <div class="cv cv-d"><img src="assets/cap/dark/cover-video.png" alt="MD View playing a video, dark theme" /></div>
      <img class="icon" src="assets/images/logo.png" alt="MD View app icon" />
      <div class="name">MD View</div>
      <div class="tag">Native Markdown. Now with video.</div>
      <div class="ver">{VERSION}</div>
    </div>
  </div>
  <script>
    (() => {{
      const tl = gsap.timeline({{ paused: true }});
      tl.set(".name", {{ opacity: 1 }}, 0);
      window.__timelines["cover"] = tl;
    }})();
  </script>
</template>
"""
(ROOT / "compositions/cover.html").write_text(cover)

# ---------------------------------------------------------------- INDEX + SOUND
sfx = []


def s(i, t, f, v, d, lane=12):
    sfx.append(f'      <audio id="{i}" class="clip" data-start="{t + INTRO:.2f}" data-duration="{d}" data-track-index="{lane}" data-volume="{v}" src="assets/sfx/{f}.mp3"></audio>')


s("sfx-launch", LAUNCH_AT - 0.04, "click", 0.5, 0.2)
s("sfx-shown", SHOWN_AT - 0.03, "pop", 0.45, 0.7, 13)
s("sfx-cold", STATS_AT + 0.05, "impact-bass-1", 0.35, 2.0, 14)
s("sfx-idle", IDLE_AT + 0.05, "key-press", 0.4, 0.2)
for i, t in enumerate([15.0 + D, 20.5 + D, EQ_AT, WATCH_AT, PDF_AT, CSV_AT]):
    s(f"sfx-wipe{i}", t + 0.04, "whoosh-short", 0.3, 0.57, 13)
for i, t in enumerate([EQ_AT + 0.6, EQ_AT + 2.0, C, C + 2.5, C + 5.0]):
    s(f"sfx-ui{i}", t - 0.04, "click", 0.5, 0.2)
s("sfx-logo", END_AT + 1.05, "impact-bass-1", 0.6, 2.1, 14)

index = f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width={W}, height={H}" />
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>
    <style>
      :root {{ --ground: #0b6bd6; --ink: #ffffff; }}
      * {{ margin: 0; padding: 0; box-sizing: border-box; }}
      html, body {{ margin: 0; width: {W}px; height: {H}px; overflow: hidden; background: var(--ground); }}
      #root {{ position: relative; width: {W}px; height: {H}px; overflow: hidden; background: var(--ground); }}
    </style>
  </head>
  <body>
    <div id="root" data-composition-id="main" data-start="0" data-duration="{INTRO + TOTAL:g}" data-width="{W}" data-height="{H}">
      <div id="cover" data-composition-id="cover" data-composition-src="compositions/cover.html" data-start="0" data-duration="{INTRO:g}" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>
      <div id="app" data-composition-id="app" data-composition-src="compositions/app.html" data-start="{INTRO:g}" data-duration="{END_AT:g}" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>
      <div id="end" data-composition-id="end" data-composition-src="compositions/end.html" data-start="{INTRO + END_AT:g}" data-duration="5" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>

      <audio id="music" class="clip" data-start="0" data-duration="44.8" data-media-start="0.125" data-track-index="10" data-volume="0.8" data-fade-out="2" src="assets/audio/chronos.mp3"></audio>
      <audio id="music2" class="clip" data-start="{43 + INTRO:g}" data-duration="{TOTAL + INTRO - 43 - INTRO:g}" data-media-start="20.125" data-track-index="11" data-volume="0.8" data-fade-in="2" data-fade-out="2.5" src="assets/audio/chronos.mp3"></audio>
{chr(10).join(sfx)}
    </div>
    <script>
      window.__timelines["main"] = gsap.timeline({{ paused: true }});
    </script>
  </body>
</html>
"""
(ROOT / "index.html").write_text(index)
print(f"ok: {INTRO + TOTAL:g} s; first frame {FIRST_MS} ms; cold {COLD_RAM} MB / {COLD_CPU} %; idle {IDLE_RAM} MB / {IDLE_CPU} %")
