# End-to-end testing

`cargo xtask e2e` runs the interaction suite against a real binary in an isolated Xvfb display.

## Requirements

- Linux with `Xvfb`, `ffmpeg`, `curl`, and `python3`
- `libX11.so.6` and `libXtst.so.6` for the XTest driver
- no window manager is required

## Resource guard

`cargo xtask ci` refuses to start with less than 4 GiB available, sizes `CARGO_BUILD_JOBS` from the
available memory (at most 4 jobs), and stops a step when available memory drops below 2 GiB or when
full memory-stall pressure reaches 20% with less than 6 GiB available. Pressure is system-wide and
includes stalls caused by the scope's own `MemoryHigh`, so pressure with memory to spare does not
stop the gate. Each step runs in its own process group, so stopping it also stops its rustc and
linker children. The E2E run applies the same 4 GiB start check and stops
when available memory drops below 2 GiB (`MDVIEW_E2E_MIN_START_MB`,
`MDVIEW_E2E_MIN_AVAILABLE_MB`).

On Linux with systemd, a heavy gate can also run in its own transient scope. If the kernel or
systemd-oomd has to end the build, it ends that scope instead of the terminal that started it:

```sh
systemd-run --user --scope --collect cargo xtask ci
```

The E2E run can use every core while keeping memory capped and swap untouched:

```sh
systemd-run --user --scope --collect \
  -p MemoryHigh=6G -p MemoryMax=8G -p MemorySwapMax=0 \
  -p "CPUQuota=$(nproc)00%" \
  env CARGO_BUILD_JOBS=4 cargo xtask e2e
```

That is the configuration used for the 1.0.0-rc.4 E2E pass: 16 cores, 243 MiB peak app memory,
and no swap. A one-core quota also completed most stages but made the clipboard step race the
selection; the test now waits for the click selection and reports the actual clipboard contents
when it fails.

Close browsers, language servers, or virtual machines before running the full gate on a desktop
that is already using most of its memory. The guards protect the desktop; they do not make an
under-provisioned machine faster.

## What it covers

- Startup with a workspace and a Markdown document; diagnostics log creation.
- Every title-bar menu (`File`, `Edit`, `View`, `Go`, `Help`).
- Settings: all six tabs, every switch on and off, the numeric steppers, the 5–40% margin
  slider at both ends and the default, Done, close, Escape, and persisted configuration values
  (including editor style, reading margins, and equation size).
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
- Markdown: lists typed into `src/format.md` (continuation, nesting, leaving a list) and Ctrl+B,
  compared byte for byte after saving.
- Editor interactions on `src/interact.md`: double-click and Ctrl+B, triple-click, copy through the
  real X11 CLIPBOARD and paste of text owned by a separate client, regex Replace All with `$2:$1`
  through the find panel's controls, a drag across soft-wrapped
  rows, the formatting bar's table grid, **Format: Heading 2** from the palette, and a bracket
  highlight screenshot; the saved file is checked.
- Formatting bar: every button once on `src/toolbar.md` (heading menu, italic, underline, quote,
  code, link, the three lists), compared exactly after saving.
- Explorer context menu: a file renamed and deleted (with confirmation) through its menu items,
  and a new file created inside a folder from the folder's menu.
- Large-file mode: a 210,000-line file opens, typing at its end is saved, and the tab closes.
- Split mode and 120 wheel events across both panes, then a 200-event end-of-document run
  with stable editor offsets and the last preview block visible.
- Split-end hold: 400 wheel events at the editor end, asserting the editor offset is stable,
  the preview stays at its maximum, and the preview end target is written at most once.
