# Rich content (`1.0.0-rc.5`)

MD View renders images, static SVG, LaTeX math, and Mermaid diagrams without a webview or
JavaScript runtime. Media never edits the document; the source buffer remains authoritative.

## Image documents

PNG/APNG, JPEG, GIF, WebP, BMP, TIFF, SVG, ICO/CUR, TGA, DDS, Radiance HDR, OpenEXR, PNM
(PBM/PGM/PPM/PAM), QOI, Farbfeld, and JPEG XL files open directly in a fitted image viewer instead
of the text editor, and the same formats render in Markdown. Raster formats decode with pure-Rust
codecs (`image`, and `jxl-oxide` for JPEG XL); TGA, which has no signature, is recognised by its
extension. HDR and EXR are shown clamped to 8 bits, without tone mapping. AVIF and HEIC are not
supported: their decoders need native libraries (dav1d, libheif). Clicking the image opens the
lightbox; animated images play under the same Media settings. Unsupported or corrupt files show a
bounded diagnostic.

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
