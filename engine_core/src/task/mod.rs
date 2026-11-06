#[cfg(not(target_os = "emscripten"))]
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

/// Runs a closure on a background thread and provides a non-blocking poll
/// to retrieve the result. The main loop must never await — call `poll()`
/// once per frame and handle `Some(result)` when it arrives.
pub struct BackgroundTask<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T: Send + 'static> BackgroundTask<T> {
    /// Spawns `f` on a new thread. Returns immediately.
    pub fn spawn<F: FnOnce() -> T + Send + 'static>(f: F) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        Self { receiver: rx }
    }

    /// Returns `Some(result)` if the background work is complete, `None` otherwise.
    /// Never blocks.
    pub fn poll(&mut self) -> Option<T> {
        self.receiver.try_recv().ok()
    }
}

/// Contract for a system that runs persistently in the background and must be
/// ticked once per frame. Implementations must never block inside `poll`.
pub trait BackgroundService {
    /// Called once per frame by the game loop. Must never block.
    fn poll(&mut self, dt: f32);
}

#[cfg(not(target_os = "emscripten"))]
#[derive(Clone)]
struct FileReadJob {
    id: String,
    path: PathBuf,
}

/// Completion from a background file read.
#[derive(Clone, Debug)]
pub struct FileReadCompleted {
    /// Caller-supplied identifier associated with the file read.
    pub id: String,
    /// Source file path for the completed read.
    pub path: PathBuf,
    /// File contents read from disk.
    pub result: Result<Vec<u8>, String>,
}

/// Bounded background pool for file reads.
///
/// The pool owns a fixed number of worker threads and exposes a cloneable
/// submit handle. Call [`FileReadPool::queue_read`] to enqueue work and
/// [`FileReadPool::try_recv_completed`] to poll finished reads without
/// blocking.
#[derive(Clone)]
pub struct FileReadPool {
    #[cfg(not(target_os = "emscripten"))]
    submit_tx: mpsc::SyncSender<FileReadJob>,
    #[cfg(target_os = "emscripten")]
    failure_tx: mpsc::Sender<FileReadCompleted>,
    completed_rx: Arc<Mutex<mpsc::Receiver<FileReadCompleted>>>,
    worker_count: usize,
}

impl FileReadPool {
    /// Creates a new bounded file-read pool.
    pub fn new() -> Self {
        let (completed_tx, completed_rx) = mpsc::channel();
        #[cfg(target_os = "emscripten")]
        {
            return Self {
                failure_tx: completed_tx,
                completed_rx: Arc::new(Mutex::new(completed_rx)),
                worker_count: 0,
            };
        }
        #[cfg(not(target_os = "emscripten"))]
        {
            let worker_count = std::thread::available_parallelism()
                .map(|count| count.get().clamp(2, 4))
                .unwrap_or(2);
            let queue_capacity = worker_count * 4;
            let (submit_tx, submit_rx) = mpsc::sync_channel::<FileReadJob>(queue_capacity);
            let submit_rx = Arc::new(Mutex::new(submit_rx));

            for _ in 0..worker_count {
                let submit_rx = Arc::clone(&submit_rx);
                let completed_tx = completed_tx.clone();
                std::thread::spawn(move || {
                    loop {
                        let job = {
                            let receiver = submit_rx.lock().unwrap_or_else(|poisoned| {
                                poisoned.into_inner()
                            });
                            receiver.recv()
                        };

                        let Ok(job) = job else {
                            break;
                        };

                        let result = fs::read(&job.path).map_err(|error| {
                            format!("failed to read {}: {error}", job.path.display())
                        });
                        let _ = completed_tx.send(FileReadCompleted {
                            id: job.id,
                            path: job.path,
                            result,
                        });
                    }
                });
            }

            Self {
                submit_tx,
                completed_rx: Arc::new(Mutex::new(completed_rx)),
                worker_count,
            }
        }
    }

    /// Returns the number of worker threads owned by the pool.
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Queues a file read for the given `id` and `path`.
    pub fn queue_read(&self, id: String, path: PathBuf) {
        #[cfg(target_os = "emscripten")]
        let _ = self.failure_tx.send(unavailable_sound_result(id, path));
        #[cfg(not(target_os = "emscripten"))]
        let _ = self.submit_tx.send(FileReadJob { id, path });
    }

    /// Returns the next completed file read, if one is available.
    pub fn try_recv_completed(&self) -> Option<FileReadCompleted> {
        self.completed_rx
            .lock()
            .ok()
            .and_then(|receiver| receiver.try_recv().ok())
    }
}

#[cfg(any(test, target_os = "emscripten"))]
fn unavailable_sound_result(id: String, path: PathBuf) -> FileReadCompleted {
    FileReadCompleted {
        id,
        result: Err(format!("browser audio source unavailable: {}", path.display())),
        path,
    }
}

impl Default for FileReadPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{FileReadPool, unavailable_sound_result};
    use std::fs;
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bishop-file-read-pool-{nanos}.bin"))
    }

    #[test]
    fn browser_queue_when_audio_file_is_unavailable_returns_path_error() {
        let path = std::path::PathBuf::from("audio/sfx/missing.wav");
        let completed = unavailable_sound_result("sfx/missing".into(), path.clone());

        assert_eq!(completed.id, "sfx/missing");
        assert_eq!(completed.path, path);
        assert!(completed.result.unwrap_err().contains(&path.display().to_string()));
    }

    #[test]
    fn file_read_pool_worker_count_is_bounded() {
        let pool = FileReadPool::new();

        assert!((2..=4).contains(&pool.worker_count()));
    }

    #[test]
    fn file_read_pool_reads_bytes_from_a_cloned_handle() {
        let pool = FileReadPool::new();
        let submitter = pool.clone();
        let path = unique_temp_path();
        let expected = vec![1_u8, 2, 3, 4];

        fs::write(&path, &expected).unwrap();
        submitter.queue_read("test/sound".to_string(), path.clone());

        let completed = loop {
            if let Some(completed) = pool.try_recv_completed() {
                break completed;
            }
            std::thread::sleep(Duration::from_millis(10));
        };

        assert_eq!(completed.id, "test/sound");
        assert_eq!(completed.result.unwrap(), expected);

        let _ = fs::remove_file(path);
    }
}
