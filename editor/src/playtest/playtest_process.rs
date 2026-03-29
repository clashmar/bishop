// editor/src/playtest/playtest_process.rs
use engine_core::logging::{now_str, LOG_HISTORY};
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Manages a running playtest process, capturing its stdout/stderr output.
pub struct PlaytestProcess {
    child: Child,
    stdout_rx: Receiver<String>,
    stderr_rx: Receiver<String>,
}

impl PlaytestProcess {
    /// Spawns a new playtest process with the given executable and payload paths.
    /// Output from stdout/stderr is captured and can be polled via `poll()`.
    pub fn spawn(exe_path: &Path, payload_path: &Path) -> io::Result<Self> {
        let mut command = Command::new(exe_path);
        command.arg(payload_path);

        let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("Failed to capture stderr"))?;

        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if stdout_tx.send(line).is_err() {
                    break;
                }
            }
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if stderr_tx.send(line).is_err() {
                    break;
                }
            }
        });

        Self::push_log(log::Level::Info, "Started");

        Ok(Self {
            child,
            stdout_rx,
            stderr_rx,
        })
    }

    /// Drains available output from the process and pushes to LOG_HISTORY.
    pub fn poll(&mut self) -> bool {
        self.drain_channel(&self.stdout_rx, log::Level::Info);
        self.drain_channel(&self.stderr_rx, log::Level::Error);

        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.drain_channel(&self.stdout_rx, log::Level::Info);
                self.drain_channel(&self.stderr_rx, log::Level::Error);

                let msg = if status.success() {
                    "Exited".to_string()
                } else {
                    format!("Exited with code {}", status.code().unwrap_or(-1))
                };
                Self::push_log(log::Level::Info, &msg);
                false
            }
            Ok(None) => true,
            Err(e) => {
                Self::push_log(log::Level::Error, &format!("Error checking process: {e}"));
                false
            }
        }
    }

    /// Kills the child process if still running.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn drain_channel(&self, rx: &Receiver<String>, level: log::Level) {
        while let Ok(line) = rx.try_recv() {
            Self::push_log(level, &line);
        }
    }

    fn push_log(level: log::Level, message: &str) {
        let prefixed = format!("[PLAYTEST] {}", message);
        if let Ok(mut history) = LOG_HISTORY.lock() {
            history.push(level, prefixed, now_str(), "playtest", 0);
        }
    }
}

impl Drop for PlaytestProcess {
    fn drop(&mut self) {
        self.kill();
    }
}
