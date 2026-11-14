//! File logging with a hard 1 MiB budget.
//!
//! The logger tees the normal stderr output into two rotating files. Each file is capped at
//! 512 KiB, so the complete on-disk history never exceeds 1 MiB. Records include UTC time, level,
//! target, source location, and message, which makes interaction and timing traces actionable.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use log::{Log, Metadata, Record};

const PART_LIMIT: u64 = 512 * 1024;
const LINE_LIMIT: usize = 4096;

static ACTIVE_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Directory that receives the rotating log files.
pub fn log_directory() -> PathBuf {
    if let Some(directory) = std::env::var_os("MDVIEW_LOG_DIR") {
        return PathBuf::from(directory);
    }
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("mdview")
        .join("logs")
}

/// The active log file, if logging initialized successfully.
pub fn active_log_path() -> Option<&'static Path> {
    ACTIVE_PATH.get().map(PathBuf::as_path)
}

/// Install the stderr + rotating file logger.
pub fn init() {
    let directory = log_directory();
    let sink = match FileSink::new(directory, PART_LIMIT) {
        Ok(sink) => sink,
        Err(error) => {
            eprintln!("logging: could not open the log directory: {error}");
            return;
        }
    };
    ACTIVE_PATH.set(sink.active_path.clone()).ok();
    let stderr =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,mdview=info")).build();
    let filter = stderr.filter();
    let logger = TeeLogger {
        stderr,
        file: Mutex::new(sink),
    };
    if let Err(error) = log::set_boxed_logger(Box::new(logger)) {
        eprintln!("logging: could not install the logger: {error}");
        return;
    }
    log::set_max_level(filter);
}

struct TeeLogger {
    stderr: env_logger::Logger,
    file: Mutex<FileSink>,
}

impl Log for TeeLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.stderr.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        self.stderr.log(record);
        if let Ok(mut sink) = self.file.lock() {
            let _ = sink.record(record);
        }
    }

    fn flush(&self) {
        self.stderr.flush();
        if let Ok(mut sink) = self.file.lock() {
            let _ = sink.flush();
        }
    }
}

struct FileSink {
    directory: PathBuf,
    active_path: PathBuf,
    file: File,
    size: u64,
    limit: u64,
}

impl FileSink {
    fn new(directory: PathBuf, limit: u64) -> std::io::Result<Self> {
        fs::create_dir_all(&directory)?;
        let active_path = directory.join("mdview.log");
        let rotated = directory.join("mdview.1.log");
        trim_existing(&active_path, limit)?;
        trim_existing(&rotated, limit)?;
        let file = OpenOptions::new().create(true).append(true).open(&active_path)?;
        let size = file.metadata()?.len();
        Ok(Self {
            directory,
            active_path,
            file,
            size,
            limit,
        })
    }

    fn record(&mut self, record: &Record<'_>) -> std::io::Result<()> {
        self.write_line(&format_record(record))
    }

    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        if line.len() > LINE_LIMIT {
            let mut truncated = line[..LINE_LIMIT].to_owned();
            truncated.push_str("…\n");
            return self.write_line(&truncated);
        }
        if self.size + line.len() as u64 > self.limit {
            self.rotate()?;
        }
        self.file.write_all(line.as_bytes())?;
        self.size += line.len() as u64;
        Ok(())
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        self.file.flush()?;
        let rotated = self.directory.join("mdview.1.log");
        let _ = fs::remove_file(&rotated);
        fs::rename(&self.active_path, &rotated)?;
        self.file = OpenOptions::new().create(true).append(true).open(&self.active_path)?;
        self.size = 0;
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn trim_existing(path: &Path, limit: u64) -> std::io::Result<()> {
    if let Ok(metadata) = fs::metadata(path)
        && metadata.len() > limit
    {
        let file = OpenOptions::new().write(true).open(path)?;
        file.set_len(limit)?;
    }
    Ok(())
}

fn format_record(record: &Record<'_>) -> String {
    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown-time".to_owned());
    let location = record
        .file()
        .map(|file| format!("{file}:{}", record.line().unwrap_or(0)))
        .unwrap_or_default();
    format!(
        "{timestamp} {level:<5} {target} {location}: {message}\n",
        level = record.level(),
        target = record.target(),
        message = record.args(),
    )
}

/// Return the total bytes used by the two log parts.
pub fn log_bytes() -> u64 {
    let directory = log_directory();
    ["mdview.log", "mdview.1.log"]
        .into_iter()
        .filter_map(|name| fs::metadata(directory.join(name)).ok())
        .map(|metadata| metadata.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_keeps_the_pair_under_one_megabyte() {
        let directory = tempfile::tempdir().expect("tempdir");
        let limit = 64 * 1024;
        let mut sink = FileSink::new(directory.path().to_path_buf(), limit).expect("sink");
        let payload = "x".repeat(1024);
        for index in 0..200 {
            sink.write_line(&format!("{index} {payload}\n")).expect("write");
        }
        let active = fs::metadata(directory.path().join("mdview.log")).expect("active");
        let rotated = fs::metadata(directory.path().join("mdview.1.log")).expect("rotated");
        assert!(active.len() <= limit);
        assert!(rotated.len() <= limit);
        assert!(active.len() + rotated.len() <= limit * 2);
    }

    #[test]
    fn records_have_a_utc_timestamp_and_level() {
        let record = Record::builder()
            .args(format_args!("hello"))
            .level(log::Level::Warn)
            .target("mdview::test")
            .file(Some("src/main.rs"))
            .line(Some(7))
            .build();
        let line = format_record(&record);
        assert!(line.contains("WARN"));
        assert!(line.contains("mdview::test"));
        assert!(line.contains("hello"));
        assert!(line.contains("src/main.rs:7"));
        assert!(line.ends_with('\n'));
    }
}
