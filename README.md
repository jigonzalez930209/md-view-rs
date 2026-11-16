# MD View

A native Markdown, text, and code editor for Windows, macOS, and Linux, built with Rust and egui.
No webview, no browser engine, and no JavaScript runtime.

[![CI](https://github.com/jigonzalez930209/md-view-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/jigonzalez930209/md-view-rs/actions/workflows/ci.yml)
[![Docs](https://github.com/jigonzalez930209/md-view-rs/actions/workflows/docs.yml/badge.svg)](https://jigonzalez930209.github.io/md-view-rs/)
[![Release](https://img.shields.io/github/v/release/jigonzalez930209/md-view-rs?include_prereleases)](https://github.com/jigonzalez930209/md-view-rs/releases)

**Documentation:** <https://jigonzalez930209.github.io/md-view-rs/>

## What it is

MD View opens a folder as a workspace and a tab per document. It renders CommonMark and GFM itself:
GitHub-style tables, tasks, alerts, footnotes, safe HTML, syntax-colored code, LaTeX math, Mermaid
diagrams, local and cached remote images, and animated SVG. The editor is virtualized and uses
syntect with Oniguruma, regex find/replace, go-to-line, encoding and EOL detection, mixed-EOL
preservation, and a large-file mode.

The explorer follows the active tab, expands its folders, and marks the file in the tree. Documents
opened outside the workspace appear in a fixed **External files** group. View, Edit, and Split modes
share a block-accurate source map; split view scrolls and zooms both panes in exact, flicker-free
sync, and nine themes cover light, dark, and system-follow modes.

The 1.0.0-rc.1 release candidate ships Linux packages (AppImage, deb, RPM, tarball), macOS DMG/zip,
and Windows MSI/zip. macOS and Windows RC builds are unsigned; signing and notarization remain the
final 1.0.0 release-operator steps. See [`docs/release.md`](docs/release.md).

## Build

Requirements:

- Rust 1.97.1 (see `rust-toolchain.toml`)
- Linux: the usual egui/winit build dependencies for X11 or Wayland
- macOS and Windows: the platform toolchains required by eframe/winit
- A C compiler for the Oniguruma syntax engine

```sh
cargo build --locked
cargo run -p mdview-app -- README.md
```

Use `cargo run --release -p mdview-app -- README.md` when evaluating typing,
syntax-highlighting, or preview performance. Debug builds are unoptimized and are not a
representative measure of editor latency.

The acceptance workspace and feature map live in
[`examples/v1-workspace`](examples/v1-workspace) and [`docs/feature-matrix.md`](docs/feature-matrix.md).

## Documentation site

The VitePress site is in [`website/`](website) and is deployed to GitHub Pages by
`.github/workflows/docs.yml`.

```sh
cd website
npm ci
npm run dev
npm run build
```

## Repository rules

Read [AGENT.md](AGENT.md) before changing code. It defines the English-only repository policy, commit
format, 500-line file limit, crate boundaries, and progress-tracking rule.

## Status

The project is at the `1.0.0-rc.1` release candidate. Implementation status, evidence, and the
remaining `1.0.0` gates are maintained in [`ROADMAP.md`](ROADMAP.md). Hosted CI is in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) and the local gate is `cargo xtask ci`.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-file-length
cargo build --release -p mdview-app --locked
```

## License

MIT or Apache-2.0, at your option.
