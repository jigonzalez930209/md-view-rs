//! Local packaging pipeline.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::icons;

/// Package version from the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Debian sorts `~` before the empty string, so `1.0.0~rc.0` upgrades cleanly to `1.0.0`.
fn debian_version() -> String {
    VERSION.replace('-', "~")
}

/// RPM forbids `-` in `Version`; `~` has the same pre-release ordering semantics.
fn rpm_version() -> String {
    VERSION.replace('-', "~")
}

/// Build the Linux package set and update manifest.
pub fn run(no_build: bool) -> Result<(), String> {
    icons::generate()?;
    if !no_build {
        run_command("cargo", &["build", "--release", "--locked", "-p", "mdview-app"])?;
    }
    let binary = PathBuf::from("target/release/mdview");
    if !binary.is_file() {
        return Err("target/release/mdview is missing; run without --no-build".to_owned());
    }
    let arch = std::env::consts::ARCH;
    let stage = PathBuf::from(format!("dist/mdview-{VERSION}-linux-{arch}"));
    let _ = fs::remove_dir_all(&stage);
    fs::create_dir_all(&stage).map_err(|error| error.to_string())?;
    install_tree(&stage, &binary)?;
    icons::install(&stage)?;
    let tarball = format!("mdview-{VERSION}-linux-{arch}.tar.gz");
    tar_gz(&stage, &PathBuf::from("dist").join(&tarball))?;

    let deb = format!("mdview_{}_amd64.deb", debian_version());
    let deb_path = debian(&stage, Path::new("dist"), &deb, arch)?;
    let rpm_name = format!("mdview-{}-1.{}.rpm", rpm_version(), rpm_arch(arch));
    let rpm_path = rpm(&stage, Path::new("dist"), arch)?;
    let appimage = appdir(&stage, arch)?;
    let mut artifacts = vec![
        (
            tarball,
            PathBuf::from("dist").join(format!("mdview-{VERSION}-linux-{arch}.tar.gz")),
        ),
        (deb, deb_path),
        (rpm_name, rpm_path),
    ];
    if let Some(path) = appimage {
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        artifacts.push((name, path));
    }
    checksums(&artifacts)?;
    sbom(Path::new("dist/sbom.cdx.json"))?;
    manifest(&artifacts, Path::new("dist/latest.json"))?;
    println!("packages written to dist/");
    Ok(())
}

