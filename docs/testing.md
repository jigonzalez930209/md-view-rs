# End-to-end testing

`cargo xtask e2e` runs the interaction suite against a real binary in an isolated Xvfb display.

## Requirements

- Linux with `Xvfb`, `ffmpeg`, `curl`, and `python3`
- `libX11.so.6` and `libXtst.so.6` for the XTest driver
- no window manager is required

## What it covers

- Startup with a workspace and a Markdown document; diagnostics log creation.
- Every title-bar menu (`File`, `Edit`, `View`, `Go`, `Help`).
- Settings: all five tabs, every switch on and off, the numeric text-scale stepper, Done,
  close, Escape, and persisted configuration values.
- Explorer: expand, context menu, open a file, rename through the palette command, delete with
  cancel and confirmation, new-file Escape/Cancel/close/Create, new folder, collapse all, refresh.
- Quick Open across indexed workspace files, command palette filtering and activation.
- Tabs: select, close, close other tabs, new tab, overflow menu.
- Editor: edit and save, find, replace bar, go-to-line with the up/down arrows.
- Split mode and 120 wheel events across both panes.
- Rendering: LaTeX, tables, local and HTTP PNG/SVG, animated SVG, two Mermaid diagrams,
  code blocks, and task checkboxes.
- Shutdown through the custom window close button.
- Diagnostics: log file present, event coverage, remote media sources, no panics, no media
  failures, final 120 Hz windows with zero frames over 8.33 ms, and total log size ≤ 1 MiB.

Screenshots are written to `target/e2e/`. Logs and the disposable workspace stay in the
temporary directory printed on success or failure.

## Profiles

```sh
cargo xtask e2e
MDVIEW_E2E_PROFILE=release cargo xtask e2e
```

The release profile is the authoritative interaction/performance run. It also proves that the
release binary packages the same behavior.

## Snapshots and unit tests

Visual snapshots live next to the crates:

- `crates/mdview-ui/tests/snapshots/` — title bar, activity rail, settings, palette, prompts
- `crates/mdview-app/tests/snapshots/` — sidebar, tab strip, conflict view
- `crates/mdview-render/tests/snapshots/` — preview, media, split, and themes

Run `UPDATE_SNAPSHOTS=1 cargo test --workspace` to refresh them after intentional UI changes.
