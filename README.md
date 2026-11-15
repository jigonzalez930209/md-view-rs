# MD View

A native Markdown, text, and code editor for Windows, macOS, and Linux, built with Rust and egui.

The workbench places the explorer directly under the menu bar and keeps document tabs in the editor column, with Cursor-style active tabs. Standalone image files open through the read-only viewer plugin API, and Settings → Reading provides a 5–40% margin slider plus independent body, heading, code, table, and equation sizes. The pre-1.0 acceptance workspace and feature map are in
[`examples/v1-workspace`](examples/v1-workspace) and
[`docs/feature-matrix.md`](docs/feature-matrix.md). The project is at the `0.14.0` pre-1.0 milestone: a native shell with document tabs, a lazy folder tree, debounced filesystem watching, session restore, conflict handling, a virtualized code editor, syntax highlighting, regex find/replace, encoding detection, mixed-EOL preservation, background large-file loading, atomic saves, and English/Spanish UI. Markdown files open in a token-styled reading view with GFM tables, tasks, footnotes, alerts, safe HTML, internal links, source mapping, and Edit/Reading modes. Split view keeps the editor and preview synchronized with block anchors. The reading view embeds local and cached remote images, static and animated SVG, LaTeX math, and native Mermaid diagrams, with a zoomable lightbox. The shell uses a desktop title bar with File/Edit/View/Go/Help menus, document history, document/workspace search, animated document tabs with a View/Editor/Split control, and a VS Code Material Icon Theme workspace tree; settings and prompts are modal dialogs. The Markdown preview is block-virtualized for large documents. Nine themes, settings tabs, command palette, workspace Quick Open, text scaling, rotating diagnostics capped at 1 MiB, and accessibility polish are in place. Local packaging produces AppImage, deb, RPM, portable tarballs, icons, SBOM, checksums, and an update manifest; signed Windows/macOS artifacts and hosted distribution remain release-operator steps described in [docs/release.md](docs/release.md).

## Build

Requirements:

- Rust 1.97.1 (see `rust-toolchain.toml`)
- Linux: the usual egui/winit build dependencies for X11 or Wayland
- macOS and Windows: the platform toolchains required by eframe/winit

```sh
cargo build --locked
cargo run -p mdview-app -- README.md
```

## Repository rules

Read [AGENT.md](AGENT.md) before changing code. It defines the English-only repository policy, commit format, 500-line file limit, crate boundaries, and progress-tracking rule.

## Status

The project is at the `0.14.0` pre-1.0 milestone. The implementation status, evidence, and
remaining `1.0.0` gates are maintained in [`ROADMAP.md`](ROADMAP.md); the current acceptance
workspace is [`examples/v1-workspace`](examples/v1-workspace). Hosted CI is in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) and the local gate is `cargo xtask ci`.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-file-length
cargo build --release -p mdview-app --locked
```

Packaging and update commands:

```sh
cargo xtask icons
cargo xtask package
cargo xtask release-check
cargo xtask perf
cargo xtask e2e
```

The local pipeline writes AppImage, deb, RPM, portable tarball, icons, `SHA256SUMS`, an SBOM, and
an update manifest to `dist/`. Platform signing and install checks are documented in
[`docs/release.md`](docs/release.md).

Testing and diagnostics are documented in [`docs/testing.md`](docs/testing.md) and
[`docs/logging.md`](docs/logging.md). Viewer extension points are in
[`docs/plugins.md`](docs/plugins.md). The roadmap is the source of truth for scope. The deferred
1.0 platform checks are in
[`docs/release-checks.md`](docs/release-checks.md). Open pre-1.0 findings and the UX backlog
are tracked in [`docs/pre-1.0-review.md`](docs/pre-1.0-review.md).
