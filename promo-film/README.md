# MD View promo films

Four HyperFrames films cut from the same real footage of MD View (1920 × 1000 captures at 150 %
UI scale, recorded in Xvfb by `linkedin/tools`). Each film is its own project with a generator
in `tools/gen*.py`; `kit.py` holds the shared panels, captions, and audio helpers. Rendered files
are in `out/`.

| Film | Format | Length | Platform | Idea |
|---|---|---|---|---|
| `linkedin/` | 1080 × 1350 (4:5) | 44 s | LinkedIn feed | Every feature, light above and dark below in lockstep; the equations section shows the gallery and the hover recording. |
| `vertical/` | 1080 × 1920 (9:16) | 22.5 s | Reels, Shorts, TikTok | Hook first: the LaTeX and the formula it draws, stacked from one recording, light up in frame 0; then the build, the gallery, a beat-cut montage, the end card. Text stays inside the strictest safe area of the three platforms. |
| `trailer/` | 1920 × 1080 (16:9) | 56 s | YouTube, website, README | Five numbered chapters (Read, Write, Math, Find, Fast) on one window card with a camera; real recordings for every interaction and the numbers measured by the rc.5 E2E run. |
| `loop/` | 1080 × 1080 (1:1) | 13.2 s | X, autoplay feeds, GitHub social preview | One continuous take of building a formula and hovering it; the cut points are chosen by pixel match so the last frame flows into the first. No music, so the sound loops too. |

Regenerate a film from its project root with `python3 tools/gen.py` (`tools/gen_film.py` for
`linkedin/`), check it with `npx hyperframes@0.9.58 check`, and render with
`npx hyperframes@0.9.58 render -o ../out/<name>.mp4`.

Music: "Chronos" from FreePD (CC0). Sound effects: HyperFrames bundled library.
