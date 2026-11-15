# Changelog

## Unreleased

- Replaced the three Reading width presets with a 5–40% margin slider and added an
  **Equations** size setting for inline and display math.
- Updated split scrolling to interpolate within multi-line blocks in both directions, so the
  editor and preview stay on the same source position instead of snapping to block tops.
- Replaced the View/Editor/Split text segments with eye, pencil, and split icons.
- Centered list bullets, ordered numbers, and task boxes on the first text line.
- Moved search to a command-palette-style panel anchored at the top center.
- Restored the back/forward history buttons and the selected document name in the title bar.
- Added 2 px more padding around the File/Edit/View/Go/Help menu labels.
- Made the preview end-stick tolerance account for media that finishes measuring after a
  scroll.


## 0.14.0 — Pre-1.0 review resolution

- Removed the activity rail and moved the workbench controls into the title bar: back/forward
  history, document/workspace search, sidebar, palette, settings, and window controls.
- Added a floating document/workspace search popover on a cancellable worker, highlighting
  matches in the reading view and activating results at the source line.
- Added the View/Editor/Split segmented control with disabled modes for images and code files,
  replacing the duplicate sidebar and split toggle buttons.
- Redesigned find/replace as a floating overlay with icon toggles, match counts, and a
  collapsible replace row.
- Stabilized editor and preview scroll ends: fixed-height content independent of transient
  scrollbars, sticky bottom, width/media-aware estimates, and scroll-beyond-last-line.
- Added editor typography controls (font size, line height), indent guides, whitespace rendering,
  current-line emphasis, matching gutter background, and overlay scrollbar clearance.
- Added image zoom, pan, fit, actual-size, and a toolbar; the lightbox gained the same controls.
- Added icon-size tokens, device-pixel-ratio Material icons, context-menu icons and destructive
  styling, tab alignment, missing-file styling, and settings-dialog polish.
- Scaled math to the reading typography, enforced a legible Mermaid minimum with horizontal
  scroll and a matching background, moved image captions below images, and tightened task rows.
- Added licenses, `deny.toml`, `cargo xtask ci`, hosted CI, release-check dependency pin checks,
  a disk-backed test note, and split `commands`, `document_ops`, and `media_runtime` into
  responsibility-based submodules.
- Added scroll-end, media-heavy preview, history, search, editor-style, 1x/2x golden-image,
  Spanish/pseudo-locale, dark-theme, lightbox, conflict, and segmented-mode coverage.


## 0.13.0 — Reading typography, aligned lists, and viewer plugins

- Added a **Reading** settings tab with three width presets (Comfortable, Compact, Full width)
  and independent point sizes for body text, headings, code, and tables.
- Reduced scrollbar width from 10 px to a 6 px active handle with a 3 px dormant line.
- Kept the tab close button visible while the pointer is anywhere over the tab; it no longer
  fades out when the pointer moves onto the button.
- Vertically centered bullets, ordered numbers, and task checkboxes on the first line of each
  list item instead of aligning them to the top of the block.
- Disabled ScrollArea animation for programmatic split-sync targets and added sync diagnostics,
  removing the end-of-document overshoot and flicker.
- Introduced the read-only viewer plugin API: `ViewerPlugin`, `ViewerInstance`, `ViewerContext`,
  and a priority registry. The image viewer is now the built-in image plugin.
- Added `docs/plugins.md`, the custom-typography snapshot, and E2E coverage for all reading
  presets and the body-size stepper.


## 0.12.0 — Cursor-style workbench, image documents, and compact reading

- Moved the explorer directly under the menu bar. The folder tree and activity rail now occupy
  the full workbench height, and the document tabs belong to the editor column only, matching
  VS Code/Cursor.
- Reworked the tab strip for that model: raised chrome surface, active tab merging with the
  editor background, muted inactive labels, 34 px height, and 16 px file icons.
