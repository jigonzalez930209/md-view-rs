# Pre-1.0 review and UX backlog

Status: **resolved in `0.14.0`** — review of `0.13.0` (commit `d99b52f`), implemented and
verified on Linux on 2026-11-15. Each item keeps its ID so commits and tests can reference it.
The resolution evidence is listed at the end of each section and in the release verification
summary below.

Severity scale:

- **P0** — blocks `1.0.0-beta.1`.
- **P1** — must be fixed before `1.0.0`.
- **P2** — should be fixed before `1.0.0`; may ship as a documented known issue.
- **P3** — polish.

## 1. Verification run

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo xtask check-file-length` | pass (7 files are above 450 lines) |
| `cargo test --workspace --locked` | pass with a disk-backed `TMPDIR`; see REL-04 and REL-05 |
| `cargo xtask e2e` (release) | **E2E PASS**, 23,760 frames, final windows 0 frames over 8.33 ms, 231 KB log |
| `cargo xtask perf` | combined p50 1.559 ms, p95 2.337 ms, max 2.474 ms |
| `cargo xtask release-check` | pass, 4 assets, version 0.14.0 |
| `cargo deny check` | advisories, bans, licenses, and sources pass (REL-02) |

The E2E suite passes, but it only checks that the interactions finish and that the logs are
clean. It does **not** check scroll end positions, layout, or icon rendering. Every visual
finding below came from reviewing the screenshots in `target/e2e/` and the committed
snapshots, not from a failing test. Section 7 lists the coverage this review needs.

## 2. Release and process

| ID | Sev | Finding | Action |
|---|---|---|---|
| REL-01 | P0 | No `LICENSE-MIT` / `LICENSE-APACHE` files, although `Cargo.toml` declares `MIT OR Apache-2.0`. | Add both license files and reference them from the packaging metadata. |
| REL-02 | P0 | No `deny.toml`. Advisories report `bincode` (RUSTSEC-2025-0141), `yaml-rust` (RUSTSEC-2024-0320), and `ttf-parser` (RUSTSEC-2026-0192) as unmaintained. License policy is not enforced. | Add `deny.toml` with license allow-list, sources, and documented advisory exceptions; run it in `release-check`. |
| REL-03 | P0 | `craft-*` crates are git dependencies on `storytold/craft-libs`. H11 requires rebuilding from the tag. | Confirm the repository and pinned revision are public and permanent, or vendor/mirror them. |
| REL-04 | P1 | The `mdview-render` doctest (`src/lib.rs`, line 7) fails to link with `ld ... Bus error` when `/tmp` (tmpfs) is low on space. It links the whole media/font stack for one example. | Mark the example `no_run`, or set a disk-backed temp dir for builds. Document the requirement in `docs/testing.md`. |
| REL-05 | P2 | One unit test failed once (58 passed / 1 failed in a binary of 59), then passed on rerun. The test was not identified. | Run `cargo test --workspace` in a loop, identify the flaky test, and fix it. |
| REL-06 | P1 | Hosted CI is deferred until 1.0, but H10 requires full automated regression on three OSes. Windows and macOS have never been compiled. | Add minimal CI now: fmt, clippy, test, and file-length on Linux, plus `cargo build` on macOS/Windows. |
| REL-07 | P2 | No single local gate. | Add `cargo xtask ci` that runs the five README commands plus `cargo deny check`. |
| REL-08 | P2 | `mdview-app` has 7,559 lines (a third of the code) around one `MdViewApp`. Files are split by line count rather than responsibility. | Before feature freeze, group `commands`, `document_ops`, and `media_runtime` into submodules with their own state. |
| REL-09 | P3 | Three `#[allow(clippy::too_many_arguments)]` in `crates/mdview-editor/src/render.rs`. | Introduce a paint context struct. |
| REL-10 | P1 | The ROADMAP says "plugins stay out" of 1.0, but 0.13 ships a viewer plugin API, and the versioning section treats plugin API breaks as a major version. | Declare the plugin API internal (not public) for 1.0, or freeze it explicitly. |
| REL-11 | P3 | AGENT.md lists a root `tests/` directory that does not exist; README typo "pipelinee"; the README status paragraph duplicates the ROADMAP; "0.11.x fixes" rows belong in the CHANGELOG. | Clean up the docs. |

