//! Time and frame timing operations.

/// Trait for frame timing and operations.
pub trait Time {
    /// Returns the time elapsed since the last frame in seconds.
    fn get_frame_time(&self) -> f32;

    /// Returns the frame spike in milliseconds if the last frame exceeded the threshold, or 0.0.
    fn get_frame_spike_ms(&self) -> f32;

    /// Returns redraw-to-redraw interval in seconds when available.
    fn get_redraw_interval(&self) -> f32 {
        self.get_frame_time()
    }

    /// Returns surface acquire wait in seconds when available.
    fn get_acquire_wait(&self) -> f32 {
        0.0
    }

    /// Returns present wait in seconds when available.
    fn get_present_wait(&self) -> f32 {
        0.0
    }

    /// Called at the start of each frame to update internal state.
    fn update(&mut self);
}
