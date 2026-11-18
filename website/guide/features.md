# Feature matrix

The 1.0.0-rc.5 release candidate covers the following surface. The repository's
`docs/feature-matrix.md` records the test or snapshot evidence for each row.

## Application

| Feature | Status |
|---|---|
| Native desktop shell, no webview | Yes |
| Windows, macOS, Linux builds | Yes (unsigned RC) |
| Tabs, recent files, session restore | Yes |
| Lazy workspace tree and watcher | Yes |
| Active tab reveal and external-files group | Yes |
| Command palette, Quick Open, keyboard menus | Yes |
| English, Spanish, pseudo-locale | Yes |
| Nine themes, light/dark and system follow | Yes |
| 0.8–2.0 UI zoom, 2× snapshots | Yes |
| Atomic save, conflict diff, missing-file recovery | Yes |

## Markdown

| Feature | Status |
|---|---|
| CommonMark and GFM | Yes |
| GitHub-parity preview layout | Yes |
| Tables, tasks, alerts, footnotes | Yes |
| Safe HTML subset | Yes |
| Syntax-colored code fences | Yes |
| LaTeX inline and display math | Yes |
| Mermaid flowcharts and sequence diagrams | Yes |
| Local PNG/JPEG/GIF/WebP/BMP/TIFF/SVG | Yes |
| Cached remote images | Yes |
| Animated SVG subset | Yes |
| Image zoom, pan, fit, lightbox | Yes |

## Editor

| Feature | Status |
|---|---|
| Virtualized rows | Yes |
| Syntax highlighting with per-line cache | Yes |
| Find/replace with regex | Yes |
| Go to line | Yes |
| Wrap and line numbers | Yes |
| Encoding and EOL detection | Yes |
| Mixed-EOL preservation | Yes |
| Large-file mode with background load | Yes |
| IME commit and surrounding delete coverage | Yes |
| Scroll-beyond-last-line and split sync | Yes |
| Equation gallery and formula editor (live preview, source-to-formula hover) | Yes |

## Packaging

| Platform | Assets |
|---|---|
| Linux | tarball, deb, RPM, AppImage, AppDir |
| macOS | `.app` in a DMG and zip; unsigned RC |
| Windows | per-user MSI and portable zip; unsigned RC |
| Release metadata | SHA-256 checksums, CycloneDX SBOM, update manifest |
| Update verification | SHA-256 before install; AppImage/deb/RPM/MSI/DMG routing |

## Known RC limitations

- macOS and Windows builds are not signed or notarized yet. The final 1.0.0 release adds the
  platform certificates.
- The Windows MSI is per-user and registers Markdown associations under `HKCU`; no machine-wide
  installer is published.
- The Flatpak manifest is present in the repository but is not part of the RC asset set.
- The manual screen-reader and real-hardware checks in `docs/release-checks.md` remain 1.0.0
  release-gate work.
