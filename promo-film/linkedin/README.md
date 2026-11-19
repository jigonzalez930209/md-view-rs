# MD View — LinkedIn promo film

A 57.5 s, 1080 × 1350 (4:5) HyperFrames film: GitHub Light above, GitHub Dark below, two
full-width screens with no gap, the same app state in both. It opens on the measured cold start
(shown ten times slower) and the memory and CPU footprint, tours every feature including the
equation gallery and formula editor, and ends on the 1.0.0-rc.8 video player, PDF and CSV
viewers. The rendered file is `../out/linkedin-4x5.mp4`. Three alternative
cuts built from the same footage live next to this project (see `../README.md`).

- `SPEC.md` — the film's brief; `REVIEW.md` — review notes.
- `assets/cap/{light,dark}` — real MD View captures (1920 × 1000, 150 % UI scale) and the
  split-view scroll recordings.
- `tools/launch.sh`, `tools/capture.py`, `tools/record_split.py` — drive a release build in
  Xvfb to recapture both themes in identical state (requires Xvfb, xwininfo, xsetroot, ffmpeg).
- `tools/stitch.py` — stitches the reading-view captures into `doc-tall.png`.
- `tools/capture_equations.py` — records the formula editor (double-click and hover, the build
  from palette blocks, the gallery) with `tools/docs/equations.md`, plus an event log per
  recording; `tools/sync_events.py` aligns each log with its video (`video_t`), and
  `tools/contact_sheet.py` renders a review sheet at the logged events.
- `tools/measure.py` — five cold starts in Xvfb: time to the first painted window, peak RSS
  and CPU over the first two seconds, RSS and CPU at idle. `tools/metrics.json` holds the
  numbers the film shows (first frame from `measure.py`; RAM and CPU from the same procedure
  run on the real GPU session, since Xvfb renders in software).
- `tools/capture_media.py` — per theme: the launch recording (`launch-1/2.png` are its first
  two painted frames), the video playback recording, and the PDF and CSV viewers.
- `tools/xdrive.py` — XTest pointer, keys, and capture helpers (no xdotool).
- `tools/gen_film.py` — generates `index.html` and `compositions/` from the captures, and the
  recording flipbooks in `assets/frames/` (see `REVIEW.md`).

Recapture a theme (one instance at a time: promo instances share a user name):

```sh
tools/launch.sh github-dark :95 /tmp/promo-dark "$PWD/tools/docs/equations.md"
python3 tools/capture_equations.py :95 /tmp/eq/dark && python3 tools/sync_events.py /tmp/eq/dark
```

Render: `npx hyperframes render -o ../out/linkedin-4x5.mp4`.
Music: "Chronos" from FreePD (CC0). Sound effects: HyperFrames bundled library.
