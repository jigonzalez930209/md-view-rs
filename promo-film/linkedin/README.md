# MD View — LinkedIn promo film

A 38 s, 1080 × 1350 (4:5) HyperFrames film: GitHub Light above, GitHub Dark below, the same
app state in both, painted and toured feature by feature. The rendered file is
`../out/linkedin-4x5.mp4`.

- `SPEC.md` — the film's brief; `REVIEW.md` — review notes.
- `assets/cap/{light,dark}` — real MD View captures (1920 × 1000, 150 % UI scale) and the
  split-view scroll recordings.
- `tools/launch.sh`, `tools/capture.py`, `tools/record_split.py` — drive a release build in
  Xvfb to recapture both themes in identical state (requires Xvfb, xdotool, ffmpeg).
- `tools/stitch.py` — stitches the reading-view captures into `doc-tall.png`.
- `tools/gen_film.py` — generates `index.html` and `compositions/` from the captures.

Render: `npx hyperframes render -o ../out/linkedin-4x5.mp4`.
Music: "Chronos" from FreePD (CC0). Sound effects: HyperFrames bundled library.
