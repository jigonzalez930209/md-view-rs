# End-to-end testing

`cargo xtask e2e` runs the interaction suite against a real binary in an isolated Xvfb display.

## Requirements

- Linux with `Xvfb`, `ffmpeg`, `curl`, and `python3`
- `libX11.so.6` and `libXtst.so.6` for the XTest driver
- no window manager is required

## What it covers

- Startup with a workspace and a Markdown document; diagnostics log creation.
- Every title-bar menu (`File`, `Edit`, `View`, `Go`, `Help`).
- Settings: all six tabs, every switch on and off, the numeric steppers, Done, close, Escape,
  and persisted configuration values (including editor style and reading typography).
- Workbench layout: explorer under the menu bar, tabs confined to the editor column, and the
  new tab surfaces.
- Explorer: five nested folders expanded, a root file touched so the debounced watcher rescans
  the tree, then the fifth-level file opened; context menu, open a file, rename through the
  palette command, delete with cancel and confirmation, new-file Escape/Cancel/close/Create,
  new folder, collapse all, refresh.
- Quick Open across indexed workspace files, command palette filtering and activation.
- Tabs: select, close, close other tabs, new tab, overflow menu, wheel scrolling, and the
  View/Editor/Split segmented control with disabled modes on image documents.
- Editor: edit and save, floating find/replace overlay, go-to-line, indent guides, whitespace,
  and scroll-beyond-last-line end stability.
- Split mode and 120 wheel events across both panes, then a 200-event end-of-document run
  with stable editor offsets and the last preview block visible.
- Rendering: inline and display LaTeX, tables with a clickable cell link, local and HTTP
  PNG/SVG, animated SVG, two Mermaid diagrams, syntax-highlighted Rust code, and task
  checkboxes.
- Standalone image documents: open `logo.svg` and `photo.png` from Quick Open and verify the
  image viewer, zoom, pan, lightbox, and `viewer open` diagnostics.
- Reading: select all three margin presets, increment and decrement the body-size stepper,
  and verify the persisted values.
- Search: workspace search, result activation, back navigation, and the document search
  popover; the find overlay is captured separately.
- Conflict: external write while dirty, banner capture, and reload through `Alt+R`.
- Spanish, pseudo-locale, and a dark theme captured from the settings dialog.
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
release binary packages the same behavior. The `examples/v1-workspace` fixture is the manual
counterpart and is also asserted by `cargo test -p mdview-workspace --lib`; see
[`docs/feature-matrix.md`](feature-matrix.md).

## Snapshots and unit tests

Visual snapshots live next to the crates:

- `crates/mdview-ui/tests/snapshots/` — title bar at 1×/2×, search popover, settings, palette, prompts
- `crates/mdview-app/tests/snapshots/` — sidebar and tab strip at 1×/2×, conflict view
- `crates/mdview-render/tests/snapshots/` — preview, media, split, and themes

Run `UPDATE_SNAPSHOTS=1 cargo test --workspace` to refresh them after intentional UI changes.

## Disk-backed temporary storage

Doctests and test binaries link a large media/font stack. If the system temporary filesystem is
small, point the build at a disk-backed directory:

```sh
TMPDIR=$PWD/target/tmp cargo test --workspace
```

`cargo xtask ci` sets that disk-backed `TMPDIR` automatically before running the local gate.
