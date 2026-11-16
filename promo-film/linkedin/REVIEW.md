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
