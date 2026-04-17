//! Time and frame timing state for wgpu backend.

use std::time::Instant;

/// Threshold in seconds above which a frame is considered a spike (18ms).
const SPIKE_THRESHOLD: f32 = 0.018;

/// Tracks frame timing information.
pub struct TimeState {
    start_time: Instant,
    last_redraw_time: Instant,
    last_frame_time: Instant,
    delta_time: f32,
    frame_spike_ms: f32,
    redraw_interval: f32,
    acquire_wait: f32,
    present_wait: f32,
}

impl TimeState {
    /// Creates a new time state starting from now.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_redraw_time: now,
            last_frame_time: now,
            delta_time: 0.0,
            frame_spike_ms: 0.0,
            redraw_interval: 0.0,
            acquire_wait: 0.0,
            present_wait: 0.0,
        }
    }

    /// Marks the moment a redraw request starts processing.
    pub fn begin_redraw(&mut self) -> Instant {
        let now = Instant::now();
        self.redraw_interval = now.duration_since(self.last_redraw_time).as_secs_f32();
        self.last_redraw_time = now;

        now
    }

    /// Finalizes frame timing once the surface is acquired and the game frame can run.
    pub fn begin_frame(&mut self) {
        let now = Instant::now();
        self.delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        if self.delta_time > SPIKE_THRESHOLD {
            self.frame_spike_ms = self.delta_time * 1000.0;
        } else {
            self.frame_spike_ms = 0.0;
        }
    }

    pub fn set_acquire_wait(&mut self, started_at: Instant) {
        self.acquire_wait = started_at.elapsed().as_secs_f32();
    }

    pub fn set_present_wait(&mut self, started_at: Instant) {
        self.present_wait = started_at.elapsed().as_secs_f32();
    }

    /// Returns the time elapsed since the last frame in seconds.
    pub fn frame_time(&self) -> f32 {
        self.delta_time
    }

    /// Returns the frame spike in milliseconds if the last frame exceeded the threshold, or 0.0.
    pub fn frame_spike_ms(&self) -> f32 {
        self.frame_spike_ms
    }

    pub fn redraw_interval(&self) -> f32 {
        self.redraw_interval
    }

    pub fn acquire_wait(&self) -> f32 {
        self.acquire_wait
    }

    pub fn present_wait(&self) -> f32 {
        self.present_wait
    }

    /// Returns the time elapsed since the application started in seconds.
    pub fn elapsed(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

impl Default for TimeState {
    fn default() -> Self {
        Self::new()
    }
}
