# Changelog

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
