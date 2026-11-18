# SPEC — MD View · LinkedIn cut (A × B × C)

GOAL: In under 40 s a developer scrolling LinkedIn sees every part of MD View working, in light and dark at once, and remembers "native Markdown, every feature, both themes".

OUTPUT: HyperFrames, 1080 × 1350 (4:5, LinkedIn feed), 30 fps.

TIME: 44 s = 1320 frames. "Chronos" (CC0) at 120 BPM, `data-media-start=0.125` so beats land on every 0.5 s. Section changes land on beats: 3.0, 5.5, 8.0, 10.5, 13.0, 15.0, 17.5, 20.5, 25.5, 27.5, 29.5, 31.5, 34.0, 36.5, 39.0.

FROM EACH FILM:
- A: the tour of the real document (headings, tables, math, diagrams) and the claim "drawn natively"; no glyph dives.
- B: the paint-order design — strokes → flat fills → grey glyph runs → real pixels; brand-blue ground; raster-sweep repaints; outlined blocks with tags.
- C: two worlds locked together — the same app state twice, GitHub Light above, GitHub Dark below, every camera move, scroll and video frame identical.

LOOK:
- Ground: flat #0b6bd6 (the app icon's blue). No gradients, glow, glass, grain.
- Two cards, 980 × 490, radius 20, x 50: light at y 262, dark at y 772 (20 px gap). Shadow 0 30 px 70 px rgba(3,25,60,.45).
- Material: real MD View captures (1.0.0-rc.5 build), 1920 × 1000 at 150 % UI scale, scripted in Xvfb so both themes share exact state (tools/capture.py). Reading view = one tall page stitched from 7 captures at measured 480 px offsets.
- Accent: #e8336d — feature outlines (4 px on screen), tags (white Inter 800, 30 px on screen), scanline.
- Captions: Inter 900, 66 px, line-height 1.04, white, left-aligned at x 60, top 84; max 2 lines, ≤ 3 words a line.

MOTION:
- Camera (both cards, one tween): power3.inOut, 0.6 s. Scroll (both cards): power2.inOut, 0.8 s.
- Repaint sweeps: clip-path top→bottom 0.4 s linear with the scanline riding the edge.
- UI actions (palette opens, query typed, dropdown opens) are hard cuts — that is how the UI behaves.
- Outlines draw by clip in 0.25 s; tags pop with back.out(1.6) 0.3 s. Captions blur in per word (8 px → 0, 0.15 s, 0.06 s stagger); exit blur 0.12 s.

SHOTS:
- 0.0–3.0 PAINT. Card edges stroke, fills (sampled theme colours), 70 glyph runs at their real positions, real pixels at 2.3. "Markdown, drawn natively."
- 3.0–5.5 EXPLORER. Push to the tree. Tag "explorer".
- 5.5–15.0 READ. Camera to the reading column; the page scrolls in both cards to tables+tasks (5.5), LaTeX (8.0), Mermaid (10.5), alerts (13.0); each stop outlined and tagged.
- 15.0–20.5 EDIT. Sweep to Edit mode; push to the find bar; regex `level\d` with `.*` on; replace row opens at 19.0.
- 20.5–25.5 SPLIT. Sweep to Split mode; the real scroll recordings play in both themes; tags "source" / "preview".
- 25.5–31.5 EQUATIONS. Sweep to the editor with the Σ and √x buttons; cut to the open gallery, tagged "Σ gallery"; push to the formula editor where the real hover recording plays in both themes (ħ, ∂/∂t, t, Ψ light up in the preview as the pointer crosses the source), tagged "hover the LaTeX" and "live preview".
- 31.5–39.0 COMMANDS. Palette (Ctrl+Shift+P) → "mode"; workspace search (Ctrl+Shift+F) "Mermaid"; Settings → theme list.
- 39.0–44.0 END. Pixels leave, the two cards fold into the two halves of the icon, the ground repaints white, icon + wordmark + CTA.

BANNED: dark void, glow/particles, gradients, glass cards, cream paper + serif, centred-everything, fade + slide-up, titles moving after landing, three cards in a row, docked logo, invented UI, glyph dives.

QA: t=0 caption + edges; 2.5 real pixels both cards; 4.5 explorer tag; 7, 9.5, 12, 14.5 tags on the right blocks in both cards; 18.5 regex result; 23 both videos mid-scroll at matching positions; 26.8 "mode" palette; 29 search; 32 theme list; 37.5 end card inside title-safe.
