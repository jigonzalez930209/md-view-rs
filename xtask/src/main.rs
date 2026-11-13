//! Repository automation for MD View.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MAX_LINES: usize = 500;

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("check-file-length") => check_file_length(),
        Some("help") | None => {
            println!("Usage: cargo xtask <command>");
            println!();
            println!("Commands:");
            println!("  check-file-length  Fail when a hand-written Rust file exceeds 500 lines");
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
