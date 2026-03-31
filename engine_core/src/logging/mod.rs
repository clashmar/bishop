// engine_core/src/logging/mod.rs
use crate::storage::editor_config::app_dir;
use std::backtrace::Backtrace;
use std::fs::{self, OpenOptions};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use flexi_logger::*;
use log::Record;
use once_cell::sync::Lazy;
use std::io::*;

pub use crate::onscreen_debug;
pub use crate::onscreen_error;
pub use crate::onscreen_info;
pub use crate::onscreen_log;
pub use crate::onscreen_warn;

const MAX_LOG_ENTRIES: usize = 500;

#[derive(Clone)]
pub struct LogEntry {
    pub level: log::Level,
    pub message: String,
    pub time: String,
    pub file: &'static str,
    pub line: u32,
    pub count: u32,
}

/// Stores log entries with a monotonically increasing counter for change detection.
#[derive(Default)]
pub struct LogHistory {
    entries: Vec<LogEntry>,
    total_pushed: usize,
}

impl LogHistory {
    pub fn push(
        &mut self,
        level: log::Level,
        message: String,
        time: String,
        file: &'static str,
        line: u32,
    ) {
        if let Some(last) = self.entries.last_mut()
            && last.level == level
            && last.message == message
            && last.file == file
            && last.line == line
        {
            last.count += 1;
            last.time = time;
            self.total_pushed += 1;
            return;
        }
        if self.entries.len() >= MAX_LOG_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(LogEntry {
            level,
            message,
            time,
            file,
            line,
            count: 1,
        });
        self.total_pushed += 1;
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    pub fn last(&self) -> Option<&LogEntry> {
        self.entries.last()
    }

    /// Returns the total number of entries ever pushed, even after cycling.
    pub fn total_pushed(&self) -> usize {
        self.total_pushed
    }

    /// Clears all entries but preserves total_pushed for change detection.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

pub static LOG_HISTORY: Lazy<Mutex<LogHistory>> = Lazy::new(|| Mutex::new(LogHistory::default()));

/// File locations used by runtime telemetry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTelemetryPaths {
    pub log_dir: PathBuf,
    pub crash_report_path: PathBuf,
    pub log_basename: String,
}

/// Returns the current time formatted as `HH:MM:SS.mmm`.
pub fn now_str() -> String {
    format!("{}", DeferredNow::new().format("%H:%M:%S%.3f"))
}

/// Helper macro that allow logs to be displayed by
/// the program and printed to the console.
#[macro_export]
macro_rules! onscreen_log {
    ($lvl:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let time = $crate::logging::now_str();
        println!("{} {:5} [{}:{}] {}", time, $lvl, file!(), line!(), &msg);
        log::log!($lvl, $($arg)*);
        {
            let mut history = $crate::logging::LOG_HISTORY.lock().unwrap();
            history.push($lvl, msg.clone(), time, file!(), line!());
            drop(history);
        }
    }};
}

/// Helper macro that allow logs to be displayed by the program.
#[macro_export]
macro_rules! onscreen_info  { ($($arg:tt)*) => { $crate::onscreen_log!(log::Level::Info,  $($arg)*) }; }

/// Helper macro that allow logs to be displayed by the program.
#[macro_export]
macro_rules! onscreen_warn  { ($($arg:tt)*) => { $crate::onscreen_log!(log::Level::Warn,  $($arg)*) }; }

/// Helper macro that allow logs to be displayed by the program.
#[macro_export]
macro_rules! onscreen_error { ($($arg:tt)*) => { $crate::onscreen_log!(log::Level::Error, $($arg)*) }; }

/// Helper macro that allow logs to be displayed by the program.
#[macro_export]
macro_rules! onscreen_debug { ($($arg:tt)*) => { $crate::onscreen_log!(log::Level::Debug, $($arg)*) }; }

/// Initializes the system logger.
pub fn init_file_logger() {
    let log_dir = runtime_log_dir();
    init_logger_with_basename(&log_dir, "bishop_engine");
    onscreen_info!("Log dir: {}.", log_dir.display());
}

/// Returns the directory used for runtime log files.
pub fn runtime_log_dir() -> PathBuf {
    app_dir().join("logs")
}

/// Returns the log and crash-report paths for a runtime process.
pub fn runtime_telemetry_paths(log_dir: PathBuf, process_name: &str) -> RuntimeTelemetryPaths {
    let log_basename = sanitise_process_name(process_name);
    let crash_report_path = log_dir.join(format!("{log_basename}_crash.log"));
    RuntimeTelemetryPaths {
        log_dir,
        crash_report_path,
        log_basename,
    }
}

/// Initializes file logging and panic reporting for a shipped runtime.
pub fn init_runtime_telemetry(process_name: &str) -> RuntimeTelemetryPaths {
    let paths = runtime_telemetry_paths(runtime_log_dir(), process_name);
    init_logger_with_basename(&paths.log_dir, &paths.log_basename);
    install_panic_hook(paths.clone());
    onscreen_info!("Log dir: {}.", paths.log_dir.display());
    onscreen_info!("Crash report: {}.", paths.crash_report_path.display());
    paths
}

fn init_logger_with_basename(log_dir: &Path, basename: &str) {
    fs::create_dir_all(log_dir).expect("Unable to create log directory.");

    let file_spec = FileSpec::default()
        .directory(log_dir)
        .basename(basename)
        .suffix("log");

    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(file_spec)
        .format(format_log_record)
        .rotate(
            Criterion::Size(5_000_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(5),
        )
        .write_mode(WriteMode::BufferAndFlush)
        .start()
        .expect("Unable to init logger.");
}

fn install_panic_hook(paths: RuntimeTelemetryPaths) {
    panic::set_hook(Box::new(move |panic_info| {
        let backtrace = Backtrace::force_capture();
        let exe_path = std::env::current_exe().ok();
        let location = panic_info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let message = panic_payload_message(panic_info.payload());
        let report = format!(
            "{} PANIC [{}] {}\nExecutable: {}\nCrash report: {}\nBacktrace:\n{}\n\n",
            now_str(),
            location,
            message,
            exe_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<unknown>".to_string()),
            paths.crash_report_path.display(),
            backtrace
        );

        let _ = fs::create_dir_all(&paths.log_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&paths.crash_report_path)
        {
            let _ = file.write_all(report.as_bytes());
        }

        eprintln!("{report}");
    }));
}

fn sanitise_process_name(process_name: &str) -> String {
    let mut out = String::new();
    let mut previous_was_separator = false;

    for ch in process_name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
            continue;
        }

        if !out.is_empty() && !previous_was_separator {
            out.push('_');
            previous_was_separator = true;
        }
    }

    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "game".to_string()
    } else {
        trimmed.to_string()
    }
}

fn format_log_record(write: &mut dyn Write, now: &mut DeferredNow, record: &Record) -> Result<()> {
    write!(
        write,
        "{} {:5} [{}  {}:{}] {}",
        now.format("%Y-%m-%d %H:%M:%S%.3f"),
        record.level(),
        record.module_path().unwrap_or("<unknown>"),
        record.file().unwrap_or("<unknown>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_string();
    }
    "Unknown panic payload".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn runtime_telemetry_paths_use_sanitized_process_name() {
        let paths = runtime_telemetry_paths(PathBuf::from("/tmp/bishop-tests"), "My Cool Game!");

        assert_eq!(paths.log_dir, PathBuf::from("/tmp/bishop-tests"));
        assert_eq!(
            paths.crash_report_path,
            PathBuf::from("/tmp/bishop-tests/my_cool_game_crash.log")
        );
        assert_eq!(paths.log_basename, "my_cool_game");
    }
}
