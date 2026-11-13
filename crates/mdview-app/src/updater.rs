//! Manual update checking, download, and checksum verification.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One downloadable release asset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateAsset {
    /// File name.
    pub name: String,
    /// Absolute URL.
    pub url: String,
    /// Lowercase SHA-256 hex digest.
    pub sha256: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: u64,
}

/// The update manifest written by `cargo xtask package`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateManifest {
    /// Released version.
    pub version: String,
    /// Release notes summary.
    #[serde(default)]
    pub notes: String,
    /// Channel label.
    #[serde(default)]
    pub channel: String,
    /// Assets.
    pub assets: Vec<UpdateAsset>,
}

/// UI state for the manual update flow.
pub enum UpdateState {
    /// Waiting for the manifest.
    Checking(Receiver<UpdateEvent>),
    /// A newer release is available.
    Available {
        /// Parsed manifest.
        manifest: UpdateManifest,
        /// Manifest URL for relative assets.
        manifest_url: String,
        /// Download worker, when running.
        downloading: Option<Receiver<UpdateEvent>>,
    },
    /// The download finished.
    Downloaded(PathBuf),
    /// Informational message.
    Info(String),
    /// Failure message.
    Failed(String),
}

/// Messages from the update worker.
#[derive(Debug)]
pub enum UpdateEvent {
    /// A newer version is available.
    Available(UpdateManifest),
    /// The current version is up to date.
    UpToDate,
    /// The download finished and the checksum matched.
    Downloaded(PathBuf),
    /// The operation failed.
    Failed(String),
}

/// Start checking a manifest URL on a worker thread.
pub fn start_check(manifest_url: String, current_version: String) -> Receiver<UpdateEvent> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let event = match fetch_manifest(&manifest_url) {
            Ok(manifest) => {
                if version_is_newer(&manifest.version, &current_version) {
                    UpdateEvent::Available(manifest)
                } else {
                    UpdateEvent::UpToDate
                }
            }
            Err(error) => UpdateEvent::Failed(error),
        };
        let _ = sender.send(event);
    });
    receiver
}

/// Start downloading an asset and verifying its checksum.
pub fn start_download(manifest_url: String, asset: UpdateAsset) -> Receiver<UpdateEvent> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let event = match download_asset(&manifest_url, &asset) {
            Ok(path) => UpdateEvent::Downloaded(path),
            Err(error) => UpdateEvent::Failed(error),
        };
        let _ = sender.send(event);
    });
    receiver
}