- Increased activity-rail icons to 24 px, explorer action icons to 16 px, and tree chevrons to
  14 px while keeping 21 px tree rows.
- Opening PNG, JPEG, GIF, WebP, BMP, TIFF, or SVG files now shows a dedicated image viewer
  instead of decoding the file as text. The viewer fits the pane, keeps SVG animation, opens
  the lightbox on click, and reports the format in the status bar.
- Added the Appearance setting **Compact reading margins** to widen Markdown previews and trim
  their top and bottom spacing.
- Added compact-preview, image-document, and settings coverage to the automated snapshots and
  E2E for the new explorer/tab layout.


## 0.11.3 — Tab wheel feel, LaTeX sizing, and table links

- Tab strips now consume raw wheel events at 40 px per notch instead of replaying egui's
  smoothed scroll delta, so one notch moves one controlled step and the strip no longer
  overshoots to the end after the wheel stops.
- Inline LaTeX keeps ordinary formulas close to the text height while giving fractions,
  integrals, and sums the extra height they need; display formulas are centered and rendered
  at full readable size.
- Links inside table cells are now clickable, with a pointing-hand cursor and destination
  tooltip, including local workspace documents.
- Added a table link to `showcase.md` and the E2E, which opens `docs/guide.md` from the table.
- Verified the final release E2E with zero frames over the 8.33 ms budget.


## 0.11.2 — Nested folders, GitHub-style code, and the pre-1.0 workspace

- Fixed the remaining nested-folder collapse: any watcher refresh of a parent directory now
  records and re-applies every expanded descendant, not only root-level folders.
- Added `examples/v1-workspace`, five folders nested one inside the next with alternating
  Markdown, JSON, shell, CSV, Python, TSX, Rust, PNG, SVG, and Mermaid files, plus a full
  render matrix in `showcase.md`.
- Added `docs/feature-matrix.md` with per-feature implementation and automated evidence.
- Reworked code blocks: 19 px line height, a separated header with language and copy, no
  trailing blank line, horizontal scroll for long lines, and visible inline-code chips.
- Increased LaTeX inline sizing and restored full-size display formulas.
- Extended the release E2E with five-level expansion, a watcher refresh, the fifth-level file
  open, and a robust edit/save handshake.


## 0.11.1 — Table, list, tree, and tab-scroll fixes

- Rebuilt table rendering with measured, weighted columns that fill the reading width, header
  surfaces, zebra rows, separators, and wrapped cell text.
- Fixed tight GFM lists: inline code and surrounding text stay in one paragraph, so long
  checklist lines no longer break into one fragment per line.
- Constrained list content to the full available reading width; aligned task checkboxes with
  the first text line and made them square.
- Reduced explorer row height by 2 px, disabled text-selection cursors over tree rows, and kept
  nested folder expansion across root refreshes.
- Added horizontal mouse-wheel scrolling to the open-tab strip.
- Extended the E2E with tab scrolling, long checklist content, and release frame-budget checks.


## 0.11.0 — Workbench parity, diagnostics, and full E2E

- Matched the reference workbench layout: title bar menus, activity rail, document tabs,
  explorer only when a folder is loaded, and a settings modal with Appearance, Editor,
  Explorer, Media, and About tabs.
- Added compact, homogeneous context menus with 12 px labels and tight spacing; animated
  document tabs and settings tabs with hover and selection transitions.
- Introduced shared controls: vertically centered text fields, consistent 30 px primary and
  secondary buttons, and numeric fields with up/down arrows.
- Fixed the explorer: rows are clickable, opening a folder reveals the panel and keeps it open,
  root refreshes preserve expansion, collapse-all works, and changes request a repaint.
- Quick Open now searches the workspace file index, not only open and recent files.
- Rendered GFM task lists with drawn checkboxes, improved table column spacing, and kept
  LaTeX, Mermaid, local/remote PNG and SVG, and HTTP media covered by the E2E.
