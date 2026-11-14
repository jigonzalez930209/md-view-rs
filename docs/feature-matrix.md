# Pre-1.0 feature matrix

This page is the acceptance map for the feature set that must stay working before `1.0.0`.
The fixture workspace at `examples/v1-workspace` contains five folders nested one inside the
next, with Markdown and source/data files alternating at every level. Open it from the
repository root:

```sh
cargo run -p mdview-app -- examples/v1-workspace \
  examples/v1-workspace/level1/level2/level3/level4/level5/showcase.md
```

## Fixture layout

```text
examples/v1-workspace/
├── README.md
├── config.json
└── level1/
    ├── notes.md
    ├── build.sh
    └── level2/
        ├── guide.md
        ├── data.csv
        └── level3/
            ├── reference.md
            ├── sample.py
            └── level4/
                ├── component.tsx
                ├── notes.txt
                └── level5/
                    ├── showcase.md
                    ├── deep.rs
                    ├── image.png
                    ├── mark.svg
                    └── diagram.mmd
```

`showcase.md` is the render matrix. It includes fenced Rust, JSON, and Bash blocks,
a weighted full-width table, task and ordered lists, inline and display LaTeX, a local PNG,
a local SVG, two Mermaid diagrams (flowchart and sequence), an alert, a quote, a footnote,
and a horizontal rule.

## Feature acceptance

| Feature | Implementation | Automated evidence |
|---|---|---|
| Five-level lazy workspace tree | `mdview-workspace` loads children on expansion | `v1_feature_workspace_has_five_nested_levels`; E2E `deep/nested/three/four/five` |
| Expansion survives watcher refreshes | `refresh_dir` records expanded descendants and re-applies them after rescanning | `refreshing_a_parent_keeps_nested_expansion`; E2E waits 1.4 s after `touch` and opens `final.md` |
| Markdown/GFM | `mdview-markdown` with tables, tasks, footnotes, strikethrough, alerts, heading attributes, math | parser tests and `preview-light`/`preview-dark` snapshots |
| Syntax-highlighted code | `mdview-syntax` (syntect/two-face), language header, copy, 19 px line height, horizontal scrolling | `media-*` and `preview-*` snapshots; E2E code block stage |
| LaTeX | `mdview-media` RaTeX layout; inline formulas sized to the text, display formulas shown at full size | `common_formulas_render`, `one_hundred_formulas_render`; `media-light`/`media-dark` snapshots |
| Raster images | `mdview-media` local and cached remote images with lightbox | `media-*` snapshots; E2E local and HTTP PNG assets |
| SVG | `craft-svg`/`resvg` rasterization, static and supported SMIL/CSS animation | `media-*` snapshots; E2E local and HTTP animated SVG assets |
| Mermaid | native `mermaid-rs-renderer`, no browser | `mermaid.rs` diagram corpus; flowchart and sequence in `media-*` and E2E |
| Tables | weighted columns fill the reading width, header surface, zebra rows, separators, and clickable links inside cells | `preview-*` snapshots; E2E opens `docs/guide.md` from a table link |
| Editor and preview sync | Rope editor, split view, block-anchor bidirectional scroll | workspace tests, split snapshots, E2E scroll stress |
| 120 Hz scrolling | block-virtualized preview, release frame-budget gate | `cargo xtask perf`; release E2E `over_8.33ms=0` |
| Diagnostics | two rotating 512 KiB logs, 1 MiB hard cap | E2E log-size assertion and `docs/logging.md` |

## Commands

```sh
cargo test -p mdview-workspace --lib
cargo test -p mdview-media
cargo test -p mdview-render
cargo test --workspace
MDVIEW_E2E_PROFILE=release cargo xtask e2e
cargo xtask perf
cargo xtask release-check
```

The release E2E screenshots land in `target/e2e/` (gitignored). The nested-folder proof is
`08a-nested-folders.png`; it is taken after the five folders are expanded and a root file is
touched, so the watcher has refreshed the tree.

## Deferred after 1.0

PDF viewing, printing, plugins, AI features, hosted CI, and signed Windows/macOS artifacts
remain outside the pre-1.0 feature gate.
