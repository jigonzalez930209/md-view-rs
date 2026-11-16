# Release 1.0.0-rc.0

**Release date:** 2026-11-15

MD View 1.0.0-rc.0 is the first public release candidate. It is a native Rust desktop editor with a
GitHub-parity Markdown reading view, a virtualized code editor, a workspace explorer, and a
self-contained media renderer.

## Highlights

- **GitHub Markdown parity.** The reading view follows GitHub's measured body layout: block
  spacing, heading ratios, alerts, tables, code chips, footnotes, safe HTML, math, Mermaid, and
  GitHub Primer syntax colors.
- **Faster file switching.** Syntax highlighting now uses syntect's Oniguruma engine with two-face's
  matching dumps. A cold TypeScript highlight dropped from about 2.8 s to 33 ms in debug builds and
  from 254 ms to 18 ms in release.
- **Explorer follows tabs.** The active tab expands its folders, scrolls into view, and is marked
  in the tree. Open documents outside the workspace appear in a fixed **External files** group.
- **Taller search results.** Document and workspace search results grow with the window and scroll
  instead of being clipped to a single row.
- **Unsigned desktop builds.** Linux packages are ready to use. macOS and Windows RC builds are
  unsigned; signing and notarization are the remaining 1.0.0 release-operator steps.

## Downloads

Use the [GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.0)
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