- Added `cargo xtask e2e`: an Xvfb-driven button-by-button test for menus, settings, prompts,
  explorer operations, tabs, find/replace/goto, split scrolling, all renderers, and shutdown.
- Added rotating diagnostics: UTC log lines for every command, file open/save timings, workspace
  indexing, media loads, errors, and 120 Hz frame metrics, capped at 1 MiB total.
- About now reports diagnostic log usage and opens the log folder.


## 0.10.1 — 120 Hz frame budget and large-file scrolling

- Added CPU frame-time instrumentation to the editor, preview, and full application, with a rolling 120-frame p50/p95/max/over-budget log.
- Added `cargo xtask perf` and a release 120 Hz benchmark with a strict 8.33 ms combined p95 gate.
- Disabled expensive editor syntax highlighting above 2,000 lines or 1 MB; the visible preview still highlights code fences.
- Fixed the source-map gap fallback that made preview scrolling jump to the end of large documents.
- Documented the measurement method and results in `docs/performance.md`.


## 0.10.0 — Minimalist workspace and virtualized preview

- Replaced the text menu bar with a compact icon header: direct sidebar, open-folder, save, Edit/Reading/Split, search, command-palette, and settings actions plus a single hamburger menu.
- Added a VS Code Material Icon Theme file tree: the upstream manifest, full SVG catalog (including open-folder variants), light-theme overrides, exact file/extension/folder resolution, and cached textures.
- Added workspace toolbar icons and a consistent 16 px icon rhythm in the sidebar.
- Virtualized the Markdown preview by block with cached measured heights and estimated offscreen heights; the source map still covers every block for scroll synchronization.
- Kept split scroll synchronization stable across bidirectional wheel scrolling and 120-event stress runs; fixed the gap fallback that sent the editor to the end.
- Fixed shifted shortcut precedence (`Ctrl+Shift+E`, `Ctrl+Shift+S`, `Ctrl+Shift+O`, `Ctrl+Shift+Tab`) and replaced the panel split with a draggable 50/50 split.
- Added header, sidebar, and virtualization regression snapshots/tests; documented icons in `docs/icons.md`.

## 0.9.0 — Distribution, associations, and updates

- Added `cargo xtask package`: release build, portable tarball, Debian package, RPM, AppDir, optional AppImage, multi-size PNG/ICO/ICNS icons, SHA-256SUMS, CycloneDX SBOM, and an update manifest.
- Added `cargo xtask release-check` and `cargo xtask icons`; the release check validates every manifest asset and packaging input.
- Added Linux desktop, MIME, AppStream, hicolor icon trees, and package maintainer hooks.
- Added WiX v4 per-user MSI sources, macOS `Info.plist`/entitlements/DMG script, and a Flatpak manifest with portal-based permissions.
- Added Markdown file associations for `.md`/`.markdown` across the three packaging families; `.txt` remains opt-in.
- Added manual update checking: manifest fetch, version comparison, platform asset selection, timeout/limit bounds, SHA-256 verification, download cache, prompt, and installation.
- Added update installation paths: atomic AppImage self-replace, `pkexec dpkg`/`rpm`, Flatpak update routing, MSI/DMG launchers, and a detached restart waiter.
- Added install-plan and AppImage replacement tests; dirty documents block installation until saved.
- Documented the local packaging matrix, signing requirements, associations, install/uninstall, and update flow in `docs/release.md`.
- Generated AppImage, Debian, RPM, portable tarball, icons, SBOM, checksums, and update manifest locally; `release-check` passes against all four assets.

## 0.8.0 — Themes, polished UI, and accessibility

