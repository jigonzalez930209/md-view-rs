//! Release verification.

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Verify the generated artifacts against the update manifest and checksums.
pub fn run() -> Result<(), String> {
    let status = Command::new("cargo")
        .args(["deny", "check"])
        .status()
        .map_err(|error| format!("could not run cargo deny: {error}"))?;
    if !status.success() {
        return Err("cargo deny check failed".to_owned());
    }
    let version = env!("CARGO_PKG_VERSION");
    let manifest = Path::new("dist/latest.json");
    let text = fs::read_to_string(manifest)
        .map_err(|error| format!("dist/latest.json: {error}; run `cargo xtask package`"))?;
    let document: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let manifest_version = document.get("version").and_then(Value::as_str).unwrap_or_default();
    if manifest_version != version {
        return Err(format!(
            "manifest version {manifest_version} does not match workspace {version}"
        ));
    }
    let mut failures = Vec::new();
    let mut checked = 0usize;
    if let Some(assets) = document.get("assets").and_then(Value::as_array) {
        for asset in assets {
            let Some(name) = asset.get("name").and_then(Value::as_str) else {
                failures.push("asset without a name".to_owned());
                continue;
            };
            let path = Path::new("dist").join(name);
            if !path.is_file() {
                failures.push(format!("missing {}", path.display()));
                continue;
            }
            let expected = asset.get("sha256").and_then(Value::as_str).unwrap_or_default();
            let actual = sha256(&path)?;
            if expected != actual {
                failures.push(format!("checksum mismatch for {name}"));
            }
            checked += 1;
        }
    }
    // Release packages ship their own FFmpeg for the video viewer.
    let stage = format!("dist/mdview-{version}-linux-{}/usr/lib/mdview", std::env::consts::ARCH);
    for tool in ["ffmpeg", "ffprobe"] {
        let path = Path::new(&stage).join(tool);
        if !path.is_file() {
            failures.push(format!(
                "missing {}; build it with scripts/ffmpeg/build.sh and package with MDVIEW_FFMPEG_DIR",
                path.display()
            ));
        }
    }
    let lock = fs::read_to_string("Cargo.lock").map_err(|error| error.to_string())?;
    let allowed_git = [
        "https://github.com/storytold/craft-libs",
        "https://github.com/storytold/egui",
        "https://github.com/storytold/vectorcraft",
        "https://github.com/storytold/winit",
    ];
    for line in lock.lines().filter(|line| line.contains("source = \"git+")) {
        if !line.contains("?rev=") {
            failures.push(format!("unpinned git source: {line}"));
        }
        if !allowed_git.iter().any(|host| line.contains(host)) {
            failures.push(format!("unexpected git source: {line}"));
        }
    }
    for required in [
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "deny.toml",
        ".github/workflows/ci.yml",
        "dist/SHA256SUMS",
        "dist/sbom.cdx.json",
        "packaging/linux/mdview.desktop",
        "packaging/linux/mdview.xml",
        "packaging/flatpak/org.mdview.MDView.json",
        "packaging/macos/Info.plist",
        "packaging/windows/mdview.wxs",
        "assets/icon.svg",
        "assets/icons/mdview.ico",
        "assets/icons/mdview.icns",
    ] {
        if !Path::new(required).is_file() {
            failures.push(format!("missing {required}"));
        }
    }
    if failures.is_empty() {
        println!("release check passed: {checked} assets, version {version}");
        Ok(())
    } else {
        for failure in failures {
            eprintln!("{failure}");
        }
        Err("release check failed".to_owned())
    }
}

fn sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
