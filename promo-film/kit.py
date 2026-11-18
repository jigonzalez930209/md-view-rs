"""Shared building blocks for the MD View promo films (HyperFrames).

The films cut real MD View footage: 1920 x 1000 captures at 150 % UI scale, made by
linkedin/tools (launch.sh, capture.py, capture_equations.py, sync_events.py). A "panel" shows
one region of a capture, scaled, anywhere in the frame; several panels of the same recording,
started together, stay frame-locked because they share src, start, and media offset.
"""
import json
from pathlib import Path

ACCENT = "#e8336d"
BRAND = "#0b6bd6"
VERSION = "1.0.0-rc.5"
REPO = "github.com/jigonzalez930209/md-view-rs"
CAPTURE_W, CAPTURE_H = 1920, 1000


def events(root, theme, name):
    """The synced event log of a recording (each event has `video_t`)."""
    return json.loads((Path(root) / f"assets/cap/{theme}/{name}.json").read_text())


def at(log, kind, nth=0, **match):
    """Video time of the nth event of `kind` whose fields match."""
    hits = [e for e in log if e["kind"] == kind and all(e.get(k) == v for k, v in match.items())]
    return hits[nth]["video_t"]


def panel(pid, src, region, scale, left, top, *, video=None, radius=18, extra=""):
    """A region (x, y, w, h) of a capture, scaled and placed at (left, top) in the frame.

    `video` is (start, duration, media_start, track) for a recording; None shows a still.
    """
    x, y, w, h = region
    inner = f"left:{-x * scale:.1f}px; top:{-y * scale:.1f}px; width:{CAPTURE_W * scale:.1f}px; height:{CAPTURE_H * scale:.1f}px"
    if video:
        start, duration, media_start, track = video
        media = (f'<video id="{pid}-v" class="clip" style="position:absolute; {inner}" data-start="{start:.3f}" '
                 f'data-duration="{duration:.3f}" data-media-start="{media_start:.3f}" data-track-index="{track}" '
                 f'muted playsinline src="{src}"></video>')
    else:
        media = f'<img style="position:absolute; {inner}" src="{src}" alt="" />'
    return (f'<div id="{pid}" class="panel" style="left:{left:.1f}px; top:{top:.1f}px; width:{w * scale:.1f}px; '
            f'height:{h * scale:.1f}px; border-radius:{radius}px; {extra}">{media}</div>')


PANEL_CSS = ".panel { position: absolute; overflow: hidden; opacity: 0; }"


def caption(cid, lines, cls="cap"):
    words = "".join('<div class="ln">' + " ".join(f"<span>{w}</span>" for w in line.split(" ")) + "</div>"
                    for line in lines)
    return f'<div class="{cls}" id="{cid}">{words}</div>'


def caption_in(cid, t, stagger=0.06):
    return [f'tl.set("#{cid}", {{ opacity: 1 }}, {t:.2f});',
            f'tl.fromTo("#{cid} span", {{ opacity: 0, filter: "blur(10px)", y: 18 }}, '
            f'{{ opacity: 1, filter: "blur(0px)", y: 0, duration: 0.22, ease: "power3.out", stagger: {stagger} }}, {t:.2f});']


def caption_out(cid, t):
    return [f'tl.to("#{cid}", {{ opacity: 0, filter: "blur(8px)", duration: 0.14, ease: "power2.in" }}, {t - 0.14:.2f});']


def show(pid, t, how="cut", duration=0.35):
    if how == "cut":
        return [f'tl.set("#{pid}", {{ opacity: 1 }}, {t:.2f});']
    if how == "rise":
        return [f'tl.fromTo("#{pid}", {{ opacity: 0, y: 40 }}, {{ opacity: 1, y: 0, duration: {duration}, ease: "power3.out" }}, {t:.2f});']
    if how == "wipe":
        return [f'tl.set("#{pid}", {{ opacity: 1 }}, {t:.2f});',
                f'tl.fromTo("#{pid}", {{ clipPath: "inset(0 100% 0 0 round 18px)" }}, '
                f'{{ clipPath: "inset(0 0% 0 0 round 18px)", duration: {duration}, ease: "power3.inOut" }}, {t:.2f});']
    raise ValueError(how)


def hide(pid, t):
    return [f'tl.set("#{pid}", {{ opacity: 0 }}, {t:.2f});']


def sfx(sid, t, name, volume, duration, lane=11):
    return (f'      <audio id="{sid}" class="clip" data-start="{t:.2f}" data-duration="{duration}" '
            f'data-track-index="{lane}" data-volume="{volume}" src="assets/sfx/{name}.mp3"></audio>')


def index_html(width, height, total, body, audio, ground):
    return f"""<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width={width}, height={height}" />
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>
    <style>
      * {{ margin: 0; padding: 0; box-sizing: border-box; }}
      html, body {{ margin: 0; width: {width}px; height: {height}px; overflow: hidden; background: {ground}; }}
      #root {{ position: relative; width: {width}px; height: {height}px; overflow: hidden; background: {ground}; }}
    </style>
  </head>
  <body>
    <div id="root" data-composition-id="main" data-start="0" data-duration="{total:g}" data-width="{width}" data-height="{height}">
{body}
{chr(10).join(audio)}
    </div>
    <script>
      window.__timelines["main"] = gsap.timeline({{ paused: true }});
    </script>
  </body>
</html>
"""


def composition(cid, width, height, css, html, timeline, initial=""):
    lines = "\n      ".join(timeline)
    return f"""<template>
  <style>
    #root {{ position: absolute; inset: 0; overflow: hidden; font-family: Inter, sans-serif; }}
    {PANEL_CSS}
    {css}
  </style>
  <div id="root" data-composition-id="{cid}" data-width="{width}" data-height="{height}">
    <div id="{cid}-stage" style="position:absolute; inset:0; overflow:hidden" data-layout-allow-overflow data-layout-allow-overlap data-layout-allow-occlusion>
      {html}
    </div>
  </div>
  <script>
    (() => {{
      const tl = gsap.timeline({{ paused: true }});
      {initial}
      {lines}
      window.__timelines["{cid}"] = tl;
    }})();
  </script>
</template>
"""
