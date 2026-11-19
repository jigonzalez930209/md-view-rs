# Media fixtures

Sample files for the viewers added after `1.0.0-rc.5`. Open this workspace and click through
each folder; every file must open in its viewer, not in the text editor. The files are built
by `scripts/fixtures/make_media.py` (except the hand-written SVGs), and
`crates/mdview-media/tests/workspace_fixtures.rs` and
`crates/mdview-table/tests/workspace_fixtures.rs` open them through the real loaders.

## `images/`: one still image per format

Each card prints its format name, so a wrong decoder shows at a glance.

- PNG, JPEG (`.jpg`, `.jfif`), WebP, GIF, BMP (`.bmp`, `.dib`), TIFF, ICO.
- TGA, Radiance HDR, OpenEXR, PBM, PGM, PPM, PAM, QOI, Farbfeld (`.ff`), JPEG XL.
- Check: the image is fitted to the view, and zoom and the status bar size (240 × 150) work.

## `animated/`: rasters and SVGs that play

| File | What to look for |
|---|---|
| `bounce.gif`, `bounce.webp`, `bounce.apng` | The white disc swings left and right at 20 fps, looping |
| `line-drawing.svg` | The blue curve draws itself with easing, stays drawn, then the green dot fades in |
| `orbit.svg` | The yellow arrow follows the track and turns with it; the blue dot eases back and forth; the orange disc pulses |
| `css-loader.svg` | Three purple dots rise in a staggered wave; the red tick spins in 8 steps; the blue ring draws and undraws |
| `colors.svg` | Background fades through named, `#rgb`, and `rgb()` colors; the square steps through colors; the bar grows; the circle turns green after 1 s |
| `partial.svg` | The dot moves and an "Animation not fully supported" badge shows (`keyPoints`) |
| `static.svg` | A still image with no playback controls |

## `documents/guide.pdf`

Six pages: A4 with text, shapes, and an embedded JPEG; a 40-row table; three US Letter text
pages; and a landscape page. Check ◀ ▶, the page counter, −/+, Fit width, and 100%.

## `video/`

| File | Contents |
|---|---|
| `clip.mp4` | H.264 + AAC, 4 s, test pattern with a beeping tone |
| `clip.webm` | VP9 + Opus, the same clip |
| `silent.mkv` | H.264 color bars, no audio track (no Mute button) |
| `portrait.mov` | Landscape frames with a 90° rotation flag; must show upright, taller than wide |

Check play/pause (button, click, Space), the clock, seeking with the slider and ←/→, and Mute.
Every clip decodes in-process with the pure-Rust decoders; nothing needs to be installed.

`video/formats/` has one 2 s test pattern with a beeping tone per further container and codec;
each must play with sound:

| File | Video + sound |
|---|---|
| `hevc.mp4` | HEVC + AAC |
| `av1.mkv` | AV1 + FLAC |
| `vp8.webm` | VP8 + Vorbis |
| `xvid.avi` | MPEG-4 Part 2 (Xvid, with B-frames) + MP3 |
| `mjpeg.avi` | Motion JPEG + 16-bit PCM |
| `h264.ts` | H.264 + AAC (ADTS) in an MPEG transport stream |
| `mpeg2.ts` | MPEG-2 (with B-frames) + AC-3 |
| `mpeg1.mpg` | MPEG-1 + MP2 in an MPEG program stream |
| `h264.flv` | H.264 + AAC in Flash Video |
| `h263.3gp` | H.263 (QCIF) + AAC |

## `tables/`: the editable grid

| File | Dialect | What it exercises |
|---|---|---|
| `people.csv` | comma | Quoted commas and quotes, a two-line cell, CJK text, edge spaces, an empty cell |
| `metrics.tsv` | tab | Tab-separated values |
| `inventory.psv` | pipe | Pipe-separated values |
| `european.csv` | semicolon | Sniffed `;` delimiter, decimal commas, a quoted `;` |
| `large.csv` | comma, CRLF | 5,000 rows for scrolling and virtualisation; edits keep CRLF |

Edit a cell, insert and delete rows and columns, paste a block from a spreadsheet, undo, and
save; then reopen in Split mode and confirm only the edited fields changed in the text.
