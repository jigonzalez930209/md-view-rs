#!/usr/bin/env python3
"""Capture the 1.0.0-rc.8 material in both themes: the cold start, video playback, PDF and CSV.

Usage: capture_media.py <display> <home> <outdir>
<home> holds Documents/Videos/Format samples (video clips) and Documents/Project-notes/ws
(a copy of examples/v1-workspace), so the status bar shows neutral paths. Writes, per theme:
  launch.mp4      the screen from before exec until the window is painted (exec about 0.6 s in)
  video-play.mp4  play the fractal, then xvid.avi, then h264.ts
  viewer-pdf.png, viewer-csv.png
Same 1920 x 1000 display and 150 % UI scale as tools/capture.py.
"""
import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from xdrive import Display  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
disp, home, out = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
VIDEOS = home / "Documents/Videos/Format samples"
NOTES = home / "Documents/Project-notes"
CONFIG = (ROOT / "promo-film/linkedin/tools/launch.sh").read_text().split("<<EOF\n", 1)[1].split("\nEOF\n", 1)[0]


class Session:
    """A fresh X server and a fresh MD View config for one theme."""

    def __init__(self, theme, work):
        self.work = work
        subprocess.run(["rm", "-rf", str(work / "config"), str(work / "logs"), str(work / "runtime")], check=True)
        for name in ("config", "logs", "runtime"):
            (work / name).mkdir(parents=True, exist_ok=True)
        os.chmod(work / "runtime", 0o700)
        (work / "config/config.toml").write_text(CONFIG.replace("$THEME", theme) + "\n")
        self.xvfb = subprocess.Popen(["Xvfb", disp, "-screen", "0", "1920x1000x24"], stderr=subprocess.DEVNULL)
        time.sleep(1.0)
        env = dict(os.environ, DISPLAY=disp, XCURSOR_THEME="Yaru", XCURSOR_SIZE="48")
        subprocess.run(["xsetroot", "-cursor_name", "left_ptr"], env=env, check=True)
        self.app = None

    def start(self, workspace, file):
        env = dict(os.environ, DISPLAY=disp, XDG_SESSION_TYPE="x11", USER="mdview_promo",
                   XDG_RUNTIME_DIR=str(self.work / "runtime"), MDVIEW_CONFIG_DIR=str(self.work / "config"),
                   MDVIEW_LOG_DIR=str(self.work / "logs"), RUST_LOG="warn")
        env.pop("WAYLAND_DISPLAY", None)
        self.app = subprocess.Popen([str(ROOT / "target/release/mdview"), str(workspace), str(file)], env=env,
                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def stop(self):
        if self.app:
            self.app.terminate()
            self.app.wait(timeout=5)
            self.app = None

    def close(self):
        self.stop()
        self.xvfb.terminate()
        self.xvfb.wait(timeout=5)
        time.sleep(0.5)


def capture(theme):
    folder = out / theme
    folder.mkdir(parents=True, exist_ok=True)
    showcase = NOTES / "ws/level1/level2/level3/level4/level5/showcase.md"

    # Cold start: the recording runs before exec, so the film shows the real wait.
    session = Session(f"github-{theme}", NOTES)
    display = Display(disp)
    display.record(folder / "launch.mp4", 2.5, lambda: session.start(NOTES / "ws", showcase), mouse=False)
    session.close()

    # Video playback.
    session = Session(f"github-{theme}", NOTES)
    session.start(VIDEOS, VIDEOS / "fractal-zoom.mp4")
    time.sleep(3.0)
    display = Display(disp)
    display.focus()
    display.park()
    time.sleep(0.5)

    def play():
        time.sleep(0.4)
        display.glide(373, 926, 0.5)
        display.click(373, 926, wait=0.1)  # play the fractal
        time.sleep(3.6)
        display.glide(98, 535, 0.6)
        display.click(98, 535, wait=0.6)  # xvid.avi
        display.glide(373, 926, 0.4)
        display.click(373, 926, wait=0.1)
        time.sleep(2.0)
        display.glide(98, 310, 0.5)
        display.click(98, 310, wait=0.6)  # h264.ts
        display.glide(373, 926, 0.4)
        display.click(373, 926, wait=0.1)
        time.sleep(2.0)

    display.record(folder / "video-play.mp4", 13.0, play)
    session.close()

    # PDF and CSV viewers.
    for name, file in [("viewer-pdf", NOTES / "ws/media/documents/guide.pdf"), ("viewer-csv", NOTES / "ws/media/tables/people.csv")]:
        session = Session(f"github-{theme}", NOTES)
        session.start(NOTES / "ws", file)
        time.sleep(3.0)
        display = Display(disp)
        display.focus()
        display.park()
        display.shot(folder / f"{name}.png", wait=1.0)
        session.close()


for theme in sys.argv[4:] or ["light", "dark"]:
    capture(theme)
    print(f"{theme}: done")