- Split zoom: at zoom 1.3 (stacked split) and 0.9 (side by side), each pane is scrolled to the
  end; the `split settled` log must show both panes at their maximum with a lock error within
  0.005 points, and no `split overshoot` (a frame painted past a pane's end) may be logged.
- Media warm-up: local images, SVG, math, and Mermaid are resolved before the first preview
  layout; intrinsic sizes reserve stable boxes so no texture load reflows the document.
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

## Process cleanup and memory guard

`scripts/e2e/lifecycle.sh` owns the processes the run starts (the app, the web server, and
Xvfb when the run started it). They are stopped on success, failure, Ctrl+C, SIGTERM, and
SIGHUP: SIGTERM first, SIGKILL after three seconds. Each process is recorded in
`target/e2e-pids`; if a run is killed with SIGKILL, the next run stops the recorded processes
that still run the same command before it starts.

A watchdog samples the app every 200 ms. The run fails, and the app is killed, when its
resident memory exceeds `MDVIEW_E2E_MAX_RSS_MB` (default 1024 MiB; a normal run peaks near
250 MiB) or the machine has less than `MDVIEW_E2E_MIN_AVAILABLE_MB` available (default 768 MiB).
The summary prints the app's peak resident memory.

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

Split synchronization has frame-level tests in `crates/mdview-app/src/split_sync_tests.rs`. They
drive real editor and preview widgets with wheel events past the end of the document and assert
that neither pane ever moves against the wheel or past its maximum, that the follower holds still
at the end, and that a preview leader re-locks the editor after a soft-wrap re-layout. A
painted-position test reads the final paragraph's rectangle from the widget tree on every frame
and fails if it is ever painted above its resting place and snaps back; the previous line-based
sync fails it with a 334-point one-frame jump. A zoom test applies the app zoom factor the way
the window experiences it (constant pixels, so fewer or more points) at 0.8–1.6 with either pane
leading, and asserts the panes stay locked within 0.005 points and reach both ends together. A
rendered-pixel test renders every frame with wgpu while zooming at the end (soft wrap on and off,
either pane leading) and requires both panes to be pixel-identical to the settled frame from the
first frame after each zoom, and the frames to come to rest; without the discarded passes it
fails with a 65-pixel one-frame jump. Every frame test also asserts that no frame is shown painted
past a pane's end (`overshoot_paints`). The pure
offset map in `split_sync.rs` has continuity, monotonicity, round-trip, and range tests.

Editor typing and navigation have frame tests in `crates/mdview-editor/src/typing_tests.rs`.
They send real key, text, and pointer events through egui: Enter and Space after a mouse click
must not move the cursor to the pointer, typing near the viewport edge must not scroll, Enter at
the bottom scrolls at most one row, the highlight cache must show edited text, and Up/Down,
Page Up/Down, Home/End, Tab/Shift+Tab, Backspace, word deletion, Ctrl+arrows, auto-pairing,
and undo are checked against the resulting text and cursor.

`crates/mdview-editor/src/feature_tests.rs` covers double- and triple-click selection, dragging
across wrapped rows and Shift+click, copy/cut/paste with undo, find with regex, whole-word, and
case options (including line anchors and empty matches), regex Replace All with groups, large-file
mode, and bracket matching. `sidebar_tests.rs` drives the explorer with a click and the arrow
keys, Enter, and Left.

Markdown formatting has frame tests in `crates/mdview-editor/src/markdown_tests.rs`: toggling and
combining bold, italic, and underline (with and without a selection, and across lines), headings,
quotes, list conversions, list continuation and renumbering on Enter, leaving and nesting lists,
tables and Tab navigation, links, code blocks, single-step undo, and the `Ctrl/Cmd+B/I/U`
shortcuts. The E2E run types lists into `src/format.md`, bolds a word with `Ctrl+B`, saves, and
compares the file byte for byte.

## Disk-backed temporary storage

Doctests and test binaries link a large media/font stack. If the system temporary filesystem is
small, point the build at a disk-backed directory:

```sh
TMPDIR=$PWD/target/tmp cargo test --workspace
```

`cargo xtask ci` sets that disk-backed `TMPDIR` automatically before running the local gate.
