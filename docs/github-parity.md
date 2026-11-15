# GitHub rendering parity

The preview layout is checked against GitHub's own output, not against memory:

1. Render the `render_snapshots.rs` document with the GitHub Markdown API
   (`gh api markdown -f mode=gfm -f text=...`).
2. Wrap the HTML in `github-markdown-css` 5.8.1 (`github-markdown-light.css`) with a
   520 px `.markdown-body` at a 14 px font size, which matches the preview column of
   `preview-light.png`.
3. Screenshot it with headless Chrome at 760 px and compare it side by side with
   `crates/mdview-render/tests/snapshots/preview-light.png`.

Values taken from that comparison and GitHub's CSS:

| Element | GitHub | Preview |
|---|---|---|
| Body | Noto Sans, line height 1.5 | bundled Noto Sans, 1.5 |
| Block spacing | 16 px at 16 px (1em) | 1em |
| Headings | h1 2em … h6 0.85em, weight 600, rule under h1/h2 | same ratios, Noto Sans SemiBold |
| Lists | 2em indent, 0.25em between items, disc/circle/square | same; markers on the measured first-row center |
| Task lists | checkbox in the marker column | same |
| Inline code | 85%, 0.2em × 0.4em padding, 6 px radius | same; a chip that does not fit moves to the next row |
| Code blocks | 16 px padding, 85%, line height 1.45, 6 px radius | same |
| Tables | content width, 6 × 13 px cells, 1 px borders, zebra rows, semibold header | same |
| Quotes | 0.25em left bar, muted text | same |
| Alerts | colored bar, Octicon + title (weight 500) | same Octicons; regular weight title |
| Footnotes | numbered `[n]`, top border, small muted list | same |

Known gaps: math is not part of the API comparison (GitHub renders it client-side with
MathJax); the preview uses RaTeX with font-size scaling and baseline alignment.
Footnotes do not draw GitHub's trailing `↩` link; the note number is the back link.
