# Manual editor checks

H2 (`0.2.0`) is implemented and verified locally on Linux. The remaining H2 gates are manual
checks on macOS and Windows and a screen-reader pass. This file is the checklist to run before
treating H2 as fully complete.

Hosted CI/CD is intentionally disabled until `1.0.0`. Run these checks locally and record the
result in the H2 progress table in [`ROADMAP.md`](../ROADMAP.md).

## Visual snapshots

The editor has four deterministic snapshots:

- light, no wrap
- light, wrap
- dark, no wrap
- dark, wrap

Run them locally:

```sh
cargo test -p mdview-editor --test editor_snapshots
```

The images live in `crates/mdview-editor/tests/snapshots/`. To regenerate after a deliberate
visual change:

```sh
UPDATE_SNAPSHOTS=1 cargo test -p mdview-editor --test editor_snapshots
```

Review every changed PNG before committing it. Do not raise `max_failed_pixels` to hide a
one-pixel layout regression.

The harness uses egui's bundled default fonts and does not install system fonts, so the images
are reproducible on a machine with the same renderer. If a platform renderer produces a small
anti-aliasing difference, record it in the snapshot options rather than changing the editor.

## macOS

Run on both Apple Silicon and Intel (or a universal build on one machine and an Intel build in a
VM):

- [ ] Open a Rust file and a Markdown file with spaces and accents in the path.
- [ ] Cmd+O, Cmd+S, Cmd+Shift+S, Cmd+N, Cmd+Q and Cmd+F behave correctly.
- [ ] The unsaved-changes prompt appears for Cmd+Q and the window close button.
- [ ] IME: commit Japanese and Simplified Chinese text; verify the cursor advances by characters,
      not bytes, and Backspace removes the committed character.
- [ ] IME: use `DeleteSurrounding` during composition and verify the intended range is deleted.
- [ ] Trackpad scroll, horizontal scroll, and two-finger selection behave correctly.
- [ ] Retina/2x scaling: gutter, text, selection, cursor, and wrapped rows align.
- [ ] Open the 10.5 MB / 220k-line fixture; the loading dialog shows progress, can be cancelled,
      and the editor enters large-file mode after completion.
- [ ] Save after editing the large fixture and confirm the file is readable by another editor.
- [ ] VoiceOver: focus the editor and verify it reports a text-input role, its value, and selection
      changes; document any limitation.

## Windows

Run on Windows 10 and Windows 11 x64:

- [ ] Open a Rust file and a Markdown file with spaces and accents in the path.
- [ ] Ctrl+O, Ctrl+S, Ctrl+Shift+S, Ctrl+N, Ctrl+Q and Ctrl+F behave correctly.
- [ ] AltGr combinations used by a Spanish keyboard do not trigger editor shortcuts.
- [ ] The unsaved-changes prompt appears for Ctrl+Q and the window close button.
- [ ] IME: commit Japanese and Simplified Chinese text; verify cursor movement and Backspace.
- [ ] IME: use `DeleteSurrounding` during composition and verify the intended range is deleted.
- [ ] 100%, 150%, and 200% display scaling: gutter, text, selection, and wrapped rows align.
- [ ] Open the 10.5 MB / 220k-line fixture; loading progress and cancellation work.
- [ ] Save after editing the large fixture and confirm the file is readable by another editor.
- [ ] NVDA: focus the editor and verify text-input role, value, and selection announcements;
      document any limitation.

## Linux regression checks

Linux has already been exercised, but repeat these after any dependency upgrade:

- [ ] X11 and Wayland sessions.
- [ ] IME: Japanese and Simplified Chinese commit and surrounding deletion.
- [ ] Orca: editor role, value, and selection announcements.
- [ ] Open the 10.5 MB / 220k-line fixture; load, cancel, edit, and save.
- [ ] Mixed CRLF/LF/CR file round-trips unchanged when saved without edits.

## H2 exit gate

H2 can be marked fully `DONE` when:

- [ ] The four snapshot tests pass locally.
- [ ] Every macOS and Windows item above is checked on real hardware.
- [ ] The screen-reader item is checked on at least one platform, with limitations documented.
- [ ] The local format, Clippy, test, release-build, and file-length commands pass.
- [ ] No data-loss or input-corruption issue remains open.
