#!/usr/bin/env python3
"""Measure MD View's cold start and its idle footprint in a private Xvfb display.

Usage: measure.py <display> <workdir> [runs]
Each run starts a fresh X server and a fresh config, launches the release build on the
showcase document, and records:
  - first frame: milliseconds from exec until the window shows its painted sidebar;
  - startup:     peak RSS and mean CPU over the first two seconds;
  - idle:        RSS and mean CPU over five seconds, ten seconds after launch.
CPU is a share of one core. The medians are printed as JSON for tools/gen_film.py.
"""
import ctypes
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
disp, work = sys.argv[1:3]
runs = int(sys.argv[3]) if len(sys.argv) > 3 else 5
work = Path(work)
TICK = os.sysconf("SC_CLK_TCK")
PAGE = os.sysconf("SC_PAGE_SIZE")
SIDEBAR = (0xF6, 0xF8, 0xFA)  # GitHub Light sidebar


def cpu_seconds(pid):
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return (int(fields[11]) + int(fields[12])) / TICK


def rss_mib(pid):
    return int(Path(f"/proc/{pid}/statm").read_text().split()[1]) * PAGE / 2**20


class Pixels:
    def __init__(self, name):
        self.x11 = ctypes.CDLL("libX11.so.6")
        self.x11.XOpenDisplay.restype = ctypes.c_void_p
        self.x11.XDefaultRootWindow.restype = ctypes.c_ulong
        self.x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
        self.x11.XGetImage.restype = ctypes.c_void_p
        self.x11.XGetImage.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_int,
                                       ctypes.c_uint, ctypes.c_uint, ctypes.c_ulong, ctypes.c_int]
        self.x11.XGetPixel.restype = ctypes.c_ulong
        self.x11.XGetPixel.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
        self.x11.XDestroyImage = None
        self.dpy = self.x11.XOpenDisplay(name.encode())
        self.root = self.x11.XDefaultRootWindow(self.dpy)

    def at(self, x, y):
        image = self.x11.XGetImage(self.dpy, self.root, x, y, 1, 1, 0xFFFFFFFF, 2)
        value = self.x11.XGetPixel(image, 0, 0)
        return (value >> 16) & 255, (value >> 8) & 255, value & 255


def one_run(index):
    run = work / f"run{index}"
    subprocess.run(["rm", "-rf", str(run)], check=True)
    subprocess.run(["mkdir", "-p", str(run)], check=True)
    launch = (ROOT / "promo-film/linkedin/tools/launch.sh").read_text()
    config = launch.split("<<EOF\n", 1)[1].split("\nEOF\n", 1)[0].replace("$THEME", "github-light")
    (run / "config").mkdir()
    (run / "logs").mkdir()
    (run / "runtime").mkdir(mode=0o700)
    (run / "config/config.toml").write_text(config + "\n")
    subprocess.run(["cp", "-r", str(ROOT / "examples/v1-workspace"), str(run / "ws")], check=True)
    xvfb = subprocess.Popen(["Xvfb", disp, "-screen", "0", "1920x1000x24"], stderr=subprocess.DEVNULL)
    time.sleep(1.0)
    pixels = Pixels(disp)
    env = dict(os.environ, DISPLAY=disp, XDG_SESSION_TYPE="x11", USER="mdview_promo",
               XDG_RUNTIME_DIR=str(run / "runtime"), MDVIEW_CONFIG_DIR=str(run / "config"),
               MDVIEW_LOG_DIR=str(run / "logs"), RUST_LOG="warn")
    env.pop("WAYLAND_DISPLAY", None)
    show = run / "ws/level1/level2/level3/level4/level5/showcase.md"
    start = time.monotonic()
    app = subprocess.Popen([str(ROOT / "target/release/mdview"), str(run / "ws"), str(show)], env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    first = None
    peak = 0.0
    while time.monotonic() - start < 10.0:
        peak = max(peak, rss_mib(app.pid))
        if first is None and pixels.at(40, 300) == SIDEBAR and pixels.at(1100, 200) == (255, 255, 255):
            first = time.monotonic() - start
        if time.monotonic() - start >= 2.0 and first is not None:
            break
        time.sleep(0.004)
    startup_cpu = cpu_seconds(app.pid) / (time.monotonic() - start)
    startup_peak = peak
    time.sleep(max(0.0, 10.0 - (time.monotonic() - start)))
    c0, t0 = cpu_seconds(app.pid), time.monotonic()
    time.sleep(5.0)
    idle_cpu = (cpu_seconds(app.pid) - c0) / (time.monotonic() - t0)
    idle_rss = rss_mib(app.pid)
    app.terminate()
    app.wait(timeout=5)
    xvfb.terminate()
    xvfb.wait(timeout=5)
    time.sleep(0.5)
    result = dict(first_frame_ms=round((first or 0) * 1000), startup_peak_mib=round(startup_peak, 1),
                  startup_cpu_pct=round(startup_cpu * 100, 1), idle_rss_mib=round(idle_rss, 1),
                  idle_cpu_pct=round(idle_cpu * 100, 2))
    print(f"run {index}: {result}", file=sys.stderr)
    return result


results = [one_run(i) for i in range(runs)]
median = {key: statistics.median(r[key] for r in results) for key in results[0]}
print(json.dumps(dict(runs=results, median=median), indent=2))
