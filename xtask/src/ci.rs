//! Local CI gate.

use std::fs;
use std::path::Path;
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::Duration;
#[cfg(unix)]
use std::time::Instant;

/// Refuse to start below this much available memory. The full gate runs Clippy, every test
/// binary, and a release build with LTO; starting with less memory has frozen desktops.
const MIN_START_AVAILABLE_MB: u64 = 4096;
/// Stop a running step below this much available memory.
const MIN_STEP_AVAILABLE_MB: u64 = 2048;
/// Stop at this much full memory-stall pressure (kernel avg10) once memory is getting scarce.
///
/// Pressure is reported system-wide, and it also counts stalls from the gate's own systemd
/// scope reaching `MemoryHigh`: linking the test binaries in parallel reached 55% with 14 GiB
/// still available. Those stalls throttle only the build, and the scope's `MemoryMax` already
/// keeps it off the desktop, so pressure is a reason to stop only when memory is scarce.
const LOW_MEMORY_PRESSURE_AVG10: f64 = 20.0;
/// Below this much available memory, `LOW_MEMORY_PRESSURE_AVG10` applies.
const LOW_MEMORY_MB: u64 = 6144;
/// Memory budget used to size Cargo's parallel jobs.
const JOB_MEMORY_MB: u64 = 2048;
const MAX_JOBS: usize = 4;

/// Run every local gate in the same order as the README, without letting the build exhaust the
/// machine. Each step runs in its own process group and is stopped before systemd-oomd has to
/// kill the terminal that started it.
pub fn run() -> ExitCode {
    let available = available_memory_mb();
    let pressure = full_pressure_avg10();
    if let Some(available) = available
        && available < MIN_START_AVAILABLE_MB
    {
        eprintln!(
            "ci: refusing to start with {available} MiB available; close memory-heavy \
             applications first (need at least {MIN_START_AVAILABLE_MB} MiB)"
        );
        return ExitCode::FAILURE;
    }
    let jobs = build_jobs(available);
    if let Some(available) = available {
        println!(
            "ci: available memory {available} MiB, full pressure {:.2}%, build jobs {jobs}",
            pressure.unwrap_or(0.0)
        );
    }

    let steps: &[(&str, &[&str])] = &[
        ("format", &["fmt", "--all", "--", "--check"]),
        (
            "clippy",
            &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        ),
        ("tests", &["test", "--workspace", "--locked", "--", "--test-threads=1"]),
        ("deny", &["deny", "check"]),
        ("file length", &["xtask", "check-file-length"]),
        ("release build", &["build", "--release", "-p", "mdview-app", "--locked"]),
    ];
    let tmp = std::env::current_dir().unwrap_or_default().join("target/tmp");
    let _ = fs::create_dir_all(&tmp);
    for (name, args) in steps {
        if let Err(error) = run_step(name, args, jobs, &tmp) {
            eprintln!("ci: {error}");
            return ExitCode::FAILURE;
        }
    }
    println!("ci passed");
    ExitCode::SUCCESS
}

fn run_step(name: &str, args: &[&str], jobs: usize, tmp: &Path) -> Result<(), String> {
    println!("== ci: {name} ==");
    let mut command = Command::new("cargo");
    command
        .args(args)
        .env("TMPDIR", tmp)
        .env("CARGO_BUILD_JOBS", jobs.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not run {name}: {error}"))?;
    let pid = child.id();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("{name} failed with {status}")),
            Ok(None) => {}
            Err(error) => return Err(format!("could not wait for {name}: {error}")),
        }
        let available = available_memory_mb().unwrap_or(u64::MAX);
        let pressure = full_pressure_avg10().unwrap_or(0.0);
        if must_stop(available, pressure) {
            eprintln!("ci: stopping {name}: available memory {available} MiB, full pressure {pressure:.2}%");
            terminate(&mut child, pid);
            return Err(format!(
                "{name} was stopped to protect the desktop from running out of memory"
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Whether a running step endangers the desktop: too little memory left, or memory stalls
/// while memory is already scarce.
fn must_stop(available_mb: u64, full_pressure: f64) -> bool {
    available_mb < MIN_STEP_AVAILABLE_MB || (full_pressure >= LOW_MEMORY_PRESSURE_AVG10 && available_mb < LOW_MEMORY_MB)
}

/// Stop the whole process group so rustc and linker children cannot keep the machine under
/// pressure after Cargo is gone.
fn terminate(child: &mut Child, pid: u32) {
    #[cfg(unix)]
    {
        let group = format!("-{pid}");
        let _ = Command::new("kill").args(["-TERM", "--", &group]).status();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn build_jobs(available: Option<u64>) -> usize {
    if let Ok(value) = std::env::var("CARGO_BUILD_JOBS")
        && let Ok(jobs) = value.parse::<usize>()
        && jobs > 0
    {
        return jobs;
    }
    let cpus = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    let by_memory = available
        .map(|available| (available / JOB_MEMORY_MB).max(1) as usize)
        .unwrap_or(cpus);
    cpus.min(by_memory).clamp(1, MAX_JOBS)
}

fn available_memory_mb() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let value = text
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()?;
    Some(value / 1024)
}

fn full_pressure_avg10() -> Option<f64> {
    let text = fs::read_to_string("/proc/pressure/memory").ok()?;
    text.lines()
        .find(|line| line.starts_with("full "))?
        .split_whitespace()
        .find_map(|field| field.strip_prefix("avg10="))?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::must_stop;

    #[test]
    fn pressure_with_plenty_of_memory_does_not_stop_the_gate() {
        // The runs that motivated this: stalls from the scope's MemoryHigh with memory to spare.
        assert!(!must_stop(14_050, 23.83));
        assert!(!must_stop(14_534, 55.15));
    }

    #[test]
    fn scarce_memory_stops_the_gate() {
        assert!(must_stop(2_000, 0.0));
        assert!(must_stop(5_000, 20.0));
        assert!(!must_stop(5_000, 19.9));
    }
}
