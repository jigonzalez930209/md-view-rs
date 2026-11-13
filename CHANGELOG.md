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
