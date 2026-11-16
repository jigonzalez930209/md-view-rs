"""Generate the LinkedIn film (1080 x 1350): compositions/app.html, compositions/end.html, index.html.

All app geometry is in capture pixels (1920 x 1000 window at 150 % UI scale). Both cards
(light above, dark below) share one camera, one scroll and one timeline, so they never drift.
Run from the project root: python3 tools/gen_film.py
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BARS = json.loads((ROOT / "tools/bars.json").read_text())

W, H = 1080, 1350
CARD_W, CARD_H = 980, 490
CARDS = {"light": (50, 262), "dark": (50, 772)}
ACCENT = "#e8336d"
THEME = {
    "light": dict(menu="#ffffff", side="#f6f8fa", tabs="#f6f8fa", doc="#ffffff", status="#ffffff", bar="#c9d3df"),
    "dark": dict(menu="#0d1117", side="#161b22", tabs="#161b22", doc="#0d1117", status="#0d1117", bar="#30363d"),
}
REGIONS = [("menu", 0, 0, 1920, 50), ("side", 0, 51, 322, 914), ("tabs", 323, 51, 1597, 49),
           ("doc", 323, 101, 1597, 864), ("status", 0, 966, 1920, 34)]
DOC = (323, 101, 1572, 864)  # doc pane in the window (x, y, w, h)

FRAMES = {  # scale, focus x, focus y (window px)
    "full": (0.49, 960, 500), "side": (0.85, 576, 500), "doc": (0.85, 1117, 500),
    "find": (1.0, 1430, 290), "pal": (0.72, 960, 490), "search": (0.75, 960, 430), "set": (0.7, 960, 500),
}


def cam(name):
    s, fx, fy = FRAMES[name]
    return dict(x=round(CARD_W / 2 - fx * s, 2), y=round(CARD_H / 2 - fy * s, 2), scale=s)


def js(d):
    return "{ " + ", ".join(f"{k}: {v}" for k, v in d.items()) + " }"


# Outlines + tags: id, group ("doc" = page coords, "win" = window coords), rect, label, framing scale.
MARKS = [
    ("explorer", "win", (6, 104, 312, 750), "explorer", 0.85),
    ("table", "doc", (238, 878, 742, 366), "table", 0.85),
    ("tasks", "doc", (238, 1314, 872, 100), "task list", 0.85),
    ("latex", "doc", (238, 1744, 770, 214), "LaTeX", 0.85),
    ("mermaid", "doc", (440, 2744, 718, 104), "mermaid", 0.85),
    ("alert", "doc", (238, 3396, 1120, 104), "alert", 0.85),
    ("find", "edit", (1258, 122, 660, 136), "regex", 1.0),
    ("src", "split", (330, 104, 784, 858), "source", 0.49),
    ("pre", "split", (1120, 104, 792, 858), "preview", 0.49),
    ("pal", "cmd", (518, 198, 884, 604), "Ctrl+Shift+P", 0.72),
    ("srch", "cmd", (554, 176, 814, 44), "Ctrl+Shift+F", 0.75),
    ("thm", "cmd", (1146, 346, 258, 318), "9 themes", 0.7),
]


def mark_html(k, group):
    out = []
    for mid, g, (x, y, w, h), label, s in MARKS:
        if g != group:
            continue
        bw = 4 / s
        fs = 30 / s
        side = f"right:{-bw:.1f}px" if g == "doc" else f"left:{-bw:.1f}px"
        rad = f"{8 / s:.1f}px {8 / s:.1f}px 0 {8 / s:.1f}px" if g == "doc" else f"{8 / s:.1f}px {8 / s:.1f}px {8 / s:.1f}px 0"
        out.append(
            f'<div class="mk mk-{mid}" style="left:{x}px; top:{y}px; width:{w}px; height:{h}px; border-width:{bw:.1f}px">'
            f'<div class="tg" style="font-size:{fs:.1f}px; padding:{8 / s:.1f}px {14 / s:.1f}px; top:{-fs * 1.0 - 16 / s - bw:.1f}px; {side}; border-radius:{rad}">{label}</div></div>'
        )
    return "".join(out)


def card(k):
    t = THEME[k]
    cx, cy = CARDS[k]
    fills = "".join(
        f'<div class="fl fl-{r}" style="left:{x}px; top:{y}px; width:{w}px; height:{h}px; background:{t[r]}"></div>'
        for r, x, y, w, h in REGIONS)
    strokes = "".join(
        f'<div class="rs rs-{r}" style="left:{x}px; top:{y}px; width:{w}px; height:{h}px"></div>' for r, x, y, w, h in REGIONS)
    bars = "".join(
        f'<div class="br br-{g}" style="left:{x}px; top:{y + 2}px; width:{w}px; height:{max(h - 4, 8)}px; background:{t["bar"]}"></div>'
        for g in ["menu", "tabs", "side", "doc", "status"] for (x, y, w, h) in BARS[g])
    dx, dy, dw, dh = DOC
    cap = f"assets/cap/{k}"
    return f"""
      <div class="card" id="card-{k}" style="left:{cx}px; top:{cy}px">
        <div class="cbg" style="background:{t['doc']}"></div>
        <div class="cam" id="cam-{k}">
          <div class="g-view">
            {fills}{bars}
            <div class="real">
              <img class="win" src="{cap}/01-view-top.png" alt="MD View reading view, {k} theme" />
              <div class="docclip" style="left:{dx}px; top:{dy}px; width:{dw}px; height:{dh}px">
                <div class="doc">
                  <img src="{cap}/doc-tall.png" alt="" style="width:1572px; height:3744px" />
                  {mark_html(k, 'doc')}
                </div>
              </div>
              {mark_html(k, 'win')}
            </div>
            {strokes}
          </div>
          <div class="grp g-edit">
            <img class="win e-07" src="{cap}/07-edit-top.png" alt="MD View editor, {k} theme" />
            <img class="win e-08" src="{cap}/08b-find-regex.png" alt="Regex find, {k} theme" />
            <img class="win e-09" src="{cap}/09-replace.png" alt="Find and replace, {k} theme" />
            {mark_html(k, 'edit')}
          </div>
          <div class="grp g-split">
            <img class="win" src="{cap}/11-split-top.png" alt="MD View split view, {k} theme" />
            <video id="vid-{k}" class="clip win" data-start="21.0" data-duration="4.5" data-media-start="{0.333 if k == "light" else 0.3}" data-track-index="{1 if k == "light" else 2}" muted playsinline src="{cap}/split-scroll.mp4"></video>
            {mark_html(k, 'split')}
          </div>
          <div class="g-cmd">
            <img class="win c-13" src="{cap}/13-palette.png" alt="Command palette, {k} theme" />
            <img class="win c-14" src="{cap}/14-palette-mode.png" alt="Command palette filtered, {k} theme" />
            <img class="win c-16" src="{cap}/16-search.png" alt="Workspace search, {k} theme" />
            <img class="win c-17" src="{cap}/17-settings.png" alt="Settings, {k} theme" />
            <img class="win c-17b" src="{cap}/17b-settings-themes.png" alt="Theme list, {k} theme" />
            {mark_html(k, 'cmd')}
          </div>
        </div>
        <div class="scan"></div>
        <div class="edge e-top"></div><div class="edge e-right"></div><div class="edge e-bottom"></div><div class="edge e-left"></div>
      </div>"""


CAPS = [
    (0.0, 3.0, ["Markdown,", "drawn natively."]),
    (3.0, 5.5, ["Your folder is", "the workspace."]),
    (5.5, 8.0, ["GitHub tables", "and task lists."]),
    (8.0, 10.5, ["LaTeX, rendered", "as real math."]),
    (10.5, 13.0, ["Mermaid diagrams,", "no browser."]),
    (13.0, 15.0, ["Alerts, quotes", "and footnotes."]),
    (15.0, 17.5, ["A real editor", "underneath."]),
    (17.5, 20.5, ["Regex find", "and replace."]),
    (20.5, 25.5, ["Source and preview,", "scrolling in sync."]),
    (25.5, 28.0, ["Every command,", "one keystroke."]),
    (28.0, 30.5, ["Search the", "whole workspace."]),
    (30.5, 33.0, ["Nine themes.", "Light or dark."]),
]

cap_html = "".join(
    f'<div class="cap" id="cap{i}">' + "".join(f'<div class="ln">' + " ".join(f"<span>{w}</span>" for w in line.split(" ")) + "</div>" for line in lines) + "</div>"
    for i, (_, _, lines) in enumerate(CAPS))

T = []  # timeline lines
a = T.append
# ---- captions
for i, (t0, t1, _) in enumerate(CAPS):
    if i:
        a(f'tl.set("#cap{i}", {{ opacity: 1 }}, {t0});')
        a(f'tl.fromTo("#cap{i} span", {{ opacity: 0, filter: "blur(8px)" }}, {{ opacity: 1, filter: "blur(0px)", duration: 0.15, ease: "power2.out", stagger: 0.06 }}, {t0});')
    a(f'tl.to("#cap{i}", {{ opacity: 0, filter: "blur(8px)", duration: 0.12, ease: "power2.in" }}, {t1 - 0.12:.2f});')

# ---- paint (0 - 2.6)
a('tl.fromTo(".e-top", { scaleX: 0 }, { scaleX: 1, duration: 0.12, ease: "power2.in" }, 0.0);')
a('tl.fromTo(".e-right", { scaleY: 0 }, { scaleY: 1, duration: 0.12, ease: "none" }, 0.12);')
a('tl.fromTo(".e-bottom", { scaleX: 0 }, { scaleX: 1, duration: 0.12, ease: "none" }, 0.24);')
a('tl.fromTo(".e-left", { scaleY: 0 }, { scaleY: 1, duration: 0.12, ease: "power2.out" }, 0.36);')
a('tl.set(".cbg", { opacity: 1 }, 0.5);')
a('tl.set(".card", { boxShadow: "0 30px 70px rgba(3,25,60,0.45), 0 8px 20px rgba(3,25,60,0.3)" }, 0.5);')
a('tl.to(".edge", { opacity: 0, duration: 0.2 }, 0.55);')
for i, (r, *_rest) in enumerate(REGIONS):
    t = 0.6 + i * 0.15
    a(f'tl.set(".rs-{r}", {{ opacity: 1 }}, {t:.2f});')
    a(f'tl.fromTo(".rs-{r}", {{ clipPath: "inset(0 100% 0 0)" }}, {{ clipPath: "inset(0 0% 0 0)", duration: 0.12, ease: "power2.out" }}, {t:.2f});')
    a(f'tl.set(".fl-{r}", {{ opacity: 1 }}, {t + 0.1:.2f});')
for j, g in enumerate(["menu", "tabs", "side", "doc", "status"]):
    t0 = [1.3, 1.42, 1.52, 1.85, 2.15][j]
    st = [0.012, 0.02, 0.012, 0.016, 0.02][j]
    a(f'tl.set(".br-{g}", {{ opacity: 1, stagger: {st} }}, {t0});')
a('tl.to(".real", { opacity: 1, duration: 0.25, ease: "power1.inOut" }, 2.3);')
a('tl.to(".rs", { opacity: 0, duration: 0.25 }, 2.3);')
a('tl.set(".br, .fl", { opacity: 0 }, 2.6);')


def move(name, t, d=0.6, ease="power3.inOut"):
    a(f'tl.to(".cam", {{ ...{js(cam(name))}, duration: {d}, ease: "{ease}" }}, {t});')


def show_mark(mid, t):
    a(f'tl.set(".mk-{mid}", {{ opacity: 1 }}, {t:.2f});')
    a(f'tl.fromTo(".mk-{mid}", {{ clipPath: "inset(0 100% 100% 0)" }}, {{ clipPath: "inset(-200px -200px -200px -200px)", duration: 0.25, ease: "power2.out" }}, {t:.2f});')
    a(f'tl.fromTo(".mk-{mid} .tg", {{ scale: 0.6, opacity: 0 }}, {{ scale: 1, opacity: 1, duration: 0.3, ease: "back.out(1.6)" }}, {t + 0.1:.2f});')


def hide_mark(mid, t):
    a(f'tl.to(".mk-{mid}", {{ opacity: 0, duration: 0.15, ease: "power1.in" }}, {t:.2f});')


def sweep(group, t):
    a(f'tl.fromTo("{group}", {{ clipPath: "inset(0 0 100% 0)" }}, {{ clipPath: "inset(0 0 0% 0)", duration: 0.4, ease: "none" }}, {t});')
    a(f'tl.set(".scan", {{ opacity: 1, y: 0 }}, {t});')
    a(f'tl.to(".scan", {{ y: {CARD_H - 4}, duration: 0.4, ease: "none" }}, {t});')
    a(f'tl.set(".scan", {{ opacity: 0 }}, {t + 0.4:.2f});')


# ---- explorer
move("side", 3.0)
show_mark("explorer", 3.65)
hide_mark("explorer", 5.35)
# ---- reading tour: camera to the column, page scrolls in both cards
move("doc", 5.5)
STOPS = [(5.5, 715, ["table", "tasks"]), (8.0, 1421, ["latex"]), (10.5, 2551, ["mermaid"]), (13.0, 2880, ["alert"])]
for t, y, marks in STOPS:
    a(f'tl.to(".doc", {{ y: {-y}, duration: 0.8, ease: "power2.inOut" }}, {t});')
    for n, m in enumerate(marks):
        show_mark(m, t + 0.85 + n * 0.35)
nxt = {5.5: 7.85, 8.0: 10.35, 10.5: 12.85, 13.0: 14.85}
for t, y, marks in STOPS:
    for m in marks:
        hide_mark(m, nxt[t])
# ---- edit
sweep(".g-edit", 15.0)
move("full", 15.0)
move("find", 17.5)
a('tl.set(".e-08", { opacity: 1 }, 17.5);')
show_mark("find", 18.1)
a('tl.set(".e-09", { opacity: 1 }, 19.0);')
hide_mark("find", 20.35)
# ---- split
sweep(".g-split", 20.5)
move("full", 20.5)
show_mark("src", 21.2)
show_mark("pre", 21.45)
hide_mark("src", 25.35)
hide_mark("pre", 25.35)
# ---- commands
a('tl.set(".g-cmd", { opacity: 1 }, 25.5);')
a('tl.set(".c-13", { opacity: 1 }, 25.5);')
move("pal", 25.5, 0.5)
show_mark("pal", 25.9)
a('tl.set(".c-14", { opacity: 1 }, 26.5);')
a('tl.set(".mk-pal", { opacity: 0 }, 26.5);')
a('tl.set(".c-16", { opacity: 1 }, 28.0);')
move("search", 28.0, 0.5)
show_mark("srch", 28.4)
hide_mark("srch", 30.35)
a('tl.set(".c-17", { opacity: 1 }, 30.5);')
move("set", 30.5, 0.5)
a('tl.set(".c-17b", { opacity: 1 }, 31.5);')
show_mark("thm", 31.6)
a('tl.to(".cam", { opacity: 0, duration: 0.25, ease: "power1.in" }, 32.72);')

app_js = "\n      ".join(T)
initial = cam("full")

app = f"""<template>
  <style>
    #root {{ position: absolute; inset: 0; background: #0b6bd6; overflow: hidden; font-family: Inter, sans-serif; }}
    #app-stage {{ position: absolute; inset: 0; overflow: hidden; }}
    .card {{ position: absolute; width: {CARD_W}px; height: {CARD_H}px; border-radius: 20px; overflow: hidden; }}
    .cbg {{ position: absolute; inset: 0; opacity: 0; }}
    .cam {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; transform-origin: 0 0; }}
    .g-view, .grp, .g-cmd, .real {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; }}
    .win {{ position: absolute; left: 0; top: 0; width: 1920px; height: 1000px; }}
    .fl, .br, .rs {{ position: absolute; opacity: 0; }}
    .br {{ border-radius: 6px; }}
    .rs {{ border: 6px solid {ACCENT}; }}
    .real {{ opacity: 0; }}
    .docclip {{ position: absolute; overflow: hidden; }}
    .doc {{ position: absolute; left: 0; top: 0; width: 1572px; height: 3744px; }}
    .doc img {{ position: absolute; left: 0; top: 0; }}
    .grp {{ clip-path: inset(0 0 100% 0); }}
    .g-cmd {{ opacity: 0; }}
    .e-08, .e-09, .c-13, .c-14, .c-16, .c-17, .c-17b {{ opacity: 0; }}
    .mk {{ position: absolute; border-style: solid; border-color: {ACCENT}; border-radius: 8px; opacity: 0; }}
    .tg {{ position: absolute; background: {ACCENT}; color: #ffffff; font-weight: 800; letter-spacing: -0.01em; line-height: 1; white-space: nowrap; transform-origin: 0 100%; }}
    .scan {{ position: absolute; left: 0; top: 0; width: {CARD_W}px; height: 4px; background: {ACCENT}; opacity: 0; }}
    .edge {{ position: absolute; background: #ffffff; }}
    .e-top {{ left: 0; top: 0; width: 100%; height: 4px; transform-origin: 0 50%; }}
    .e-right {{ right: 0; top: 0; width: 4px; height: 100%; transform-origin: 50% 0; }}
    .e-bottom {{ left: 0; bottom: 0; width: 100%; height: 4px; transform-origin: 100% 50%; }}
    .e-left {{ left: 0; top: 0; width: 4px; height: 100%; transform-origin: 50% 100%; }}
    .cap {{ position: absolute; left: 60px; top: 84px; width: 960px; font-size: 66px; font-weight: 900; letter-spacing: -0.03em; line-height: 1.04; color: #ffffff; }}
    .cap .ln {{ white-space: nowrap; }}
    .cap span {{ display: inline-block; }}
  </style>
  <div id="root" data-composition-id="app" data-width="{W}" data-height="{H}">
    <div id="app-stage" data-layout-allow-overflow data-layout-allow-overlap data-layout-allow-occlusion>
      {card('light')}
      {card('dark')}
      {cap_html}
    </div>
  </div>
  <script>
    (() => {{
      const tl = gsap.timeline({{ paused: true }});
      gsap.set(".cam", {js(initial)});
      gsap.set(".doc", {{ y: 0 }});
      gsap.set(".cap", {{ opacity: 0 }});
      gsap.set("#cap0", {{ opacity: 1 }});
      {app_js}
      window.__timelines["app"] = tl;
    }})();
  </script>
</template>
"""
(ROOT / "compositions").mkdir(exist_ok=True)
(ROOT / "compositions/app.html").write_text(app)

# ---------------------------------------------------------------- END (33 - 38)
LX, LY = CARDS["light"]
DX, DY = CARDS["dark"]
ICON = 180
ix = W / 2 - ICON / 2  # 450
seam = (LY + CARD_H + DY) / 2  # 762
end = f"""<template>
  <style>
    #root {{ position: absolute; inset: 0; background: #0b6bd6; overflow: hidden; font-family: Inter, sans-serif; }}
    #end-stage {{ position: absolute; inset: 0; overflow: hidden; }}
    .ec {{ position: absolute; width: {CARD_W}px; height: {CARD_H}px; }}
    .ec-l {{ left: {LX}px; top: {LY}px; background: #ffffff; }}
    .ec-d {{ left: {DX}px; top: {DY}px; background: #0d1117; }}
    .ewhite {{ position: absolute; inset: 0; background: #ffffff; clip-path: inset(0 0 100% 0); }}
    .escan {{ position: absolute; left: 0; top: 0; width: {W}px; height: 5px; background: {ACCENT}; opacity: 0; }}
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
      <div class="escan"></div>
      <img class="elogo" src="assets/images/logo.png" alt="MD View app icon" />
      <div class="et ew">MD View</div>
      <div class="et ecta">Download 1.0.0-rc.2</div>
      <div class="et eos">macOS · Windows · Linux · free &amp; open source</div>
      <div class="et eurl">github.com/jigonzalez930209/md-view-rs</div>
    </div>
  </div>
  <script>
    (() => {{
      const tl = gsap.timeline({{ paused: true }});
      gsap.set(".ew, .ecta, .eos, .eurl", {{ opacity: 0 }});
      // The two cards fold into the two halves of the icon (top half light, bottom half dark).
      tl.fromTo(".ec-l", {{ clipPath: "inset(0px 0px 0px 0px round 20px 20px 20px 20px)" }},
        {{ clipPath: "inset({CARD_H - ICON / 2:.0f}px {CARD_W - (ix - LX) - ICON:.0f}px 0px {ix - LX:.0f}px round 40px 40px 0px 0px)", duration: 0.7, ease: "power3.inOut" }}, 0.1);
      tl.fromTo(".ec-d", {{ clipPath: "inset(0px 0px 0px 0px round 20px 20px 20px 20px)" }},
        {{ clipPath: "inset(0px {CARD_W - (ix - DX) - ICON:.0f}px {CARD_H - ICON / 2:.0f}px {ix - DX:.0f}px round 0px 0px 40px 40px)", duration: 0.7, ease: "power3.inOut" }}, 0.1);
      tl.to(".ec-l", {{ y: {seam - (LY + CARD_H):.0f}, duration: 0.25, ease: "power2.inOut" }}, 0.6);
      tl.to(".ec-d", {{ y: {seam - DY:.0f}, duration: 0.25, ease: "power2.inOut" }}, 0.6);
      // One last repaint: the ground sweeps to white and the icon is painted where the halves met.
      tl.set(".escan", {{ opacity: 1, y: 0 }}, 0.85);
      tl.to(".escan", {{ y: {H - 5}, duration: 0.5, ease: "none" }}, 0.85);
      tl.to(".ewhite", {{ clipPath: "inset(0 0 0% 0)", duration: 0.5, ease: "none" }}, 0.85);
      tl.set(".escan", {{ opacity: 0 }}, 1.35);
      tl.set(".elogo", {{ opacity: 1 }}, {0.85 + 0.5 * (seam - ICON / 2) / H:.2f});
      tl.set(".ec", {{ opacity: 0 }}, {0.85 + 0.5 * (seam + ICON / 2) / H:.2f});
      // Lockup rises into place; wordmark and CTA land under it.
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

# ---------------------------------------------------------------- INDEX + SOUND
sfx = []


def s(i, t, f, v, d, lane=11):
    sfx.append(f'      <audio id="{i}" class="clip" data-start="{t:.2f}" data-duration="{d}" data-track-index="{lane}" data-volume="{v}" src="assets/sfx/{f}.mp3"></audio>')


s("sfx-fill", 0.48, "impact-bass-1", 0.4, 2.1)
for i in range(5):
    s(f"sfx-reg{i}", 0.6 + i * 0.15 - 0.04, "click" if i % 2 == 0 else "key-press", 0.45, 0.14, 12)
s("sfx-type", 0.9, "typing", 0.35, 1.4, 14)
s("sfx-resolve", 2.18, "pop", 0.4, 0.7, 12)
for i, t in enumerate([3.65, 6.35, 6.7, 8.85, 11.35, 13.85, 18.1, 21.2, 25.9, 28.4, 31.6]):
    s(f"sfx-tag{i}", t + 0.06, "click-soft" if i % 2 == 0 else "key-press", 0.4, 0.2, 13)
for i, t in enumerate([15.0, 20.5]):
    s(f"sfx-sweep{i}", t + 0.04, "whoosh-short", 0.3, 0.57, 12)
for i, t in enumerate([25.5, 28.0, 30.5]):
    s(f"sfx-ui{i}", t - 0.04, "click", 0.5, 0.2, 12)
s("sfx-logo", 33.0 + 0.85 + 0.5 * (762 - 90) / 1350 - 0.05, "impact-bass-1", 0.6, 2.1)

index = f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width={W}, height={H}" />
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>
    <style>
      :root {{ --ground: #0b6bd6; --ink: #ffffff; --accent: {ACCENT}; }}
      * {{ margin: 0; padding: 0; box-sizing: border-box; }}
      html, body {{ margin: 0; width: {W}px; height: {H}px; overflow: hidden; background: var(--ground); }}
      #root {{ position: relative; width: {W}px; height: {H}px; overflow: hidden; background: var(--ground); }}
    </style>
  </head>
  <body>
    <div id="root" data-composition-id="main" data-start="0" data-duration="38" data-width="{W}" data-height="{H}">
      <div id="app" data-composition-id="app" data-composition-src="compositions/app.html" data-start="0" data-duration="33" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>
      <div id="end" data-composition-id="end" data-composition-src="compositions/end.html" data-start="33" data-duration="5" data-track-index="0" data-track-kind="graphics" data-width="{W}" data-height="{H}"></div>

      <audio id="music" class="clip" data-start="0" data-duration="38" data-media-start="0.125" data-track-index="10" data-volume="0.8" data-fade-out="2" src="assets/audio/chronos.mp3"></audio>
{chr(10).join(sfx)}
    </div>
    <script>
      window.__timelines["main"] = gsap.timeline({{ paused: true }});
    </script>
  </body>
</html>
"""
(ROOT / "index.html").write_text(index)
print("ok")
