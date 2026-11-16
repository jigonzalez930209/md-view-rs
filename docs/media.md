# Rich content (`1.0.0-rc.1`)

MD View renders images, static SVG, LaTeX math, and Mermaid diagrams without a webview or
JavaScript runtime. Media never edits the document; the source buffer remains authoritative.

## Image documents

PNG, JPEG, GIF, WebP, BMP, TIFF, and SVG files can be opened directly and render in a fitted
image viewer instead of the text editor. Clicking the image opens the lightbox; animated SVG
plays under the same Media settings. Unsupported or corrupt files show a bounded diagnostic.

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
Frames advance at 30 fps only while visible; play/pause and reduced motion live under
**View → Media**.

## Visual references

`crates/mdview-render/tests/snapshots/media-light.png` and `media-dark.png` cover a local PNG, an
SVG, inline and display math, a Mermaid flowchart, and a missing-image error placeholder.
