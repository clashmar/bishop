// engine_core/src/logging/mod.rs
use crate::storage::editor_config::app_dir;
use flexi_logger::*;
use log::Record;
use once_cell::sync::Lazy;
use std::io::*;
use std::sync::Mutex;

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

// Global mutable buffer that stores the most recent message (kept for backwards compatibility).
pub static LAST_LOG: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

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
            let mut buf = $crate::logging::LAST_LOG.lock().unwrap();
            *buf = msg;
            drop(buf);
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
    let log_dir = app_dir().join("logs");

    let file_spec = FileSpec::default()
        .directory(&log_dir)
        .basename("bishop_engine")
        .suffix("log");

    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(file_spec)
        .format(my_formatter)
        .rotate(
            Criterion::Size(5_000_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(5),
        )
        .write_mode(WriteMode::BufferAndFlush)
        .start()
        .expect("Unable to init logger.");

    onscreen_info!("Log dir: {}.", &log_dir.display());

    fn my_formatter(write: &mut dyn Write, now: &mut DeferredNow, record: &Record) -> Result<()> {
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
}