Resolution: `LICENSE-MIT`, `LICENSE-APACHE`, `deny.toml`, `.github/workflows/ci.yml`, and
`cargo xtask ci` are present; `release-check` runs `cargo deny`; the doctest is a `text` block
and `docs/testing.md` documents `TMPDIR`; the media tests use a 20 s deadline helper; the
`commands`, `document_ops`, and `media_runtime` modules are directories; the editor paint
context is `RowPaint`; the plugin API is documented as internal; docs and roadmap were cleaned.

## 3. Scrolling (highest-priority UX area)

### SCR-01 (P0 — resolved) — Editor flickers when scrolling reaches the end

Reported: scrolling the editor to the bottom makes it flicker.

Likely cause (`crates/mdview-editor/src/render.rs`):

- `content_height = (total_rows * row_height + 10.0).max(available_height)`.
- The scroll area is `ScrollArea::both()`. When the horizontal scrollbar appears or disappears,
  `available_height` changes, and so does `content_height` and the maximum offset. The view
  can oscillate between two clamped offsets across frames.
- The find bar is drawn inside the same `paint` call (`show_find_bar` before measuring), so
  opening or closing it also changes the available height.
- The split-sync code writes `vertical_scroll_offset` from the preview's anchor map. At the end
  of the document, the preview's clamped maximum can map back to a different editor line,
  which produces echo jitter despite echo suppression.

Acceptance: 300 wheel events at the end of a 10,000-line file with and without wrap, in Edit
and Split modes, produce no change in the editor offset after the first clamp. Assert the
offset frame by frame in E2E.

### SCR-02 (P0 — resolved) — Preview does not reach the end with images, diagrams, or tables

Reported: with several images, Mermaid diagrams, or tables, scrolling the preview to the end
stops before the real end.

Likely cause:

- The preview is block-virtualized. Offscreen blocks use `estimate_height`
  (`crates/mdview-render/src/layout.rs`). A paragraph containing only an image or display math
  is estimated as one text line (~32 px), while the real block is 150–400 px. Mermaid and
  media also render as spinners first (`18-split.png`), so the measured height grows after
  the async job finishes.
- The paragraph estimate assumes 72 characters per line regardless of width. Table rows are
  fixed at 30 px, even when narrow columns wrap (`18-split.png` shows "animate d").
- The total content height is therefore too small until each block has been painted once,
  so the scroll area's maximum offset is reached before the last block.

Required:

- Store measured heights per block and invalidate them only when width, typography, or media
  state changes.
- Estimate media blocks from the decoded intrinsic size (it is known as soon as the header
  is decoded), using the layout width.
- When a block above the viewport changes height, compensate the scroll offset so the visible
  content does not jump.
- Treat "scrolled to bottom" as sticky: if the user is at the end and heights grow, keep them
  at the end.

Acceptance: the last block of `examples/v1-workspace` documents with 10+ images, 5 Mermaid
diagrams, and 5 tables is fully visible after one `End` key press and after continuous
wheel input. Assert in E2E using the source map's last block.

### SCR-03 (P1 — resolved) — Scroll past end, like VS Code

Feature request: the editor must allow scrolling until the last line reaches the top of the
editor viewport (VS Code `editor.scrollBeyondLastLine`).

- Add `viewport_height - row_height` of virtual space after the last row.
- Add an Editor setting "Scroll beyond last line", on by default.
- Split sync must map the overscroll region to the preview's end, not beyond it.
- Do this together with SCR-01, because it changes the same height computation.

### SCR-04 (P2 — resolved) — Split mode re-renders math and media

Switching to Split shows spinners for math, images, and Mermaid that were already rendered in
Reading mode (`18-split.png`). The cache appears to be keyed by width or by pane. This also
contributes to SCR-02. Reuse the rasterized output, and rescale when only the width changes.

### SCR-05 (P2 — resolved) — Editor horizontal scrollbar covers the last visible line

In `18-split.png` the horizontal scrollbar is drawn over line 40. Reserve its height, or use an
overlay scrollbar that does not hide text.

