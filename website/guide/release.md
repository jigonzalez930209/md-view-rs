# Release 1.0.0-rc.6

**Release date:** 2026-11-18

MD View 1.0.0-rc.6 is the sixth public release candidate. It is a native Rust desktop editor with
a GitHub-parity Markdown reading view, a virtualized code editor, a workspace explorer, and a
self-contained media renderer. This candidate opens far more kinds of files: delimited data in an
editable grid, PDF, video, animated images, and more image formats.

## Highlights

- **Spreadsheet grid for CSV and TSV.** Delimited files open in an editable grid with detected
  delimiters, in-place editing, block copy and paste with spreadsheet applications, row and column
  insertion and deletion, and resizable columns. Edits change only the touched fields of the text.
  See the [editor guide](/guide/editor#delimited-files-csv-tsv).
- **PDF viewer.** Continuous pages, fit-width or manual zoom, and page navigation, rendered in the
  background in pure Rust.
- **Video viewer.** Every format the system FFmpeg reads plays with play/pause, seeking, ±5 s keys,
  and mute.
- **Animated GIF, WebP, and APNG**, plus ICO/CUR, TGA, HDR, EXR, PNM, QOI, Farbfeld, and JPEG XL.
- **Richer animated SVG.** Motion paths, line drawing, spline easing, `fill="freeze"`, and CSS
  delays, iteration counts, directions, and timing functions.
- **Formula editor fix.** Pointing at `\frac` highlights the whole fraction.

### Included from earlier candidates

- **Equation gallery and formula editor (rc.5).** The Σ gallery, the √x formula editor with a
  live preview and template palette, source-to-formula hover, and double-click editing.
- **The system clipboard (rc.4).** Copy, cut, and paste exchange text with other applications.
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

Use the [GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.6)
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
