# Release 1.0.0-rc.2

**Release date:** 2026-11-16

MD View 1.0.0-rc.2 is the third public release candidate. It is a native Rust desktop editor with a
GitHub-parity Markdown reading view, a virtualized code editor, a workspace explorer, and a
self-contained media renderer. This candidate fixes the editor's end of document in split view,
gives the editor keyboard focus without a click, and adds LaTeX block insertion.

## Highlights

- **Split view ends together.** With scroll-beyond-last-line on, the split editor no longer scrolls
  its last row to the top and goes blank while the preview shows the end. Zoom or resize at the
  end keeps both panes at the new end, and split sync lands exactly on its target.
- **The editor has focus when you need it.** Entering Editor or Split mode, switching tabs, and
  closing find or go-to-line return keyboard focus to the editor, so arrows, typing, and
  copy/paste work immediately.
- **Insert LaTeX Block.** `Ctrl/Cmd+Shift+M`, the Edit menu, or the command palette inserts a
  `$$` display-math block on its own lines with the cursor inside.
- **Steadier editor end.** Both editor scrollbars stay allocated, so the end of a document no
  longer flickers when a horizontal bar appears.
- **Alt keys stay out of the text.** Alt menu keys and the conflict keys `Alt+R`/`Alt+K` no longer
  type their letter into the editor.

### Included from earlier candidates

- **Flicker-free split view (rc.1).** A continuous, invertible map links editor and preview; zoom,
  resize, wrap, and late media keep the panes locked, and typing never moves the editor.
- **GitHub Markdown parity (rc.0).** Block spacing, heading ratios, alerts, tables, code chips,
  footnotes, safe HTML, math, Mermaid, and GitHub Primer syntax colors.
- **Faster file switching**, the **explorer that follows tabs** with a fixed **External files**
  group, and **taller search results**.
- **Unsigned desktop builds.** Linux packages are ready to use. macOS and Windows RC builds are
  unsigned; signing and notarization are the remaining 1.0.0 release-operator steps.

## Downloads

Use the [GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.2)
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
