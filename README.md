# MD View

A native Markdown, text, and code editor for Windows, macOS, and Linux, built with Rust and egui.

The project is at the `0.1.0` foundation milestone: a native shell with a single-document text editor, settings, English/Spanish UI, light/dark themes, atomic saves, and native open/save dialogs. The Markdown editor, workspace tree, tabs, and rich rendering arrive in later milestones described in [ROADMAP.md](ROADMAP.md).

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
- Hosted CI/CD is intentionally deferred until `1.0.0`, after the local feature set is operational. Verify changes locally with the commands below.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-file-length
cargo build --release -p mdview-app --locked
```

The roadmap is the source of truth for scope.
