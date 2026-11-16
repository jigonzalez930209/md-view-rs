#!/usr/bin/env python3
"""Record the split-view scroll for one theme with wall-clock timestamps (CFR 30 output).
Usage: record_split.py <display> <out.mp4>   (app already launched by launch.sh)"""
import os, subprocess, sys, time
from pathlib import Path
disp, out = sys.argv[1:3]
X11 = str(Path(__file__).resolve().parents[3] / "scripts/e2e/x11.py")
env = dict(os.environ, DISPLAY=disp)
def x(*a): subprocess.run(["python3", X11, *map(str, a)], env=env, check=True)
wid = subprocess.run(["xdotool", "search", "--name", "MD View"], env=env, capture_output=True, text=True).stdout.split()[0]
x("focus", hex(int(wid)))
x("key", "ctrl+shift+e"); time.sleep(1.0)
x("click", 1000, 400); time.sleep(0.4)
x("key", "ctrl+Home"); time.sleep(0.4)
subprocess.run(["xdotool", "mousemove", "160", "940"], env=env); time.sleep(0.6)
p = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-use_wallclock_as_timestamps", "1", "-f", "x11grab",
                      "-draw_mouse", "0", "-framerate", "30", "-video_size", "1920x1000", "-i", disp, "-t", "5.5",
                      "-fps_mode", "cfr", "-r", "30", "-c:v", "libx264", "-preset", "veryfast", "-crf", "14",
                      "-g", "30", "-pix_fmt", "yuv420p", "-movflags", "+faststart", out])
time.sleep(0.6)
for _ in range(26):
    x("wheel", 900, 520, 1); time.sleep(0.12)
p.wait()
