"""XTest helpers for promo captures: pointer, keys, screenshots, and recordings, without xdotool.

Keys and text go through the E2E driver (scripts/e2e/x11.py); the pointer is moved here so a
capture can glide it smoothly, the way a person moves a mouse.
"""
import ctypes
import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
X11 = ROOT / "scripts/e2e/x11.py"


class Display:
    def __init__(self, name):
        self.name = name
        self.env = dict(os.environ, DISPLAY=name)
        self.x11 = ctypes.CDLL("libX11.so.6")
        self.xtst = ctypes.CDLL("libXtst.so.6")
        self.x11.XOpenDisplay.restype = ctypes.c_void_p
        self.dpy = ctypes.c_void_p(self.x11.XOpenDisplay(name.encode()))
        self.pos = (160, 940)

    def run(self, *args):
        subprocess.run(["python3", str(X11), *map(str, args)], env=self.env, check=True)

    def key(self, chord, wait=0.4):
        self.run("key", chord)
        time.sleep(wait)

    def text(self, value, wait=0.4):
        self.run("text", value)
        time.sleep(wait)

    def move(self, x, y, wait=0.0):
        self.xtst.XTestFakeMotionEvent(self.dpy, -1, int(x), int(y), 0)
        self.x11.XFlush(self.dpy)
        self.pos = (x, y)
        time.sleep(wait)

    def glide(self, x, y, seconds=0.5, steps=None):
        """Move the pointer along an eased path, so recordings show a hand, not a jump."""
        x0, y0 = self.pos
        steps = steps or max(2, int(seconds * 60))
        for i in range(1, steps + 1):
            t = i / steps
            e = t * t * (3 - 2 * t)
            self.move(x0 + (x - x0) * e, y0 + (y - y0) * e)
            time.sleep(seconds / steps)

    def button(self, pressed, number=1):
        self.xtst.XTestFakeButtonEvent(self.dpy, number, 1 if pressed else 0, 0)
        self.x11.XFlush(self.dpy)

    def click(self, x, y, count=1, wait=0.5):
        self.move(x, y, 0.15)
        for _ in range(count):
            self.button(True)
            time.sleep(0.04)
            self.button(False)
            time.sleep(0.06)
        time.sleep(wait)

    def park(self, x=160, y=940):
        self.move(x, y, 0.3)

    def focus(self, timeout=10.0):
        """Focus the MD View window, waiting for it to map. Only one promo instance may run at
        a time: they share a user name, so a second launch is forwarded to the first."""
        deadline = time.monotonic() + timeout
        while True:
            tree = subprocess.run(["xwininfo", "-root", "-tree"], env=self.env, capture_output=True, text=True).stdout
            window = next((line.split()[0] for line in tree.splitlines() if "MD View" in line), None)
            if window:
                self.run("focus", window)
                return
            if time.monotonic() > deadline:
                raise RuntimeError(f"no MD View window on {self.name}; is another promo instance running?")
            time.sleep(0.3)

    def shot(self, path, wait=0.5, mouse=False):
        time.sleep(wait)
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "x11grab", "-draw_mouse", "1" if mouse else "0",
                        "-video_size", "1920x1000", "-i", self.name, "-frames:v", "1", str(path)], check=True)

    def record(self, path, seconds, actions, mouse=True):
        """Record with wall-clock timestamps at a constant 30 fps while `actions` runs.

        No B-frames and a keyframe every 15 frames: the HyperFrames renderer never finished
        initializing a composition with two default-GOP split recordings, and seek-friendly
        encodes also keep frame extraction fast."""
        process = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-use_wallclock_as_timestamps", "1",
                                    "-f", "x11grab", "-draw_mouse", "1" if mouse else "0", "-framerate", "30",
                                    "-video_size", "1920x1000", "-i", self.name, "-t", str(seconds), "-fps_mode", "cfr",
                                    "-r", "30", "-c:v", "libx264", "-preset", "veryfast", "-crf", "14", "-g", "15", "-bf", "0",
                                    "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(path)])
        time.sleep(0.6)
        actions()
        process.wait()