Resolution: stable content height plus the floating find overlay removed the editor end
flicker; a sticky bottom, width-aware estimates, and media-aware floors make the preview reach
the last block; `scroll_beyond_last_line` is a persisted setting; media requests use stable
raster widths; the editor reserves scrollbar clearance. Regression evidence:
`crates/mdview-editor/tests/scroll_end.rs`, `crates/mdview-render/tests/scroll_end.rs`, and the
E2E `split end stability` stage.

## 4. Icons and visual system

Review sources: `target/e2e/*.png`, `crates/*/tests/snapshots/*.png`.

| ID | Sev | Finding | Evidence |
|---|---|---|---|
| ICO-01 | P1 | **Inconsistent icon sizes.** The activity rail uses ~20 px glyphs, the title bar ~16 px, explorer header actions ~12 px, and tab-bar controls ~16 px. Define three sizes as tokens (for example 14/16/20) with a single stroke weight, and apply them by context. | `01-initial.png` |
| ICO-02 | P1 | **Duplicate controls.** The sidebar toggle and the split toggle appear in both the title bar and the tab bar. One copy of each must go (see UX-01). | `01-initial.png`, header snapshot |
| ICO-03 | P2 | `README.md` uses the Material "info" icon (a blue circle) while every other `.md` uses the Markdown icon. This is correct per Material Icon Theme, but it reads like a status badge in a sparse tree. Decide on purpose and document it. | `sidebar.png`, e2e explorer |
| ICO-04 | P2 | Material folder icons at 16 px lose their inner glyph (`docs`, `notes`, `src` are unreadable smudges), and plain folders (`deep`) are grey while special folders are colored, so the tree looks inconsistent. Render the SVGs at device pixel ratio, and consider a folder-only mode. | `sidebar.png` |
| ICO-05 | P2 | The activity rail has an 8 px strip between the window edge and the rail with a different background, and the active indicator is clipped at the left edge in the dark snapshot. | `activity.png`, all e2e shots |
| ICO-06 | P2 | Tab icons and labels are not vertically aligned with the close button; inactive tabs have fixed widths with large trailing gaps (`guide.md`, `logo.svg`). | `20b-image-svg.png`, `23-render-mermaid.png` |
| ICO-07 | P2 | A deleted file is shown as orange text with a `!` suffix (`two.md!`). Use the theme's warning token, plus a strikethrough or an icon badge instead of punctuation in the file name. | `13-tab-menu.png` |
| ICO-08 | P2 | Context menus have no icons, no shortcuts, no separators, and the destructive "Delete…" item is not styled as destructive. The menu opens over the label of the item it acts on. | `08b-context-menu.png` |
| ICO-09 | P3 | The settings dialog uses bordered buttons for its section list (it should be a selectable list); the close `×` is ~8 px; the text-scale stepper arrows are tiny. | `03-settings-appearance.png` |
| ICO-10 | P3 | The tab overflow `…` and history icons have no visible pressed state, and the hovered state is the same as the active state for the sidebar toggle. | `13-tab-menu.png` |

Reading view:

| ID | Sev | Finding |
|---|---|---|
| RDV-01 | P2 | Display math is rendered at roughly 3.5× body size (`01-initial.png`); inline math is visibly larger and sits above the text baseline (`media-dark.png`). Scale to the body size × a token factor. |
| RDV-02 | P2 | Mermaid diagrams are scaled down to fit the column until their text is ~6 px (`19-scroll-stress.png`). Set a minimum scale, and allow horizontal scroll or the lightbox beyond it. |
| RDV-03 | P2 | The Mermaid background box uses a fixed grey that does not match the dark theme (`media-dark.png`). |
| RDV-04 | P3 | Image captions/alt text are drawn to the right of the image ("Animated") instead of below it. |
| RDV-05 | P3 | Task list rows use native checkboxes with ~30 px row spacing, much looser than normal list items. |
| RDV-06 | P3 | Headings have almost no top margin after a code block or diagram ("Code and tasks"). |

Resolution: icon sizes use `crates/mdview-ui/src/metrics.rs`; duplicate sidebar/split controls
were removed; the README icon is documented; Material icons rasterize at the device pixel ratio;
the activity rail is gone; tabs are tighter and centered; missing files use warning color and a
strikethrough; context menus have icons, separators, shortcuts, and destructive styling;
settings tabs and steppers were restyled; pressed states were added. Math follows the body size,
Mermaid enforces a minimum scale with horizontal scroll and a matching background, image
captions sit below the image, task rows are compact, and headings get top spacing.

