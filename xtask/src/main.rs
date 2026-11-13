//! Repository automation for MD View.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod icons;
mod package;
mod release;

const MAX_LINES: usize = 500;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str);
    match command {
        Some("check-file-length") => check_file_length(),
        Some("package") => match package::run(args.iter().any(|arg| arg == "--no-build")) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("package: {error}");
                ExitCode::FAILURE
            }
        },
        Some("release-check") => match release::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("release-check: {error}");
                ExitCode::FAILURE
            }
        },
        Some("sbom") => match package::sbom(Path::new("dist/sbom.cdx.json")) {
            Ok(()) => {
                println!("wrote dist/sbom.cdx.json");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("sbom: {error}");
                ExitCode::FAILURE
            }
        },
        Some("icons") => match icons::generate() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("icons: {error}");
                ExitCode::FAILURE
            }
        },
        Some("help") | None => {
            println!("Usage: cargo xtask <command>");
            println!();
            println!("Commands:");
            println!("  check-file-length  Fail when a hand-written Rust file exceeds 500 lines");
            println!("  icons              Generate PNG, ICO, and ICNS assets from assets/icon.svg");
            println!("  sbom               Write a CycloneDX SBOM to dist/sbom.cdx.json");
            println!("  package            Build release artifacts, checksums, SBOM, and update manifest");
            println!("  release-check      Verify artifacts, checksums, and release inputs");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown xtask command: {other}");
            ExitCode::from(2)
        }
    }
}

fn check_file_length() -> ExitCode {
    let mut files = Vec::new();
    collect_rust_files(Path::new("crates"), &mut files);
    collect_rust_files(Path::new("xtask"), &mut files);

    let mut failures = Vec::new();
    for path in files {
        match count_lines(&path) {
            Ok(lines) if lines > MAX_LINES => failures.push((path, lines)),
            Ok(_) => {}
            Err(error) => {
                eprintln!("could not read {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    if failures.is_empty() {
        println!("file length check passed (limit: {MAX_LINES} lines)");
        ExitCode::SUCCESS
    } else {
        for (path, lines) in failures {
            eprintln!("{}: {lines} lines (limit: {MAX_LINES})", path.display());
        }
        ExitCode::FAILURE
    }
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

fn count_lines(path: &Path) -> io::Result<usize> {
    let text = fs::read_to_string(path)?;
    Ok(text.lines().count())
}
