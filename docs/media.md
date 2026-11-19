# Rich content (`1.0.0-rc.7`)

MD View renders images, static SVG, LaTeX math, and Mermaid diagrams without a webview or
JavaScript runtime. Media never edits the document; the source buffer remains authoritative.

## Image documents

PNG/APNG, JPEG, GIF, WebP, BMP, TIFF, SVG, ICO/CUR, TGA, Radiance HDR, OpenEXR, PNM
(PBM/PGM/PPM/PAM), QOI, Farbfeld, and JPEG XL files open directly in a fitted image viewer instead
of the text editor, and the same formats render in Markdown. Raster formats decode with pure-Rust
codecs (`image`, and `jxl-oxide` for JPEG XL); TGA, which has no signature, is recognised by its
extension. HDR and EXR are shown clamped to 8 bits, without tone mapping. AVIF and HEIC are not
supported: their decoders need native libraries (dav1d, libheif). Clicking the image opens the
lightbox; animated images play under the same Media settings. Unsupported or corrupt files show a
bounded diagnostic.

## PDF documents

PDF files open in a read-only viewer with continuous pages, fit-width by default, zoom out/in,
100%, and previous/next page with a counter. A worker thread owns the document and renders, with
hayro (pure Rust, CPU only), only the pages near the viewport at the current zoom, quantised to
1/16 pixel per point; the view keeps textures for the visible pages and two on each side, and a
newer request for a page replaces an older one. Pages are capped at 64 megapixels, files at
512 MB. Encrypted PDFs and some advanced features (blending, knockout groups) are not supported
by the renderer yet; pages still show what it can draw.

## Video

Video files open in a viewer with play/pause (click the picture or press Space), a seek bar,
±5 s on the left and right arrows, a time counter, and mute. Everything decodes in-process with
pure-Rust crates; MD View runs no external program and links no C codec.

| Stage | Crate | Scope |
|---|---|---|
| MP4, MOV, M4V, 3GP | `re_mp4` | edit lists, composition offsets, rotation matrix |
| MKV, WebM | `matroska-demuxer` | cue-based seeking, `V_MS/VFW` and `A_MS/ACM` tracks |
| AVI (with OpenDML) | `filmcraft-avi` | `idx1` and OpenDML indexes, dropped frames |
| MPEG-TS, M2TS, MTS, MPG, VOB | `filmcraft-mpegts` | transport and program streams, PTS unwrapping |
| FLV | `mdview-media` | legacy and enhanced (FourCC) tags |
| H.264 video | `rust_h264` | Baseline/Main/High, 8 and 10 bit, 4:2:0 and 4:2:2, frame threads |
| HEVC video | `rust_h265` | Main and Main 10, 4:2:0 and 4:4:4, frame threads |
| AV1 video | `filmcraft-av1` | 8 and 10 bit, all chroma formats, tile threads |
| VP8 video | `filmcraft-vp8` | all profiles |
| VP9 video | `oximedia-codec` | profile 0 and 10-bit 4:2:0 |
| MPEG-1 and MPEG-2 video | `filmcraft-mpeg2v` | 4:2:0 and 4:2:2, field pictures |
| MPEG-4 Part 2 and H.263 video | `oxideav-mpeg4video` | Xvid, DivX 4+, B-VOPs, short header |
| Motion JPEG | `zune-jpeg` | full-range YCbCr |
| AAC sound | `symphonia-codec-aac` | AudioSpecificConfig or ADTS frames |
| MP1, MP2, MP3 sound | `symphonia-bundle-mp3` | frames re-cut from their headers |
| Vorbis, FLAC sound | `symphonia-codec-vorbis`, `symphonia-bundle-flac` | |
| Opus sound | `opus-decoder` | pre-skip removed |
| AC-3 sound | `filmcraft-ac3` | frames re-cut from their headers |
| PCM sound | `mdview-media` | 8 to 64 bit, integer or float, either byte order |

`mdview-media`'s `VideoReader` reads packets one at a time (a long file is never loaded into
memory; AVI, MPEG streams, and FLV are indexed when opened), decodes them, and turns each
picture into straight RGBA at the requested size in one pass: YUV conversion (BT.709 from 720
lines, BT.601 below; limited range, or full range for JPEG), bilinear scaling, and the
container's quarter-turn rotation, with per-axis tables and integer arithmetic across a few
threads. The H.264, HEVC, AV1, VP8, and VP9 decoders are bit-exact with FFmpeg on the sample
clips, MPEG-1/2/4 and H.263 are within IDCT rounding (about 70 dB PSNR), and the RGBA output is
within 3 levels of FFmpeg's accurate chroma interpolation. A 1080p clip decodes and converts at
about 4× real time for H.264, 1.8× for VP9, and over 2× for AV1 on an 8-thread desktop; HEVC
decodes faster than H.264.

Pictures are timed by the packet they came in where the decoder carries it (AV1, VP8,
MPEG-1/2/4, Motion JPEG), and otherwise by giving each picture the smallest pending packet time,
as display order is presentation order. After a seek, leading pictures of an open GOP cannot be
decoded; their times are dropped so the frames that follow keep theirs. AVI has no presentation
times, so its pictures are counted one frame interval apart from the key frame. A test decodes
every sample from the start and again after a seek and checks that the frames match exactly.

