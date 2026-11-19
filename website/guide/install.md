# Install MD View 1.0.0-rc.7

The release candidate is published on the
[GitHub release page](https://github.com/jigonzalez930209/md-view-rs/releases/tag/v1.0.0-rc.7).
Every asset is listed with its SHA-256 in `SHA256SUMS`.

::: warning Release candidate
1.0.0-rc.7 builds are unsigned. macOS Gatekeeper and Windows SmartScreen may show an extra warning.
The final 1.0.0 release will add signing and notarization once the release certificates are
available. Linux packages are ready to install as-is.
:::

## Linux

Choose one of the published assets:

| Asset | Use |
|---|---|
| `mdview-1.0.0-rc.7-linux-x86_64.tar.gz` | Portable tree; extract and run `usr/bin/mdview`. |
| `mdview_1.0.0.rc.7_amd64.deb` | Debian and Ubuntu package. |
| `mdview-1.0.0.rc.7-1.x86_64.rpm` | Fedora, openSUSE, and other RPM distributions. |
| `MD-View-1.0.0-rc.7-x86_64.AppImage` | Single-file image for most desktop distributions. |

```sh
# Debian / Ubuntu
sudo dpkg -i mdview_1.0.0.rc.7_amd64.deb

# Fedora / RPM
sudo rpm -i mdview-1.0.0.rc.7-1.x86_64.rpm

# AppImage
chmod +x MD-View-1.0.0-rc.7-x86_64.AppImage
./MD-View-1.0.0-rc.7-x86_64.AppImage
```

The deb and RPM packages install the binary, desktop entry, MIME association, AppStream metadata,
and hicolor icons. The package scripts refresh the MIME and desktop databases when those tools are
available.

## macOS

Download `MD-View-1.0.0-rc.7-macOS-arm64.dmg`, open it, and drag **MD View** to Applications. The first launch
of an unsigned RC may require **Control-click → Open** or **System Settings → Privacy & Security →
Open Anyway**.

A zipped `.app` is published alongside the DMG for automation and portable use.

## Windows

Download `MD-View-1.0.0-rc.7-windows-x86_64.msi` for the per-user install, or the `.zip` for a portable copy. The
MSI installs a Start-menu shortcut and registers Markdown associations under `HKCU`; no
administrator rights are required. SmartScreen may warn on the unsigned RC; choose **More info →
Run anyway** after verifying the checksum.

## Video playback

Videos and their sound play with nothing else to install: MD View decodes them itself with
pure-Rust decoders. It plays MP4, MOV, 3GP, MKV, WebM, AVI, MPEG-TS/PS (TS, M2TS, MTS, MPG,
VOB), and FLV files with H.264, HEVC, AV1, VP8, VP9, MPEG-1/2, MPEG-4 Part 2 (Xvid, DivX),
H.263, or Motion JPEG video and AAC, MP3, Opus, Vorbis, FLAC, AC-3, or PCM sound. Windows Media
(WMV) and Ogg video (OGV) open with a message that they are not supported.

## Verify the download

```sh
sha256sum -c SHA256SUMS
```

On Windows PowerShell:

```powershell
Get-FileHash .\MD-View-1.0.0-rc.7-windows-x86_64.msi -Algorithm SHA256
```

Compare the value with the matching line in `SHA256SUMS` from the release.

## Build from source

Requirements:

- Rust 1.97.1 (see `rust-toolchain.toml`)
- The platform dependencies for egui/winit
- A C compiler for the Oniguruma syntax engine
- Linux: the ALSA development files for video sound (`libasound2-dev` on Debian/Ubuntu,
  `alsa-lib-devel` on Fedora)

```sh
git clone https://github.com/jigonzalez930209/md-view-rs.git
cd md-view-rs
cargo build --locked --release -p mdview-app
cargo run -p mdview-app -- README.md
```

Linux packaging is produced by the repository xtask:

```sh
cargo xtask icons
cargo xtask package
cargo xtask release-check
```