## 5. Layout and interaction changes (requested)

### UX-01 (P1 — resolved) — Remove the activity rail; move search to the title bar

- Remove the left activity rail for now. Its explorer toggle already exists in the title bar,
  the command palette is in the title bar and has a shortcut, and settings is in the title bar.
- The title bar becomes: `[app] [menus] … [◀ back] [document name] … [🔍 search] [layout] [settings] [window]`.
- **Back** sits to the left of the document name. It navigates the history of opened documents
  and internal links (Alt+Left / Alt+Right); it is disabled when there is no history.
- **Search** (magnifier) sits to the right of the document name and opens one search popover
  with two scopes:
  - **This document** — the same engine as the editor's find (it also works in Reading mode,
    highlighting matches in the preview).
  - **Workspace** — full-text search across indexed workspace files, with results grouped by
    file and line; activating a result opens the file at that line.
  - The scope is a segmented toggle inside the popover, and the last used scope is remembered.
  - Ctrl+F opens it in document scope; Ctrl+Shift+F opens it in workspace scope.
- Workspace full-text search is new functionality. Keep it off the UI thread, cancellable, and
  respecting `.gitignore` and the large-file limits.

### UX-02 (P0, bug — resolved) — No way to leave Split view

The tab bar's split button only ever sends `SetMode(Split)`
(`crates/mdview-app/src/tab_bar.rs`, around line 161). Once in Split, the user cannot get back to
Reading-only or Editor-only from that control. The title-bar button behaves the same way.

Replace it with a **three-way segmented control** in the same place, for Markdown documents:

```text
[ View | Editor | Split ]
```

- Exactly one segment is active and shows the current mode.
- For documents that do not support a mode, the unsupported segments are **disabled** (not
  hidden): image documents enable View only; plain text and code enable Editor only.
- The segmented control replaces both existing split buttons (see ICO-02).
- Keep the keyboard shortcuts and palette commands for all three modes.

### UX-03 (P1 — resolved) — Find bar redesign

The current find bar (`14-find.png`) is a full-width row with native checkboxes ("Aa", "Word",
".*"), text buttons ("Previous", "Next", "Close"), and "1 matches". It pushes the editor down
(contributing to SCR-01) and does not follow the app's visual language.

Target (VS Code-like, using app tokens):

- A floating overlay anchored to the top-right corner of the editor; it does not resize the
  editor viewport.
- Text field with inline icon toggles for match case, whole word, and regex (icon buttons with
  pressed state, not checkboxes).
- Count shown as `3 of 12` / `No results`, with correct singular/plural through i18n.
- Previous/next/close as icon buttons with tooltips and shortcuts (Enter, Shift+Enter, Esc).
- An expand chevron reveals the replace row (replace, replace all).
- Highlight every match in the editor and in the scrollbar track.
- This overlay is the "This document" scope of UX-01.

### UX-04 (P1 — resolved) — Image zoom and pan

Image documents and the lightbox must support zoom and pan for raster images and SVG:

- Zoom in/out with Ctrl+wheel, pinch, `+`/`-` keys, and toolbar buttons; zoom around the cursor.
- Pan by dragging, and by wheel/trackpad when zoomed in.
- Fit to window, actual size (100%), and a zoom percentage indicator.
- SVG is re-rasterized at the new scale (bounded by the media pixel budget) so it stays sharp.
- The image document currently shows a floating "SVG" label at the bottom-left that duplicates
  the status bar (`20b-image-svg.png`); replace it with the zoom toolbar.

### UX-05 (P1 — resolved) — Editor looks unfamiliar compared to VS Code-style editors

Reported: the editor "looks strange" compared to what users expect. Observed differences:

