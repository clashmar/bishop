use std::sync::mpsc;

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
