# Release 1.0.0-rc.4

**Release date:** 2026-11-17

MD View 1.0.0-rc.4 is the fourth public release candidate. It is a native Rust desktop editor with
a GitHub-parity Markdown reading view, a virtualized code editor, a workspace explorer, and a
self-contained media renderer. This candidate restores copy and paste between MD View and the rest
of the desktop.

## Highlights

- **The system clipboard works in both directions.** Copy, cut, and paste now exchange text with
  other applications. The editor and the preview's **Copy** button write to the OS clipboard, and
  text copied elsewhere can be pasted into MD View. The desktop shell had compiled egui with an
  in-process clipboard simulation, so both directions stopped at the window. The E2E run now
  reads a copied line through a separate X11 client and pastes text owned by another client.
- **Safer local gate.** `cargo xtask ci` checks available memory before starting, caps parallel
  Cargo jobs from it, and stops a step before memory pressure can make systemd-oomd kill the
  terminal. Run it as `systemd-run --user --scope --collect cargo xtask ci` on a busy desktop.

### Included from earlier candidates

- **Editor behavior (rc.3).** Enter and Space no longer move the cursor to the mouse pointer,
  typing no longer recentres the view, soft wrap breaks at word boundaries, and Tab, arrows,
  copy, and paste work without clicking the text pane.
- **Markdown formatting bar (rc.3).** Headings, bold, italic, underline, quote, code, link, a
  Word-style table picker, and bulleted, numbered, and task lists, with toggles, `Ctrl/Cmd+B/I/U/K`,
  and **Format: …** palette commands.
- **Lists and tables (rc.3).** Enter continues and renumbers lists; Tab nests items, moves between
  table cells, adds rows, and realigns columns.
- **CJK text and large files (rc.3).** Installed CJK fonts render in the editor and preview, and
  edits in a 210,000-line file re-measure only the touched lines.
- **Find, replace, explorer, and tabs (rc.3).** Regex `^`/`$` per line, `$1` replacements,
  keyboard explorer navigation, compact context menus, and a tab strip that reveals the active tab.
- **From the earlier candidates.** Split view ends together with exact sync, **Insert LaTeX Block**
  (`Ctrl/Cmd+Shift+M`), flicker-free split scrolling, GitHub Markdown parity, Oniguruma
  highlighting, the explorer that follows tabs, and the desktop packages.
- **Unsigned desktop builds.** Linux packages are ready to use. macOS and Windows RC builds are
  unsigned; signing and notarization are the remaining 1.0.0 release-operator steps.

## Downloads

Use the [GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.4)
and verify every file against `SHA256SUMS`.

| Platform | Assets |
|---|---|
| Linux | `.tar.gz`, `.deb`, `.rpm`, `.AppImage` |
| macOS | `.dmg`, `.zip` |
| Windows | `.msi`, `.zip` |

See [Install](/guide/install) for commands and the unsigned-build notes.

## Upgrade and data safety

The app writes documents through an atomic save path, preserves mixed line endings, and blocks
update installation while a tab has unsaved changes. The RC keeps the same settings, session, and
media cache layout as 0.14.0.

## Full changelog

The repository [CHANGELOG.md](https://github.com/jigonzalez930209/md-view-rs/blob/main/CHANGELOG.md)
contains the complete history, including the 0.14.0 pre-1.0 review, the parity work, and every
snapshot or E2E evidence change.