| Area | Current | Expected |
|---|---|---|
| Current line | Very faint highlight; no gutter number emphasis | Clear current-line background and bold/brighter line number |
| Gutter | Different background from the text area, no separator, numbers dim | Same background, active number highlighted, room for fold/diff markers |
| Left edge | 8 px band of another color before the gutter (`editor-dark-nowrap.png`) | Gutter starts at the pane edge |
| Indentation | No indent guides | Indent guides, with the active scope highlighted |
| Whitespace | Not configurable | Render whitespace option (selection-only by default) |
| End of file | Hard stop at the last line | Scroll beyond last line (SCR-03) |
| Find | Inline row that moves the text (UX-03) | Floating overlay |
| Horizontal scroll | Scrollbar over the text (SCR-05) | Overlay scrollbar |
| Minimap / sticky scroll | Absent | Post-1.0, document as a non-goal |
| Font | Fixed 13 px monospace, independent of the Reading typography setting | Configurable editor font size and line height |

Resolution: the activity rail was removed; the title bar has back/forward history, document
name, search, sidebar, command palette, settings, and window controls; search has document and
workspace scopes on a cancellable worker; the tab bar has a View/Editor/Split segmented control
with disabled unsupported modes; find is a floating overlay; image documents and the lightbox
have zoom, pan, fit, and actual-size controls; the editor has current-line emphasis, matching
gutter background, indent guides, whitespace rendering, scroll-beyond-last-line, an overlay
horizontal scrollbar, and configurable font size/line height. E2E stages cover all of them.
Golden-image snapshots at 1×/2× cover the title bar, tab bar, and explorer.

## 6. Feature-by-feature E2E status

E2E (`cargo xtask e2e`, Xvfb, debug profile) passed every stage. The table shows what is
exercised and what is not verified.

| Feature | Exercised | Verified visually in this review | Gaps |
|---|---|---|---|
| Startup, workspace, logs | yes | yes | — |
| Menus (File/Edit/View/Go/Help) | yes | yes | No keyboard navigation of menus |
| Settings (6 tabs, switches, steppers) | yes | yes | ICO-09; dark theme not covered by E2E |
| Explorer (nested, context menu, rename, delete, create, collapse, refresh) | yes | yes | ICO-04, ICO-08; no keyboard tree navigation |
| Quick Open, command palette | yes | yes | — |
| Tabs (select, close, close others, overflow, wheel scroll) | yes | yes | ICO-06, ICO-07 |
| Editor (edit, save, find, replace, go-to-line) | yes | yes | UX-03, UX-05; no end-of-file scroll assertion (SCR-01) |
| Split + 120 wheel events | yes | yes | Only frame time is checked, not final positions; cannot leave Split (UX-02) |
| Rendering (LaTeX, tables, links, local/remote PNG/SVG, animated SVG, Mermaid, code, tasks) | yes | yes | RDV-01..06; preview end not asserted (SCR-02) |
| Image documents (SVG, PNG) | yes | yes | No zoom/pan (UX-04) |
| Reading presets and size stepper | yes | yes | — |
| Lightbox | **no** | no | Add an E2E stage |
| Conflict banner / diff on external change | **no** (unit snapshot only) | snapshot only | Add an E2E stage with an external write |
| Spanish UI / pseudo-locale | **no** | no | Add a language-switch stage |
| Dark themes | **no** | snapshot only | Run the render stage in a dark theme |
| Back navigation, workspace search | n/a | — | New features (UX-01) |

## 7. E2E additions required by this review

1. **DONE** — Assert the editor offset is stable over 300 wheel events at the end of the file (SCR-01).
2. **DONE** — Assert the last preview block is fully visible after `End` and after continuous wheel input
   on a media-heavy document (SCR-02).
3. **DONE** — Assert the last line can reach the top of the editor viewport (SCR-03).
4. **DONE** — Cycle View → Editor → Split → View with the segmented control, and assert disabled segments
   for image and code documents (UX-02).
5. **DONE** — Lightbox and image zoom/pan: zoom in, pan, fit, 100% (UX-04).
6. **DONE** — External modification of an open dirty file → conflict banner → diff → resolve.
7. **DONE** — Switch to Spanish and the pseudo-locale and capture every dialog.
8. **DONE** — Run the render stage in one dark theme.
9. **DONE** — Golden-image comparison for the title bar, explorer, and tab bar at 1× and 2× scale, so icon
   regressions (ICO-01..07) fail the run instead of depending on manual review.

## 8. Completed order

