#!/usr/bin/env python3
"""Contact sheet of a recording at its logged events, for reviewing a capture.

Usage: contact_sheet.py <capture dir> <name> [x y w h]
Writes <capture dir>/sheet-<name>.png with one tile per event, 0.45 s after it.
"""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

folder, name = Path(sys.argv[1]), sys.argv[2]
x, y, w, h = map(int, sys.argv[3:7]) if len(sys.argv) > 6 else (300, 220, 1320, 580)
tiles = []
with tempfile.TemporaryDirectory() as tmp:
    for event in json.loads((folder / f"{name}.json").read_text()):
        t = event["video_t"] + 0.45
        frame = Path(tmp) / "f.png"
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-ss", f"{t:.2f}", "-i", str(folder / f"{name}.mp4"),
                        "-frames:v", "1", str(frame)], check=True)
        tile = Image.open(frame).crop((x, y, x + w, y + h))
        tile.thumbnail((640, 400))
        label = f"{t:.2f} {event['kind']} {event.get('label', event.get('target', event.get('text', '')))}"
        draw = ImageDraw.Draw(tile)
        draw.rectangle((0, 0, 330, 22), fill="black")
        draw.text((4, 4), label.encode("ascii", "replace").decode(), fill="yellow")
        tiles.append(tile)
tw, th = tiles[0].size
sheet = Image.new("RGB", (tw * 3, th * ((len(tiles) + 2) // 3)))
for i, tile in enumerate(tiles):
    sheet.paste(tile, ((i % 3) * tw, (i // 3) * th))
sheet.save(folder / f"sheet-{name}.png")
