#!/usr/bin/env python3
"""Drive MD View in Xvfb through a fixed script and capture each state.

Usage: capture.py <theme> <display> <workdir> <outdir> [--explore]
The same script runs for every theme, so light and dark captures share exact state.
"""
import os, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
X11 = ROOT / "scripts/e2e/x11.py"
theme, disp, work, out = sys.argv[1:5]
explore = "--explore" in sys.argv
out = Path(out); out.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, DISPLAY=disp)

def x(*args):
    subprocess.run(["python3", str(X11), *map(str, args)], env=env, check=True)
def key(k, wait=0.4): x("key", k); time.sleep(wait)
def text(t, wait=0.4): x("text", t); time.sleep(wait)
def click(a, b, wait=0.5): x("click", a, b); time.sleep(wait)
def wheel(a, b, n, wait=0.8): x("wheel", a, b, n); time.sleep(wait)
def wheelup(a, b, n, wait=0.8): x("wheelup", a, b, n); time.sleep(wait)
def park(): subprocess.run(["xdotool", "mousemove", "160", "940"], env=env); time.sleep(0.3)
def shot(name, wait=0.5):
    time.sleep(wait)
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "x11grab", "-draw_mouse", "0",
                    "-video_size", "1920x1000", "-i", disp, "-frames:v", "1", str(out / f"{name}.png")], check=True)
def record(name, seconds, actions):
    p = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "x11grab", "-draw_mouse", "0",
                          "-framerate", "30", "-video_size", "1920x1000", "-i", disp, "-t", str(seconds),
                          "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "14", str(out / f"{name}.mp4")])
    time.sleep(0.4)
    actions()
    p.wait()

def mode(name):
    key("ctrl+shift+p"); text(name); key("Return", 0.7)

wid = subprocess.run(["xdotool", "search", "--name", "MD View"], env=env, capture_output=True, text=True).stdout.split()[0]
x("focus", hex(int(wid)))
park()
shot("01-view-top", 1.0)
for name, n in [("02-view-tables", 4), ("03-view-tasks", 4), ("04-view-latex", 4), ("04b-view-images", 4), ("05-view-mermaid", 4), ("06-view-alerts", 4)]:
    wheel(1100, 520, n); park(); shot(name)
wheelup(1100, 520, 60); park()

key("ctrl+e", 0.8); click(1200, 400); key("ctrl+Home"); park(); shot("07-edit-top")
key("ctrl+f", 0.6); text("level"); park(); shot("08-find")
key("ctrl+a"); text("level\\d"); click(1677, 159, 0.6); park(); shot("08b-find-regex")
key("Escape")
key("ctrl+h", 0.6); park(); shot("09-replace")
key("Escape")
key("ctrl+g", 0.6); text("35"); park(); shot("10-goto")
key("Escape")

key("ctrl+shift+e", 0.9); click(1000, 400); key("ctrl+Home"); park(); shot("11-split-top")
def scroll_split():
    for _ in range(26):
        x("wheel", 900, 520, 1); time.sleep(0.12)
record("12-split-scroll", 5.0, scroll_split)
park(); shot("12b-split-end")
wheelup(900, 520, 60); park()

key("ctrl+shift+p", 0.6); park(); shot("13-palette")
text("mode"); park(); shot("14-palette-mode")
key("Escape")
key("ctrl+shift+f", 0.6); text("Mermaid", 1.5); park(); shot("16-search")
key("Escape")
key("ctrl+comma", 0.8); park(); shot("17-settings")
click(1160, 327, 0.7); park(); shot("17b-settings-themes")
key("Escape")
key("Escape")