1. **DONE** — REL-01..04, REL-07 (licenses, deny, CI gate).
2. **DONE** — UX-02 and SCR-01/SCR-02 with their E2E assertions.
3. **DONE** — UX-01, UX-03, ICO-01/02.
4. **DONE** — SCR-03, UX-04, UX-05.
5. **DONE** — ICO-03..10 and RDV-01..06 polish. `1.0.0-beta.1` and H10/H11 remain gated on
   signed Windows/macOS artifacts and the platform manual matrix.


## 9. Resolution evidence (0.14.0)

```text
cargo fmt --all -- --check                       pass
cargo clippy --workspace --all-targets -D warnings pass
cargo test --workspace --locked                  pass
cargo deny check                                 advisories/bans/licenses/sources ok
cargo xtask check-file-length                    pass
cargo xtask perf                                 120 Hz budget
cargo xtask e2e                                  PASS, all review stages
cargo xtask release-check                        pass
```

Key automated evidence:

- `crates/mdview-editor/tests/scroll_end.rs` — stable editor end offset.
- `crates/mdview-render/tests/scroll_end.rs` — media-heavy preview reaches the last block.
- `crates/mdview-render/tests/render_snapshots.rs` — light/dark/compact/custom typography.
- `crates/mdview-ui/tests/ui_snapshots.rs` — header 1×/2×, search popover, settings.
- `crates/mdview-app/src/tabs_tests.rs` — tab bar 1×/2× and mode restrictions.
- `crates/mdview-app/src/sidebar.rs` tests — explorer 1×/2×.
- `scripts/e2e/run.sh` — search/history, segmented modes, image zoom/pan/lightbox, conflict,
  Spanish/pseudo-locale, dark theme, and split-end stability stages.
- `scripts/e2e/run.sh` scroll-end matrix — 300 wheel events in Edit and Split, each with word
  wrap off and on; asserts the last two editor offsets differ by ≤0.1 px and the scrollbar
  clearance is at least 6 px.
- `scripts/e2e/run.sh` End-key stage — one `End` press scrolls the reading view to its maximum;
  the logged `preview end via End` metrics assert `offset == max` and
  `last_bottom <= content`.
- Image viewer evidence — the zoom stage logs successive `image reraster scale=2.50/3.00/3.50`
  lines when SVG crosses half-step raster buckets.
- Lightbox evidence — the zoom stage also logs `lightbox reraster scale=1.50` and `2.00` after
  pressing the lightbox `+` control.
- Code-document mode evidence — opening `src/segmented.rs` logs
  `mode=edit can_edit=true can_view=false can_split=false`; clicking View/Split produces no
  `SetMode` command, and `36-code-modes.png` shows Edit selected with the other segments dimmed.
- Explorer context menu — the menu is positioned at `row.left(), row.bottom() + 1`; the E2E
  records `explorer context menu ... y=...` below the row and captures `08b-context-menu.png`.
- `crates/mdview-app/src/sidebar_keyboard.rs` — keyboard tree traversal/expansion ordering.

### Acceptance mapping

| Criterion | Evidence |
|---|---|
| SCR-01, 300 wheel events in Edit and Split, wrap off/on | Release E2E scroll-end matrix; log tags `mode=edit/split wrap=false/true`; last two offsets equal within 0.1 px |
| SCR-01, horizontal scrollbar clearance | Every matrix log includes `clearance=10.0`; `scroll_end_is_stable` asserts `bottom_clearance >= 6 px` |
| SCR-02, one End key reaches the final preview block | Release E2E End stage; `preview end via End offset=2645.2 max=2645.2 last_bottom=3231.2 content=3243.2` |
| SCR-05, scrollbar clears the last line | `crates/mdview-editor/tests/scroll_end.rs` with scroll-beyond disabled; E2E `clearance=10.0` |
| ICO-08, menu no longer covers the row label | `egui::Popup::context_menu` anchored at `row.left(), row.bottom() + 1`; `08b-context-menu.png` |
| UX-04, SVG re-rasterized at zoom | Viewer logs `image reraster scale=2.50/3.00/3.50`; lightbox logs `lightbox reraster scale=1.50/2.00` |
| Section 7.4, code document disabled modes | `document mode .../segmented.rs mode=edit can_edit=true can_view=false can_split=false`; no `SetMode` on View/Split clicks; `36-code-modes.png` |
