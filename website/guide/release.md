# Release 1.0.0-rc.8

**Release date:** 2026-11-18

MD View 1.0.0-rc.8 is the eighth public release candidate. It is a native Rust desktop editor with
a GitHub-parity Markdown reading view, a virtualized code editor, a workspace explorer, and a
self-contained media renderer. This candidate plays video in-process with pure-Rust codecs.

## Highlights

- **Video with nothing to install.** The video viewer decodes in-process with pure-Rust codecs:
  MP4, MOV, 3GP, MKV, WebM, AVI, MPEG-TS/PS, and FLV with H.264, HEVC, AV1, VP8, VP9, MPEG-1/2,
  MPEG-4 Part 2, H.263, or Motion JPEG video and AAC, MP3, Opus, Vorbis, FLAC, AC-3, or PCM sound.
  No FFmpeg is bundled or needed. WMV and OGV show a "not supported" message.

### Included from earlier candidates

- **Horizontal scrollbar only when needed (rc.7).** It appears only when a line is wider than
  the view, and never with soft wrap.
- **More file types (rc.6).** An editable spreadsheet grid for CSV and TSV, PDF and video viewers,
  animated GIF, WebP, and APNG, more image formats including JPEG XL, richer animated SVG, and
  whole-fraction hover in the formula editor.
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

Use the [GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.8)
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
