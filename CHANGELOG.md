# Changelog

## 0.9.0 — Distribution, associations, and updates

- Added `cargo xtask package`: release build, portable tarball, Debian package, RPM, AppDir, optional AppImage, multi-size PNG/ICO/ICNS icons, SHA-256SUMS, CycloneDX SBOM, and an update manifest.
- Added `cargo xtask release-check` and `cargo xtask icons`; the release check validates every manifest asset and packaging input.
- Added Linux desktop, MIME, AppStream, hicolor icon trees, and package maintainer hooks.
- Added WiX v4 per-user MSI sources, macOS `Info.plist`/entitlements/DMG script, and a Flatpak manifest with portal-based permissions.
- Added Markdown file associations for `.md`/`.markdown` across the three packaging families; `.txt` remains opt-in.
- Added manual update checking: manifest fetch, version comparison, platform asset selection, timeout/limit bounds, SHA-256 verification, download cache, prompt, and explicit installation guidance.
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
