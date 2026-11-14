# Deferred release checks (1.0.0)

H2 (`0.2.0`) is complete and verified locally on Linux. The macOS, Windows, and manual
screen-reader checks in this file are deliberately deferred to the `1.0.0` publication gate;
they do not block work on the remaining milestones.

Hosted CI/CD is intentionally disabled until `1.0.0`. Run these checks locally when preparing
the 1.0 release and record the results in the release checklist in
[`ROADMAP.md`](../ROADMAP.md).

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

## 1.0.0 publication gate

The 1.0.0 release can ship when:

- [ ] The four snapshot tests pass locally.
- [ ] Every macOS and Windows item above is checked on real hardware.
- [ ] The screen-reader item is checked on at least one platform, with limitations documented.
- [ ] The local format, Clippy, test, release-build, and file-length commands pass.
- [ ] No data-loss or input-corruption issue remains open.

## Packaging and updates (`0.13.0`)

Run these local checks before tagging a release:

- [ ] `cargo xtask icons` regenerates all PNG sizes, ICO, and ICNS without errors.
- [ ] `cargo xtask package` produces the tarball, `.deb`, `.rpm`, AppDir, AppImage when the tool is
      available, `SHA256SUMS`, `sbom.cdx.json`, and `latest.json`.
- [ ] `cargo xtask release-check` reports every manifest asset and input.
- [ ] `dpkg-deb -x dist/mdview_0.13.0_amd64.deb /tmp/mdview-deb` then
      `/tmp/mdview-deb/usr/bin/mdview --version` prints `0.13.0`.
- [ ] `rpm -qlp dist/mdview-0.13.0-1.x86_64.rpm` lists the binary, desktop, MIME, metainfo, and
      hicolor icons.
- [ ] `dist/MD-View-0.13.0-x86_64.AppImage --appimage-extract-and-run --version` prints `0.13.0`.
- [ ] A local HTTP server can serve `latest.json` and an asset; the update dialog finds the
      version, downloads it, and verifies SHA-256.
- [ ] From a writable copy of the AppImage, install the downloaded AppImage; the original file is
      replaced atomically, mode 0755 is kept, and **Restart now** launches the new version.
- [ ] Install a downloaded deb/rpm through the dialog; `pkexec` prompts, the package is upgraded,
      and **Restart now** relaunches the binary.
- [ ] Flatpak: `flatpak update -y` is routed and restart works.
- [ ] With a dirty tab, **Install update** is blocked with a save/discard message.
- [ ] `desktop-file-validate packaging/linux/mdview.desktop` passes on a machine with the tool.
- [ ] `appstreamcli validate packaging/linux/org.mdview.MDView.metainfo.xml` passes.
- [ ] On Windows, `build-msi.ps1` produces a per-user MSI and `.md` double-click opens the app.
- [ ] On macOS, `build-dmg.sh` produces a signed/notarized universal app and `.md` opens it.
- [ ] On Linux, install the deb/rpm/AppImage, double-click a `.md`, uninstall, and confirm the
      association and icons are removed.
- [ ] Signing credentials are supplied by the release operator; never commit them.
