//! Local CI gate.

use std::fs;
use std::process::{Command, ExitCode};

/// Run every local gate in the same order as the README.
pub fn run() -> ExitCode {
    let steps: &[(&str, &[&str])] = &[
        ("format", &["fmt", "--all", "--", "--check"]),
        (
            "clippy",
            &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        ),
        ("tests", &["test", "--workspace", "--locked"]),
        ("deny", &["deny", "check"]),
        ("file length", &["xtask", "check-file-length"]),
        ("release build", &["build", "--release", "-p", "mdview-app", "--locked"]),
    ];
    let tmp = std::env::current_dir().unwrap_or_default().join("target/tmp");
    let _ = fs::create_dir_all(&tmp);
    for (name, args) in steps {
        println!("== ci: {name} ==");
        let status = Command::new("cargo").args(*args).env("TMPDIR", &tmp).status();
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!("ci: {name} failed with {status}");
                return ExitCode::FAILURE;
            }
            Err(error) => {
                eprintln!("ci: could not run {name}: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    println!("ci passed");
    ExitCode::SUCCESS
}
