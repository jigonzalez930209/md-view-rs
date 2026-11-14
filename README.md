# MD View

A native Markdown, text, and code editor for Windows, macOS, and Linux, built with Rust and egui.

The project is at the `0.10.1` workspace milestone: a native shell with document tabs, a lazy folder tree, debounced filesystem watching, session restore, conflict handling, a virtualized code editor, syntax highlighting, regex find/replace, encoding detection, mixed-EOL preservation, background large-file loading, atomic saves, and English/Spanish UI. Markdown files open in a token-styled reading view with GFM tables, tasks, footnotes, alerts, safe HTML, internal links, source mapping, and Edit/Reading modes. Split view keeps the editor and preview synchronized with block anchors. The reading view embeds local and cached remote images, static and animated SVG, LaTeX math, and native Mermaid diagrams, with a zoomable lightbox. The shell uses a compact icon header and a VS Code Material Icon Theme workspace tree. The Markdown preview is block-virtualized for large documents. Nine themes, settings, command palette, text scaling, and accessibility polish are in place. Local packaging produces AppImage, deb, RPM, portable tarballs, icons, SBOM, checksums, and an update manifest; signed Windows/macOS artifacts and hosted distribution remain release-operator steps described in [docs/release.md](docs/release.md).

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

- Milestone H0: technical spikes, not part of this initial implementation.
- Milestone H1: native shell, settings, i18n, light/dark tokens, atomic saves, native dialogs, close prompt, CLI, single-instance path forwarding.
- Milestone H2: Rope-based virtualized editor, syntax highlighting, line numbers, auto-indent/pairs, bracket matching, regex find/replace, go-to-line, encoding detection, EOL handling, soft wrap, and large-file mode.
- Milestone H3: document tabs, lazy workspace tree, `.gitignore` filtering, reactive watcher with polling fallback, clean-file reload, dirty-file conflict diff, missing-file state, sidebar file operations, multi-path opening, and session/recent persistence.
- Milestone H4: spanned CommonMark/GFM parser, reading view, safe HTML, heading anchors, internal and external links, footnotes, code highlighting, per-tab Edit/Reading modes, source map, and per-mode session scroll.
- Milestone H5: resizable horizontal/vertical split, per-file ratios, block-anchor bidirectional synchronization, echo suppression, and a scroll-sync toggle.
- Milestone H6: local/remote images, static SVG with security limits, LaTeX, Mermaid, media cache, worker isolation, lightbox, and private-network policy.
- Milestone H7: bounded SVG animation, play/pause, reduced motion, fuzz/hostile corpus tests, 50 MB scale coverage, and large-file preview refusal.
- Milestone H8: nine themes, contrast gates, settings, command palette, quick open, text scaling, pseudo-locale, and centered reading layout.
- Milestone H9: local AppImage/deb/RPM/tarball pipeline, icons, associations, SBOM, checksums, update manifest/check/download, and platform packaging sources.
- Post-0.9 workspace refresh: icon-first header, Material Icon Theme file tree, virtualized preview, high-frequency scroll hardening, and a 120 Hz frame-budget gate.
- Hosted CI/CD is intentionally deferred until `1.0.0`, after the local feature set is operational. Verify changes locally with the commands below.

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
```

The local pipeline writes AppImage, deb, RPM, portable tarball, icons, `SHA256SUMS`, an SBOM, and
an update manifest to `dist/`. Platform signing and install checks are documented in
[`docs/release.md`](docs/release.md).

The roadmap is the source of truth for scope. The deferred 1.0 platform checks are in
[`docs/release-checks.md`](docs/release-checks.md).