The viewer's worker thread seeks to the nearest earlier key frame, decodes up to the requested
time without converting, and hands each later frame over at its presentation time; a pause or
seek interrupts the wait, and frames more than two intervals late are dropped. `AudioReader`
decodes the soundtrack on its own thread, maps it to the device's channels, resamples it
linearly to the device's rate, and feeds cpal (CoreAudio, WASAPI, or ALSA) through a one-second
queue whose backpressure paces the decoder. Pausing stops both, seeking while paused decodes a
single frame (the last one for a seek past the end), and closing the tab stops everything.

Windows Media (WMV/ASF) and Ogg video (OGV/Theora) open in the viewer with a message that
they are not supported: there is no pure-Rust Windows Media Video decoder, and Ogg is not wired
up yet. Inside the supported containers, Microsoft MPEG-4 (DivX 3), Sorenson Spark (FLV1), VP6,
WMA, DTS, and E-AC-3 are reported the same way (a video without a playable soundtrack plays
without sound).

## Images

- Relative and absolute local paths and `file:` URLs are validated with `craft-docpath`.
  Network paths, device paths, and unsafe URLs are refused with a message.
- Remote `http(s)` images are fetched only when a document references them. HTTPS uses rustls;
  the request has a 12-second total timeout, a 32 MB source limit, and one retry after a transient
  failure or HTTP 5xx.
- Remote bytes are cached on disk under the platform cache directory (`mdview/media`) with a
  SHA-256-style BLAKE3 key and an ETag. A later request sends `If-None-Match`; `304` reuses the
  cached bytes.
- Private-network hosts (`localhost`, loopback, RFC 1918, link-local, `.local`, unique-local
  IPv6) are blocked by default. Both remote loading and private-network loading can be toggled
  under **View → Media**.
- Missing, oversized, and undecodable images show an inline placeholder with the alt text and the
  reason; the renderer never panics.
- Clicking an image opens the lightbox with 50%, 100%, and 200% zoom.

## Static SVG

- Parsed through `craft-svg` with bounded size, depth, entity, node, and embedded-document limits
  and rasterized with `resvg` 0.48.1 at the display pixel size.
- `<script>`, `javascript:`, `foreignObject`, document entities, and external `href` resources are
  rejected before parsing.
- Layout text uses the system font database; transparent pixels stay transparent and
  premultiplied output is converted back to straight RGBA for egui.
- Theme changes do not recolor content images or SVG; Mermaid and math are re-rendered by theme.

## LaTeX

- `$…$`, `$$…$$`, and ```` ```math ```` blocks use RaTeX with embedded KaTeX fonts.
- Parse, layout, and SVG errors show the TeX source with a diagnostic; no fake formula is drawn.
- The dark theme sets the formula color to white; the light theme uses black.
- A 100-formula corpus test covers fractions, roots, integrals, sums, matrices, cases, text, and
  delimiters.
- The formula editor maps a source position to the formula part it draws. RaTeX locates leaf
  nodes and braced groups; spans of composites (`\frac`, `\sqrt`, scripts, `\left…\right`,
  environments) are derived from their children and the command around them. A part is
  highlighted with `\textcolor` only in a wrap form whose layout is identical to the original
  (glyph positions and sizes compared on the display list with colors removed); otherwise the
  enclosing part is used.
- Formula editor thumbnails and its preview are rasterized at their exact display size
  (`render_math_at`) instead of being minified from the reading-view raster, which blurs thin
  strokes.

## Mermaid

- Fenced ```` ```mermaid ```` blocks render with `mermaid-rs-renderer` 0.3.1 and `resvg`.
- Supported: flowchart/graph, sequence, class, state, ER, pie, gantt, journey, timeline,
  mindmap, gitGraph, quadrant, xychart, requirement, sankey, packet, radar, C4, block,
  architecture, kanban, treemap, and zenUML as provided by the renderer.
- The release-blocking Tier A set is exercised by a 20-diagram test.
- Unsupported or invalid diagrams show the code fence source plus a diagnostic.
- Diagrams fit the available width; clicking one opens the zoomable lightbox.

## Cache and limits

| Limit | Value |
|---|---|
| Source bytes | 32 MB |
| Decoded image side | 16,384 px |
| SVG element depth | 128 |
| SVG embedded documents | 8 MB |
| Texture cache | 64 MB LRU |
| Remote timeout | 12 s |

The texture cache is global and byte-budgeted. Closing a tab clears the cache and increments the
media generation, so in-flight results from the closed tab are dropped instead of uploading late
textures. Loading runs on one dedicated worker thread with a 16 MB stack for the recursive SVG
parser; the UI never blocks on disk, network, or rasterization.

## Animation

Animated SVG uses the bounded SMIL/CSS runtime documented in [`animation.md`](animation.md).
Animated GIF, WebP, and APNG decode once into frames with their own delays (delays under 20 ms
are raised to 20 ms); playback picks the frame for the current time and uploads a texture only
when the frame changes. Decoding stops at 1,000 frames or 256 MiB of pixels, keeping the frames
decoded so far, and an animation that cannot be decoded shows its first frame. Frames advance at
up to 30 fps only while visible; play/pause and reduced motion live under **View → Media**.

## Visual references

`crates/mdview-render/tests/snapshots/media-light.png` and `media-dark.png` cover a local PNG, an
SVG, inline and display math, a Mermaid flowchart, and a missing-image error placeholder.
