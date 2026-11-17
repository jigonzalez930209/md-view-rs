# Markdown support (`1.0.0-rc.3`)

MD View parses Markdown with `pulldown-cmark` (CommonMark 0.31 + GFM) into a spanned AST. The
document buffer in the editor is always the source of truth; the reading view is derived and
never edits text.

## Supported syntax

| Feature | Behavior |
|---|---|
| Headings | Levels 1–6, GitHub-style anchors, explicit `{#id}`, duplicate suffixes |
| Emphasis / strong / strikethrough | Inline styling in the reading view |
| Links | External (`http`, `https`, `mailto`) in the system browser; anchor and workspace links in the app |
| Images | Local and cached remote PNG/JPEG/GIF/WebP, static and animated SVG render inline with a zoomable lightbox; failures show an inline diagnostic |
| Lists | Ordered, unordered, nested, and GFM task lists |
| Block quotes | Ordinary quotes and GFM alerts (`NOTE`, `TIP`, `IMPORTANT`, `WARNING`, `CAUTION`) |
| Tables | GFM tables with left, center, and right alignment, weighted columns that fill the reading width, header surface, zebra rows, and separators |
| Code | Fenced and indented blocks, language label, syntax highlighting, copy button, 19 px line height, horizontal scrolling for long lines, and visible inline-code chips |
| Thematic breaks | Rendered as separators |
| Footnotes | Definition blocks and inline references with backlinks |
| Front matter | YAML/TOML-style front matter is parsed and hidden from the view |
| Math | `$…$`, `$$…$$`, and ```` ```math ```` are laid out with RaTeX; inline formulas stay near the text size and display formulas render at full size |
| Inline HTML | Safe subset: `br`, `sub`, `sup`, `kbd`, `mark`; `script`, `style`, `iframe`, `object`, and `embed` contents are suppressed |

## Links

- `[x](#section)` scrolls the current reading view to the matching heading anchor.
- `[x](other.md#section)` resolves the path relative to the current document with
  `craft-docpath`, opens or activates a tab, switches it to the reading view, and scrolls to the
  heading.
- `[x](../assets/file.txt)` opens a local workspace file when the path is local. Network, device,
  and unsafe URL destinations are refused with an error; the reading view stays intact.
- `javascript:`, `vbscript:`, and `data:text/html` destinations are rejected.
- External links use the platform browser and never load inside the app.

## Reading layout and typography

Settings → **Reading** has a margin slider from **5%** to **40%** of the pane on each side,
replacing the old three presets. The same tab sets independent point sizes for body text,
headings, code, tables, and equations. Inline code and sub/superscripts follow the body size;
table cells use the table size; fenced code uses the code size; inline and display math use the
equation size.

## Modes and scroll

Markdown tabs start in the reading view; other files start in the editor. Use the View menu or
`Cmd+E` / `Ctrl+E` to toggle Edit and Reading modes, and `Cmd+Shift+E` for Split. Editor and
preview scroll offsets are stored per file and per mode in the session.

The preview records every top-level block with its source-line range, painted top, and painted
height. Anchors keep their block and intra-block fraction when a re-render changes the height.
This is the foundation for split synchronized scrolling.

## Visual references

The local snapshots under `crates/mdview-render/tests/snapshots/` cover the reading view in light
and dark modes. Regenerate deliberately with:

```sh
UPDATE_SNAPSHOTS=1 cargo test -p mdview-render
```

## Known limits

- Split mode reparses the document when its revision changes and repaints the visible frame;
  per-block incremental layout is not yet cached.
- Media security and caching details are in [`media.md`](media.md).
- Cross-platform manual checks are deferred to the `1.0.0` release gate.

The runnable acceptance document for every feature above is
`examples/v1-workspace/level1/level2/level3/level4/level5/showcase.md`; see
[`feature-matrix.md`](feature-matrix.md).
