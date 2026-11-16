# Markdown and rich content

MD View parses CommonMark with GFM extensions and paints the result itself. The reading layout is
calibrated against GitHub's Markdown body: 1em block spacing, 1.5 line height, heading ratios, a
rule under h1/h2, alert colors and Octicons, borderless code blocks with a hover Copy button,
content-width tables with cell borders and zebra rows, and thick horizontal rules.

## Supported syntax

- Headings, paragraphs, emphasis, strong text, links, images, and block quotes.
- Ordered, unordered, and task lists, including loose-list paragraph spacing and nested levels.
- GFM tables with aligned columns, cell links, inline code chips, and centered headers.
- Footnotes with GitHub's numbered references and `↩` back links.
- GFM alerts: Note, Tip, Important, Warning, and Caution, with GitHub's colors and icons.
- Fenced code blocks with GitHub Primer syntax colors and a copy action.
- Inline and display LaTeX, rendered as native vector media at the calibrated MathJax scale.
- Mermaid flowcharts and sequence diagrams rendered natively; no browser is involved.
- Raw HTML passed through GitHub's sanitizer subset: comments and scripts disappear, `align`,
  `<img>`, `<a>`, `<b>`, `<i>`, `<code>`, `<br>`, `<hr>`, `<ul>`, `<details>`/`<summary>`, tables,
  `<kbd>`, `<sub>`, and `<sup>` render.
- Local images in PNG, JPEG, GIF, WebP, BMP, TIFF, and SVG, plus opt-in cached remote images.

## Media

Media is resolved by a background worker and uploaded as an egui texture. Every media request is
warmed before the first preview layout, and intrinsic boxes for images, SVG, math, and Mermaid are
reserved so loading a texture cannot reflow the document or flicker while the reader is at the end.

![Rich media render](/images/media-light.png)

The image viewer supports zoom, pan, fit, actual size, and a lightbox. SVG animation is supported
for the SMIL/CSS subset documented in the repository; animated SVG pauses offscreen and honors the
reduced-motion setting.

## GitHub parity

The repository compares its render with GitHub's own output. The parity fixtures cover the
repository README and docs plus a full-element document, checking block spacing band by band and
line breaks for every common Markdown element. See `docs/github-parity.md` in the repository for
the measured evidence.

![GitHub-style preview](/images/preview-light.png)

## Remote content

Remote images are enabled by default and cached on disk. Private-network requests are disabled
unless **Settings → Media → Private networks** is enabled. The renderer never executes scripts or
loads remote HTML.

## Search highlights

Document search highlights literal matches in the reading view. Workspace search runs on a
cancellable worker over the indexed file list, keeps results from stale query generations, and
activates the matching source line.
