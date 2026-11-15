# GitHub rendering parity

The preview layout is checked against GitHub's own output, not against memory:

1. Render the `render_snapshots.rs` document with the GitHub Markdown API
   (`gh api markdown -f mode=gfm -f text=...`).
2. Wrap the HTML in `github-markdown-css` 5.8.1 (`github-markdown-light.css`) with a
   520 px `.markdown-body` at a 14 px font size, which matches the preview column of
   `preview-light.png`.
3. Screenshot it with headless Chrome at 760 px and compare it side by side with
   `crates/mdview-render/tests/snapshots/preview-light.png`.

4. For math, render `media_snapshots.rs`'s formulas the same way with MathJax 3.2.2
   (`es5/tex-svg.js`, inlined into the page) and compare with `media-light.png`.
5. GitHub sizes its margins in pixels for a 16 px body, so the full layout is also
   compared at GitHub's real scale: render the same page without the 14 px override and
   compare it with `preview-github-scale.png` (body 16, h1 32, code 14, tables 16).
6. Render `crates/mdview-render/tests/fixtures/github-parity.md` (every common element:
   h1-h6, inline styles, `<kbd>`, `<sub>`, `<sup>`, quotes and nested quotes, the five
   alerts, three-level ordered lists, loose lists, task lists, inline and display math,
   aligned tables with code, fenced code, rules, footnotes, and README-style HTML: a
   centered paragraph with a sized image, a comment, a centered heading, `<details>`
   with Markdown inside, a right-aligned `div`, and an inline image) the same way at
   16 px, with
   MathJax inlined and GitHub's `.sr-only` rule (which hides the "Footnotes" heading), and
   compare it with `github-parity.png`.
7. Run `python3 -I scripts/github-parity/measure.py <github-ref.png> <math-ref.png>
   <github-ref16.png> <parity-ref.png>`.

Last measurement (14 px body, default settings):

| Check | GitHub | Preview |
|---|---|---|
| List marker center minus first-line x-height center, 7 rows (px) | -1.5 ×5, -1.0 ×2 | -1.5 ×5, -1.0 ×2 |
| Task box size (px) | 13 | 13 |
| Display formula ink size (px) | 98 × 42 | 96 × 44 |
| Inline `E = mc^2` ink height (px) | 14 | 14 |
| Body text ink width, "An inline formula" (px) | 110 | 108 |

At GitHub's 16 px scale, measured between heading rules (each section covers every block
in it: paragraphs, alert, lists, task lists, code block, table):

| Section height (px) | GitHub | Preview |
|---|---|---|
| Title, paragraph, and alert | 215 | 216 |
| Lists, task lists, ordered list | 359 | 360 |
| Code block | 189 | 190 |
| Table | 190 | 190 |
| Alert box | 72 | 72 |

Full-element document (`github-parity.png`), ink bands matched in order (43 in both):

| Bands | Elements | Top offset vs GitHub |
|---|---|---|
| 0-28 | h1-h6, paragraph with kbd/sub/sup, wrapped paragraph, quote, nested quote, 5 alerts (72 px each), nested ordered list, loose list, tasks, inline-math line | 0 to +2 px |
| 29 | display `\sum` formula | +3 px; same 141 px width, 56 px tall vs 50 |
| 30-34 | table, code block, rule, footnote reference line | +9 to +12 px, a constant shift; heights match within 2 px |
| 35-40 | centered image + caption, centered heading, collapsed `<details>`, right-aligned `div`, inline image on the baseline | +10 px, the same constant shift; heights match exactly |
| 41-42 | footnotes (moved to the end, as on GitHub) | +11 px |

The shift after the display formula comes from the TeX engine: RaTeX sets the limits of
`\sum` 6 px taller than MathJax at the same width. Everything after it keeps GitHub's
sizes and spacing.

The math section is excluded from the section-height table: the API page has no MathJax and the snapshot has no
media provider, so both show raw TeX. Math is measured against MathJax separately above.

Values taken from that comparison and GitHub's CSS:

| Element | GitHub | Preview |
|---|---|---|
| Body | Noto Sans, line height 1.5 | bundled Noto Sans, 1.5 |
| Block spacing | 16 px at 16 px (1em) | 1em |
| Headings | h1 2em … h6 0.85em, weight 600, rule under h1/h2 | same ratios, Noto Sans SemiBold |
| Lists | 2em indent, 0.25em between items, disc/circle/square | same; markers on the measured first-row center |
| Task lists | disabled checkbox in the marker column (gray) | same |
| Inline code | 85%, 0.2em × 0.4em padding, 6 px radius | same; a chip that does not fit moves to the next row |
| Code blocks | 16 px padding, 85%, line height 1.45, 6 px radius | same |
| Tables | content width, 6 × 13 px cells, 1 px borders, zebra rows, semibold header | same |
| Quotes | 0.25em left bar, muted text | same |
| Alerts | colored bar, Octicon + title (weight 500) | same Octicons; regular weight title |
| Footnotes | numbered `[n]`, top border, small muted list, `↩` back link | same |

Math is rendered with RaTeX instead of MathJax; both use the Computer Modern / KaTeX
metrics, and the size is calibrated to MathJax's output above. The `↩` back link is drawn
as a vector arrow because the bundled fonts have no arrow glyphs.

## Code colors

Fenced code uses GitHub's Primer "prettylights" palette (`crates/mdview-syntax/src/github_theme.rs`),
light and dark. Sampled from the parity render: `def`/`return` #cf222e, function names
#6639ba, strings #0a3069, interpolation braces #953800 — the same colors as GitHub.

## Real documents

Hand-written fixtures can miss cases, so the repository's own documents are compared
too. `render_external` in `crates/mdview-render/tests/github_parity.rs` renders any file
at GitHub's scale (`MDVIEW_PARITY_DOC=README.md MDVIEW_PARITY_OUT=out.png cargo test -p
mdview-render --test github_parity render_external -- --ignored`); the reference is the
GitHub Markdown API render of the same file in a 522 px `github-markdown-css` column.

| Document | Ink bands (GitHub / preview) | Top offset range | Largest height difference |
|---|---|---|---|
| `README.md` | 70 / 70 | -1 to +11 px | 2 px |
| `docs/markdown.md` | 52 / 52 | -11 to +2 px | 11 px (a 1058 px table) |
| `docs/media.md` | 79 / 79 | 0 to +10 px | 6 px |
| `docs/themes.md` | 73 / 73 | 0 to +13 px | 4 px |

Same band counts mean every line breaks where GitHub breaks it. The comparison found and
fixed: tight list items that start with code, emphasis, or a link were split or lost
their style; linked code lost its chip; egui counted a line's trailing space when
wrapping; code did not break after hyphens or keep glued punctuation; tables did not
follow the browser's automatic column layout, header centering, or vertical centering;
and headings dropped inline code. The remaining offsets are sub-pixel accumulations
(egui rounds each text row to whole pixels) of under 0.2 px per line.
