# Release 1.0.0-rc.3

**Release date:** 2026-11-16

MD View 1.0.0-rc.3 is the third public release candidate. It is a native Rust desktop editor with a
GitHub-parity Markdown reading view, a virtualized code editor, a workspace explorer, and a
self-contained media renderer. This candidate makes the editor predictable (Enter, typing, wrap,
and keyboard focus), adds a Markdown formatting bar with list and table editing, and fixes the
explorer and tab strip. 1.0.0-rc.2 was prepared but not published; its changes are included.

## Highlights

- **The editor behaves.** Enter and Space no longer move the cursor to the mouse pointer, typing
  no longer recentres the view, up/down never jump into the next line, Tab indents instead of
  leaving the editor, and dragging selects from where the button went down.
- **Markdown formatting bar.** Headings, bold, italic, underline, quote, code, link, a Word-style
  table picker, and bulleted, numbered, and task lists, with toggles that show the formats under
  the cursor, `Ctrl/Cmd+B/I/U/K`, and **Format: …** palette commands. See
  [Markdown formatting](/guide/editor#markdown-formatting).
- **Lists and tables continue as you type.** Enter continues and renumbers lists and leaves them on
  an empty item; Tab nests items and moves between table cells, adding rows and realigning columns.
- **Clean soft wrap** at word boundaries, with no repeated text and the cursor and selection on
  the text.
- **Chinese, Japanese, and Korean text** uses an installed system font, and wide characters keep
  the editor grid aligned.
- **Fast typing in large files:** under 1 ms per key in a 210,000-line file.
- **Find and replace:** Enter steps through matches and keeps focus, regex `^`/`$` work per line,
  `$1` expands in replacements, and the panel no longer shifts while typing.
- **Explorer and tabs:** aligned rows, keyboard navigation after a click, compact context menus,
  and a tab strip that scrolls to the active tab.

### Included from earlier candidates

- **From the unpublished rc.2.** Split view ends together with exact sync, the editor has focus
  without a click, **Insert LaTeX Block** (`Ctrl/Cmd+Shift+M`), a steady editor end, and Alt keys
  no longer type into the editor.
- **Flicker-free split view (rc.1).** A continuous, invertible map links editor and preview; zoom,
  resize, wrap, and late media keep the panes locked, and typing never moves the editor.
- **GitHub Markdown parity (rc.0).** Block spacing, heading ratios, alerts, tables, code chips,
  footnotes, safe HTML, math, Mermaid, and GitHub Primer syntax colors.
- **Faster file switching**, the **explorer that follows tabs** with a fixed **External files**
  group, and **taller search results**.
- **Unsigned desktop builds.** Linux packages are ready to use. macOS and Windows RC builds are
  unsigned; signing and notarization are the remaining 1.0.0 release-operator steps.

## Downloads

Use the [GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.3)
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
