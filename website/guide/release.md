# Release 1.0.0-rc.1

**Release date:** 2026-11-16

MD View 1.0.0-rc.1 is the second public release candidate. It is a native Rust desktop editor with a
GitHub-parity Markdown reading view, a virtualized code editor, a workspace explorer, and a
self-contained media renderer. This candidate makes split view scrolling and zoom flicker-free.

## Highlights

- **No flicker at the end of split view.** Reaching the end of either pane no longer makes the other
  pane jump and snap back. No frame is ever shown painted past a pane's end.
- **Exact editor and preview synchronization.** A continuous, invertible map links the two panes, so
  both reach their ends on the same frame and either pane can lead without drift.
- **Zoom keeps the panes locked.** Zooming, resizing the split, soft wrap, and late media keep the
  follower aligned within a twentieth of a pixel, and the first frame after a zoom is already correct.
- **Typing never moves the editor.** An edit makes the editor lead, so preview re-layouts never
  scroll the text under the cursor.

### Included from 1.0.0-rc.0

- **GitHub Markdown parity.** The reading view follows GitHub's measured body layout: block
  spacing, heading ratios, alerts, tables, code chips, footnotes, safe HTML, math, Mermaid, and
  GitHub Primer syntax colors.
- **Faster file switching.** Syntax highlighting uses syntect's Oniguruma engine with two-face's
  matching dumps.
- **Explorer follows tabs**, with a fixed **External files** group, and **taller search results**.
- **Unsigned desktop builds.** Linux packages are ready to use. macOS and Windows RC builds are
  unsigned; signing and notarization are the remaining 1.0.0 release-operator steps.

## Downloads

Use the [GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.1)
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