- Added nine stable theme variants across GitHub, Dracula, and One Dark Pro, with complete color tokens, legacy ID migration, and follow-system family switching.
- Added WCAG 2.2 contrast gates for every variant and regenerated the visual snapshots for the new palette.
- Added a settings window for theme, text scale, language, editor behavior, media policy, animations, and reduced motion.
- Added command palette (`Cmd+Shift+P`), quick open (`Cmd+P`), settings (`Cmd+,`), and 80–200% text scaling shortcuts.
- Added a centered 70–80-character reading column, improved table sizing, 28-point controls, focus strokes, and consistent radii.
- Added the pseudo-locale, completed the Spanish UI catalog for shell strings, and localized all settings and palette labels.
- Added settings and palette snapshots plus three theme reference snapshots.
- Documented the theme, accessibility, and UI contracts in `docs/themes.md`.

## 0.7.0 — Animated SVG and hardening

- Added a bounded SMIL and CSS animation engine for SVG: `animate`, `animateTransform`, `set`, `@keyframes`, keyTimes, linear/discrete interpolation, repeat, and begin delays.
- Added 30 fps animated-SVG playback with play/pause, reduced motion, offscreen pause, and generation-based cancellation on tab close.
- Added an “Animated”/“Animation not fully supported” badge and static-first fallback for unsupported motion features.
- Added shared syntax highlighting so opening and closing tabs no longer rebuilds the syntax and theme sets.
- Added depth-limited Markdown parsing: nesting beyond 64 levels is flattened instead of overflowing the stack.
- Added fuzz-style random and hostile corpus tests for Markdown, SVG, LaTeX, and Mermaid.
- Added a 50 MB / >100k-line editor test, a 100 open/close cycle test, and large-file preview refusal with a notice.
- Added `docs/animation.md` with the supported subset and verification matrix.

## 0.6.0 — Images, SVG, math, and Mermaid

- Added `mdview-media`: bounded local/remote image loading, disk caching with ETag revalidation, image decoding with size limits, static SVG parsing/rasterization, RaTeX math rendering, and native Mermaid rendering.
- Added a background media worker with a 16 MB stack, generation-based cancellation, and a 64 MB LRU texture cache.
- Added inline images and image lightbox with 50%/100%/200% zoom; missing media shows an inline reason.
- Added static SVG security checks for scripts, `javascript:`, `foreignObject`, entities, and external resources.
- Added LaTeX inline/display/`math` fence rendering with embedded KaTeX fonts and dark-theme color.
- Added Mermaid fences with fit-to-width rendering, a zoomable lightbox, source fallback, and a 20-diagram Tier A test.
- Added `View → Media` toggles for remote images and private-network hosts; private networks stay blocked by default.
- Added rich-content light/dark snapshots and a 100-formula corpus test.

## 0.5.0 — Split view and synchronized scrolling

- Added a resizable 50/50 split with the editor on the left and the reading view on the right; narrow windows stack the preview below the editor.
- Added per-file split ratios persisted in the session and restored on reopen.
- Added bidirectional block-anchor scroll synchronization with line/fraction interpolation, block preservation across re-renders, and programmatic scroll echo suppression.
- Added a hover-aware rule when both panes scroll in one frame and a `Synchronize scrolling` toggle in the View menu.
- Added editor scroll APIs (`first_visible_line`, `scroll_to_line`, user-scroll reporting) and preview viewport/anchor reporting.
- Added split layout snapshots (horizontal and vertical), editor scroll tests, and a 1,000-frame oscillation test.

## 0.4.0 — Markdown reading view

- Added `mdview-markdown`, a spanned CommonMark 0.31 + GFM AST with tables, task lists, strikethrough, autolinks, footnotes, alerts, hidden front matter, math events, heading anchors, and link classification.
- Added deterministic GitHub-style heading slugs with explicit `{#id}` support and collision suffixes.
- Added `mdview-render`, a token-styled reading view for headings, paragraphs, emphasis, strong, strikethrough, links, lists, task lists, block quotes/alerts, tables, code fences with syntax highlighting and copy, rules, footnotes with backlinks, math placeholders, and safe inline HTML (`br`, `sub`, `sup`, `kbd`, `mark`); other HTML is shown as text or discarded and never executed.
- Added per-tab Edit/Reading presentation modes and automatic reading view for Markdown files.
- Added internal link handling: anchors scroll in place; workspace links resolve with `craft-docpath`, open or activate a tab, and scroll to the heading; external links open in the platform browser.
- Added a block source map with interpolation, re-render anchor preservation, and validation.
- Added editor and preview scroll offsets per mode to the persisted session.
- Added light/dark reading-view snapshots and parser, link, source-map, mode, and session tests.

