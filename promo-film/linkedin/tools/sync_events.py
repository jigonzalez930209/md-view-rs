#!/usr/bin/env python3
"""Align recorded event logs with their videos.

Usage: sync_events.py <capture dir>
ffmpeg's first frame lands a variable time after it starts, so each recording is anchored on a
visible change: the dialog opening (eq-hover), the first typed character (eq-build), and the
gallery popup (eq-gallery). Every event gets `video_t`, its time in the video.
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

# name: (crop x, y, w, h), share of changed pixels, anchor event kind, delay from it to the change
ANCHORS = {
    "eq-hover": ((300, 110, 400, 60), 0.05, "dblclick", 0.12),
    "eq-build": ((390, 436, 400, 26), 0.01, "type", 0.15),
    "eq-gallery": ((820, 160, 640, 200), 0.01, "click", 0.25),
}


def first_change(video, crop, share):
    """Time of the first frame where `share` of the crop's pixels moved by more than 40 levels.

    Counting changed pixels, not the mean, catches a popup that has the same overall brightness
    as what it covers (the light theme's gallery over the light editor)."""
    x, y, w, h = crop
    raw = subprocess.run(["ffmpeg", "-loglevel", "error", "-i", str(video), "-vf", f"crop={w}:{h}:{x}:{y}",
                          "-f", "rawvideo", "-pix_fmt", "gray", "-"], capture_output=True, check=True).stdout
    frames = np.frombuffer(raw, np.uint8).reshape(-1, h, w).astype(int)
    moved = (np.abs(frames - frames[0]) > 40).mean(axis=(1, 2))
    return np.where(moved > share)[0][0] / 30.0


folder = Path(sys.argv[1])
for name, (crop, threshold, kind, delay) in ANCHORS.items():
    log = folder / f"{name}.json"
    events = json.loads(log.read_text())
    anchor = next(e["t"] for e in events if e["kind"] == kind)
    offset = first_change(folder / f"{name}.mp4", crop, threshold) - (anchor + delay)
    for event in events:
        event["video_t"] = round(event["t"] + offset, 3)
    log.write_text(json.dumps(events, indent=1))
    print(f"{name}: offset {offset:+.3f} s")