fn install_tree(stage: &Path, binary: &Path) -> Result<(), String> {
    let bin = stage.join("usr/bin");
    fs::create_dir_all(&bin).map_err(|error| error.to_string())?;
    fs::copy(binary, bin.join("mdview")).map_err(|error| error.to_string())?;
    let applications = stage.join("usr/share/applications");
    let mime = stage.join("usr/share/mime/packages");
    let metainfo = stage.join("usr/share/metainfo");
    let doc = stage.join("usr/share/doc/mdview");
    for directory in [&applications, &mime, &metainfo, &doc] {
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    }
    fs::copy("packaging/linux/mdview.desktop", applications.join("mdview.desktop"))
        .map_err(|error| error.to_string())?;
    fs::copy("packaging/linux/mdview.xml", mime.join("mdview.xml")).map_err(|error| error.to_string())?;
    fs::copy(
        "packaging/linux/org.mdview.MDView.metainfo.xml",
        metainfo.join("org.mdview.MDView.metainfo.xml"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy("README.md", doc.join("README.md")).map_err(|error| error.to_string())?;
    fs::copy("LICENSE-MIT", doc.join("LICENSE-MIT")).map_err(|error| error.to_string())?;
    fs::copy("LICENSE-APACHE", doc.join("LICENSE-APACHE")).map_err(|error| error.to_string())?;
    fs::copy("docs/media.md", doc.join("media.md")).map_err(|error| error.to_string())?;
    Ok(())
}

fn tar_gz(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = fs::File::create(destination).map_err(|error| error.to_string())?;
    let encoder = GzEncoder::new(file, Compression::best());
    let mut builder = tar::Builder::new(encoder);
    builder
        .append_dir_all(source.file_name().unwrap_or_default(), source)
        .map_err(|error| error.to_string())?;
    builder.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn debian(stage: &Path, dist: &Path, name: &str, arch: &str) -> Result<PathBuf, String> {
    let root = dist.join("deb-root");
    let _ = fs::remove_dir_all(&root);
    copy_tree(&stage.join("usr"), &root.join("usr"))?;
    let control = root.join("DEBIAN");
    fs::create_dir_all(&control).map_err(|error| error.to_string())?;
    let version = debian_version();
    fs::write(
        control.join("control"),
        format!(
            "Package: mdview\nVersion: {version}\nSection: editors\nPriority: optional\nArchitecture: {}\nMaintainer: MD View <mdview@localhost>\nInstalled-Size: {}\nDepends: libc6, libx11-6, libxkbcommon0, libwayland-client0, libgl1\nDescription: Native Markdown, text, and code editor\n MD View opens folders as workspaces, edits Markdown in tabs, and renders\n CommonMark and GFM with images, SVG, math, and Mermaid without a webview.\n",
            if arch == "aarch64" { "arm64" } else { "amd64" },
            tree_size(&root)? / 1024
        ),
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        control.join("postinst"),
        "#!/bin/sh\nset -e\ncommand -v update-mime-database >/dev/null && update-mime-database /usr/share/mime || true\ncommand -v update-desktop-database >/dev/null && update-desktop-database /usr/share/applications || true\nexit 0\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        control.join("postrm"),
        "#!/bin/sh\nset -e\ncommand -v update-mime-database >/dev/null && update-mime-database /usr/share/mime || true\nexit 0\n",
    )
    .map_err(|error| error.to_string())?;
    make_executable(&control.join("postinst"))?;
    make_executable(&control.join("postrm"))?;
    let output = dist.join(name);
    let status = Command::new("dpkg-deb")
        .args(["--build", "--root-owner-group"])
        .arg(&root)
        .arg(&output)
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err("dpkg-deb failed".to_owned());
    }
    Ok(output)
}

fn rpm(stage: &Path, dist: &Path, arch: &str) -> Result<PathBuf, String> {
    let top = dist.join("rpm-top");
    let _ = fs::remove_dir_all(&top);
    for directory in ["BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS", "rpmdb"] {
        fs::create_dir_all(top.join(directory)).map_err(|error| error.to_string())?;
    }
    let top = fs::canonicalize(&top).map_err(|error| error.to_string())?;
    let spec = top.join("SPECS/mdview.spec");
    let stage_absolute = fs::canonicalize(stage).map_err(|error| error.to_string())?;
    let version = rpm_version();
    fs::write(
        &spec,
        format!(
            "Name: mdview\nVersion: {version}\nRelease: 1\nSummary: Native Markdown, text, and code editor\nLicense: MIT OR Apache-2.0\nBuildArch: {}\n\n%description\nMD View opens folders as workspaces, edits Markdown in tabs, and renders CommonMark and GFM without a webview.\n\n%install\nmkdir -p %{{buildroot}}\ncp -a {}/usr %{{buildroot}}/\n\n%post\ncommand -v update-mime-database >/dev/null && update-mime-database /usr/share/mime || true\n\n%postun\ncommand -v update-mime-database >/dev/null && update-mime-database /usr/share/mime || true\n\n%files\n/usr/bin/mdview\n/usr/share/applications/mdview.desktop\n/usr/share/mime/packages/mdview.xml\n/usr/share/metainfo/org.mdview.MDView.metainfo.xml\n/usr/share/icons/hicolor/*/apps/mdview.png\n/usr/share/doc/mdview/*\n",
            rpm_arch(arch),
            stage_absolute.display()
        ),
    )
    .map_err(|error| error.to_string())?;
    let status = Command::new("rpmbuild")
        .args(["-bb", "--define"])
        .arg(format!("_topdir {}", top.display()))
        .args(["--define"])
        .arg(format!("_dbpath {}", top.join("rpmdb").display()))
        .arg(&spec)
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err("rpmbuild failed".to_owned());
    }
    let directory = top.join("RPMS").join(rpm_arch(arch));
    let entry = fs::read_dir(&directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .find(|entry| entry.path().extension().is_some_and(|extension| extension == "rpm"))
        .ok_or_else(|| "rpmbuild produced no rpm".to_owned())?;
    let output = dist.join(entry.file_name());
    fs::copy(entry.path(), &output).map_err(|error| error.to_string())?;
    Ok(output)
}

fn appdir(stage: &Path, arch: &str) -> Result<Option<PathBuf>, String> {
    let appdir = PathBuf::from(format!("dist/MD-View-{VERSION}-{arch}.AppDir"));
    let _ = fs::remove_dir_all(&appdir);
    copy_tree(&stage.join("usr"), &appdir.join("usr"))?;
    fs::write(
        appdir.join("AppRun"),
        "#!/bin/sh\nHERE=$(dirname \"$(readlink -f \"$0\")\")\nexec \"$HERE/usr/bin/mdview\" \"$@\"\n",
    )
    .map_err(|error| error.to_string())?;
    make_executable(&appdir.join("AppRun"))?;
    fs::write(
        appdir.join("mdview.desktop"),
        fs::read_to_string("packaging/linux/mdview.desktop").map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::copy("assets/icons/256.png", appdir.join("mdview.png")).map_err(|error| error.to_string())?;
    let output = PathBuf::from(format!("dist/MD-View-{VERSION}-{arch}.AppImage"));
    if let Some(tool) = which("appimagetool") {
        let status = Command::new(tool)
            .env("ARCH", arch)
            .arg(&appdir)
            .arg(&output)
            .status()
            .map_err(|error| error.to_string())?;
        if status.success() {
            println!("built {}", output.display());
            return Ok(Some(output));
        }
    }
    if output.is_file() {
        println!("reusing existing {}", output.display());
        Ok(Some(output))
    } else {
        println!("appimagetool not found; AppDir written, AppImage deferred");
        Ok(None)
    }
}

fn manifest(artifacts: &[(String, PathBuf)], destination: &Path) -> Result<(), String> {
    let base = std::env::var("MDVIEW_UPDATE_BASE_URL")
        .unwrap_or_else(|_| "https://example.invalid/mdview/releases".to_owned());
    let assets = artifacts
        .iter()
        .map(|(name, path)| {
            Ok(json!({
                "name": name,
                "url": format!("{base}/{name}"),
                "sha256": sha256(path)?,
                "size": fs::metadata(path).map_err(|error| error.to_string())?.len(),
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let document = json!({
        "version": VERSION,
        "channel": "stable",
        "notes": "See CHANGELOG.md",
        "assets": assets,
    });
    fs::write(
        destination,
        serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn checksums(artifacts: &[(String, PathBuf)]) -> Result<(), String> {
    let mut lines = String::new();
    for (_, path) in artifacts {
        lines.push_str(&format!(
            "{}  {}\n",
            sha256(path)?,
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    fs::write("dist/SHA256SUMS", lines).map_err(|error| error.to_string())
}

/// Write a CycloneDX SBOM for the workspace.
pub fn sbom(destination: &Path) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("cargo metadata failed".to_owned());
    }
    let metadata: Value = serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let mut components = Vec::new();
    if let Some(packages) = metadata.get("packages").and_then(Value::as_array) {
        for package in packages {
            components.push(json!({
                "type": "library",
                "name": package.get("name").cloned().unwrap_or(Value::Null),
                "version": package.get("version").cloned().unwrap_or(Value::Null),
                "licenses": package.get("license").and_then(Value::as_str).map(|license| vec![json!({"license": {"id": license}})]).unwrap_or_default(),
            }));
        }
    }
    let document = json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {"type": "application", "name": "mdview", "version": VERSION}
        },
        "components": components,
    });
    fs::write(
        destination,
        serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn tree_size(path: &Path) -> Result<u64, String> {
    let mut size = 0;
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            size += tree_size(&entry.path())?;
        } else {
            size += metadata.len();
        }
    }
    Ok(size)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let target = destination.join(entry.file_name());
        if entry.metadata().map_err(|error| error.to_string())?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).map_err(|error| error.to_string())?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn rpm_arch(arch: &str) -> &str {
    match arch {
        "aarch64" => "aarch64",
        _ => "x86_64",
    }
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(name))
            .find(|path| path.is_file())
    })
}

fn run_command(command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} failed"))
    }
}