## 0.1.0 — Native shell foundation

- Added the Cargo workspace and the initial `mdview-*` crate boundaries.
- Added an eframe/egui native shell with a menu bar, status bar, welcome screen, and monospace document editor.
- Added persisted light/dark and language preferences through `craft-settings`.
- Added English/Spanish UI translation through `craft-i18n`.
- Added atomic UTF-8 saves through `craft-atomic`, preserving UTF-8 BOM and LF/CRLF/CR line endings.
- Added native open/save dialogs through `craft-dialogs`.
- Added the unsaved-changes close flow through `craft-quit`.
- Added a command-line interface (`--help`, `--version`, one optional file).
- Added single-instance coordination and file-path forwarding through a local socket.
- Added the first repository checks: formatting, Clippy, tests, and the 500-line file-length limit.

## 0.2.0 — Virtualized code editor

- Replaced the simple text area with a Rope-based, virtualized editor that paints only visible rows.
- Added line numbers, current-line highlighting, cursor and selection rendering, and a monospace gutter.
- Added grouping undo/redo, save points, dirty revisions, and clipboard operations.
- Added syntax highlighting for the curated language set with a lazy per-line cache.
- Added auto-indent, bracket and quote pairing, bracket matching, Tab/Shift+Tab indentation, and soft wrap.
- Added regex find/replace, match highlighting, next/previous navigation, replace-all, and go-to-line.
- Added encoding detection for UTF-8/UTF-16/Windows-1252/ISO-8859-1/Shift_JIS, reopen-with-encoding, BOM preservation, and LF/CRLF/CR/mixed EOL reporting.
- Added tab/space indentation detection and editor use of the detected indent unit.
- Added a 10 MB/200k-line large-file mode, a 100 MB hard limit, background loading with progress and cancellation, and lazy syntax highlighting.
- Preserved per-line mixed line endings on save.
- Added an IME commit smoke test and AccessKit role/value/selection announcements.
- Added cursor line/column, selection count, indentation, language, encoding, EOL, wrap, and large-file status.
- Added local deterministic editor snapshots for light/dark and wrap/no-wrap, plus the remaining macOS/Windows manual checklist.

## 0.3.0 — Workspace, tabs, and real time

- Added document tabs with path deduplication, dirty markers, missing-file markers, close, close-others, close-saved, reorder, `Ctrl+Tab`/`Ctrl+Shift+Tab`, and `Cmd+W`.
- Added a lazy workspace sidebar with folders first, hidden-file and `.gitignore` toggles, selection, expansion, and a user-resizable width.
- Added open-folder, recent workspaces, drag-and-drop, new file, new folder, rename, delete with confirmation, and reveal in the platform file manager.
- Added a debounced `notify` watcher with a polling fallback and workspace tree refresh.
- Added clean-file external reload that preserves the cursor line/column and per-tab scroll offsets.
- Added dirty-file conflict detection, a conflict banner, confirmation dialogs for reload/keep, and a `similar`-based comparison window.
- Added a missing-file state with Save As and keep-editing actions.
- Added session persistence for the workspace, open tabs, active tab, per-file scroll, sidebar state/width, window size, recent files, and recent workspaces.
- Added multi-path command-line and single-instance forwarding.
- Added sidebar, tab-bar, conflict-banner, and conflict-diff snapshots, plus 50k-file lazy-tree and 20-tab session tests.
