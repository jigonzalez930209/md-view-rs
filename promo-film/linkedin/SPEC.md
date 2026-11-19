# SPEC — MD View · LinkedIn cut (A × B × C)

GOAL: In under a minute a developer scrolling LinkedIn sees how fast and light MD View starts, every part of it working in light and dark at once, and the built-in video, PDF and CSV viewers.

OUTPUT: HyperFrames, 1080 × 1350 (4:5, LinkedIn feed), 30 fps.

TIME: 57.5 s. "Chronos" (CC0) at 120 BPM, `data-media-start=0.125` so beats land on every 0.5 s; a second copy from 20.125 s crossfades in at 43 s.

LOOK (1.0.0-rc.8 cut):
- Ground: flat #0b6bd6 (the app icon's blue). No gradients, glow, glass, grain, outlines or accent lines.
- A 225 px band on top for captions and numbers; below it two full-width screens, 1080 × 562.5 each (the 1920 × 1000 capture at 0.5625), stacked with no gap: light at y 225, dark at y 787.5. No radius, no shadow.
- Captions: Inter 900, 66 px, white, left at x 60, top 44; max 2 lines.
- Numbers: labels 28 px uppercase, values 52 px tabular figures, a 28 px note under them.

MOTION:
- Camera (both screens, one tween): power3.inOut 0.6 s, clamped so the window always fills the screen.
- Scroll: power2.inOut 0.8 s. Mode changes: a clean top-to-bottom wipe, 0.4 s, no line.
- UI actions are hard cuts. Captions blur in per word; numbers count up frame by frame.

SHOTS:
- 0.0–2.5 COLD START. Black screens; the counter runs from exec (0.5 s) to the measured first frame (157 ms, shown 10× slower); the two real first frames of the launch recording appear at their times.
- 2.5–7.5 FOOTPRINT. Cold start column (peak RAM, CPU over the first 2 s), then at 4.5 the loaded-and-idle column; the note names the machine.
- 7.5–43.5 TOUR. Explorer, reading tour (tables, LaTeX, Mermaid, alerts), editor with regex find and replace, split view, equations, palette, workspace search, themes — the earlier cut moved by 4.5 s.
- 43.5–52.5 WATCH. Video playback (fractal, then xvid.avi), PDF viewer, CSV grid.
- 52.5–57.5 END. The screens fold into the two halves of the icon; wordmark, "Download 1.0.0-rc.8", platforms, URL.

NUMBERS (tools/metrics.json): first frame from tools/measure.py (Xvfb, median of 5); RAM and CPU from the same procedure on the real GPU session (median of 3). MiB are shown as MB (× 1.048576).
