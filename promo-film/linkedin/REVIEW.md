# REVIEW — LinkedIn cut (A × B × C)

`check`: 0 errors; 2 warnings (nested video `data-start` basis — intended, scene-local).
Rendered `../out/linkedin-4x5.mp4`: 1080 × 1350, 38.0 s, −15.7 LUFS, −1.0 dBTP (measured, not heard).

Checked frames: 0.1, 0.9, 1.9, 2.7, 4.6, 7.2, 9.6, 12.2, 14.4, 16.5, 18.6, 19.6, 22, 24, 26.2, 26.9, 27.2, 29.5, 32.2, 33.5, 34.2, 35, 37.5, plus 21.6–25.3 from the render for video sync.

Fixed during review:
- Page tags covered their headings ("LaTeX", "Alerts") → tags on page blocks are right-aligned.
- "table" and "Ctrl+Shift+P" tags were clipped by the card top → tables stop at scroll 715; palette framing 0.72× at y 490.
- Split recordings drifted (light capture dropped frames: 137 vs 150) → re-recorded both with wall-clock timestamps, CFR 30 (tools/record_split.py); content match shows ≤ 2 frames lag; offsets 0.333 / 0.300 s.

Known:
- Frame 0 is the caption plus the first card edges on blue — readable, sparse as a thumbnail.
- UI text inside the cards is ~9–18 px at 1080 wide; on a phone it reads as UI, not as text. Captions carry the words.
- Raw view captures 02–06 were removed after stitching doc-tall.png; tools/capture.py regenerates them (tools/stitch.py expects them in captures/<theme>/).

## 1.0.0-rc.5 update

Rendered `../out/linkedin-4x5.mp4`: 1080 × 1350, 44.0 s, with the equations section (25.5–31.5 s).

- The renderer never finished initializing this composition with its four `<video>` elements:
  `pollVideosReady` kept waiting on a page blocked in the probe (screenshot mode, higher
  timeouts, WebM, re-encoding without B-frames, and HyperFrames 0.9.65 all hung the same way;
  the same markup without videos rendered in 22 s, and the other three films render theirs).
  The split and hover recordings now play as flipbooks: `gen_film.py` extracts JPEG frames at
  display resolution into `assets/frames/` (generated, ignored by git) and the timeline shows
  one per frame. Deterministic, seek-safe, and frame-exact between the two themes.
- Recordings are encoded without B-frames and with a 15-frame GOP (`tools/xdrive.py`,
  `tools/record_split.py`) so seeks stay cheap.

Checked frames from the render: 2.5, 7.2, 23 (both themes mid-scroll at the same line), 26.9
(gallery), 28.6 and 30.6 (ħ, then ∂/∂t lit in both themes), 37.9, 43.5.

## No pointing (2026-11-18)

Removed the outlines, tags and the click per tag (`sfx-tag*`): nothing points at the screen;
the captions name each feature. `check`: 0 errors, the same 2 warnings.
Rendered `../out/linkedin-4x5.mp4`: 1080 × 1350, 44.0 s, −15.7 LUFS, −1.2 dBFS peak (measured,
not heard). Checked frames 4.5, 7.2, 9.5, 14, 18.6, 23, 26.8, 29.5, 32.2, 35, 38.2, 43.75 in
the preview and 7.2, 23, 38 from the render: no outline or tag left.

## 1.0.0-rc.7 update (2026-11-18)

End card reads "Download 1.0.0-rc.7". `17-settings` and `17b-settings-themes` recaptured in both
themes with the rc.7 release build, so the Settings footer reads "MD View 1.0.0-rc.7"; the rest of
the footage is unchanged. `check`: 0 errors, 2 warnings. Rendered 44.0 s, −15.7 LUFS, −1.2 dBFS
peak. Checked 23, 38.5 and 43.5 s from the render.