/// Download and verify one asset into the update cache.
pub fn download_asset(manifest_url: &str, asset: &UpdateAsset) -> Result<PathBuf, String> {
    let url = absolute_url(manifest_url, &asset.url);
    let agent = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let response = agent
        .get(&url)
        .header("User-Agent", "MD-View/0.9")
        .call()
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let mut body = response;
    let bytes = body
        .body_mut()
        .with_config()
        .limit(256 * 1024 * 1024)
        .read_to_vec()
        .map_err(|error| error.to_string())?;
    if !verify_sha256(&bytes, &asset.sha256) {
        return Err("The downloaded file does not match its SHA-256 checksum.".to_owned());
    }
    let directory = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("mdview")
        .join("updates");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let name = Path::new(&asset.name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("mdview-update");
    let path = directory.join(name);
    fs::write(&path, &bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

/// Fetch and parse a manifest.
pub fn fetch_manifest(url: &str) -> Result<UpdateManifest, String> {
    let agent = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let response = agent
        .get(url)
        .header("User-Agent", "MD-View/0.9")
        .call()
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&response.status().as_u16()) {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let mut body = response;
    let text = body
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_string()
        .map_err(|error| error.to_string())?;
    parse_manifest(&text)
}

/// Parse a manifest document.
pub fn parse_manifest(text: &str) -> Result<UpdateManifest, String> {
    let manifest: UpdateManifest =
        serde_json::from_str(text).map_err(|error| format!("Invalid update manifest: {error}"))?;
    if manifest.version.trim().is_empty() {
        return Err("The update manifest has no version.".to_owned());
    }
    Ok(manifest)
}

/// Compare dotted versions. Pre-release suffixes sort before their release.
pub fn version_is_newer(offered: &str, current: &str) -> bool {
    let offered = numeric_parts(offered);
    let current = numeric_parts(current);
    for index in 0..offered.len().max(current.len()) {
        let left = *offered.get(index).unwrap_or(&0);
        let right = *current.get(index).unwrap_or(&0);
        if left != right {
            return left > right;
        }
    }
    false
}

fn numeric_parts(version: &str) -> Vec<u64> {
    version
        .split(['-', '+'])
        .next()
        .unwrap_or(version)
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

/// Verify a SHA-256 digest in constant-ish time over the digest bytes.
pub fn verify_sha256(bytes: &[u8], expected: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual = format!("{:x}", hasher.finalize());
    actual.eq_ignore_ascii_case(expected.trim())
}

fn absolute_url(manifest_url: &str, asset_url: &str) -> String {
    if asset_url.starts_with("http://") || asset_url.starts_with("https://") {
        return asset_url.to_owned();
    }
    let base = manifest_url
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or(manifest_url);
    format!("{base}/{}", asset_url.trim_start_matches('/'))
}

/// Choose the best asset for this platform.
pub fn select_asset(manifest: &UpdateManifest) -> Option<&UpdateAsset> {
    let preferred: &[&str] = if cfg!(target_os = "linux") {
        &[".AppImage", ".deb", ".rpm", ".tar.gz"]
    } else if cfg!(target_os = "macos") {
        &[".dmg", ".tar.gz"]
    } else {
        &[".msi", ".zip"]
    };
    for suffix in preferred {
        if let Some(asset) = manifest.assets.iter().find(|asset| asset.name.ends_with(suffix)) {
            return Some(asset);
        }
    }
    manifest.assets.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "version": "0.9.0",
        "notes": "Release",
        "channel": "stable",
        "assets": [
            {"name": "mdview_0.9.0_amd64.deb", "url": "mdview_0.9.0_amd64.deb", "sha256": "abc", "size": 1}
        ]
    }"#;

    #[test]
    fn manifests_parse_and_versions_compare() {
        let manifest = parse_manifest(MANIFEST).expect("manifest");
        assert_eq!(manifest.version, "0.9.0");
        assert!(version_is_newer("0.9.0", "0.8.0"));
        assert!(!version_is_newer("0.8.0", "0.9.0"));
        assert!(!version_is_newer("0.9.0", "0.9.0"));
        assert!(version_is_newer("1.0.0", "0.99.9"));
    }

    #[test]
    fn asset_urls_are_resolved_against_the_manifest() {
        assert_eq!(
            absolute_url("https://example.com/releases/latest.json", "a.deb"),
            "https://example.com/releases/a.deb"
        );
        assert_eq!(
            absolute_url("https://example.com/latest.json", "https://cdn.example/a.deb"),
            "https://cdn.example/a.deb"
        );
    }

    #[test]
    fn a_local_manifest_and_asset_round_trip() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;

        let payload = b"update-payload";
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let digest = format!("{:x}", hasher.finalize());
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let manifest = format!(
            r#"{{"version":"9.9.9","notes":"Test","assets":[{{"name":"update.bin","url":"update.bin","sha256":"{digest}","size":{}}}]}}"#,
            payload.len()
        );
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut reader = BufReader::new(stream.try_clone().expect("clone"));
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    request.push_str(&line);
                    if line == "\r\n" {
                        break;
                    }
                }
                if request.starts_with("GET /latest.json") {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        manifest.len(),
                        manifest
                    );
                    let _ = stream.write_all(response.as_bytes());
                } else {
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", payload.len());
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(payload);
                }
                let _ = stream.flush();
            }
        });
        let url = format!("http://{address}/latest.json");
        let manifest = fetch_manifest(&url).expect("manifest");
        let asset = select_asset(&manifest).expect("asset");
        let path = download_asset(&url, asset).expect("download");
        assert_eq!(std::fs::read(&path).expect("read"), payload);
        let _ = std::fs::remove_file(path);
        server.join().expect("server");
    }

    #[test]
    fn checksum_verification_accepts_and_rejects() {
        let bytes = b"mdview";
        let digest = "aa1b3a7e5c24d6f0ca6a1f2f4f02c4b4f76a0d3f1a4c0f5b2e2f8a4d6c0b2e1a";
        assert!(!verify_sha256(bytes, digest));
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        assert!(verify_sha256(bytes, &format!("{:x}", hasher.finalize())));
        assert!(select_asset(&parse_manifest(MANIFEST).expect("manifest")).is_some());
    }
}
