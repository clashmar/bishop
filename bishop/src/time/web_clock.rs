/// Browser monotonic time and per-frame delta in seconds.
pub(crate) struct WebClock {
    started_ms: f64,
    previous_ms: f64,
    current_ms: f64,
    frame_ms: f64,
}

impl WebClock {
    pub(crate) fn new(now_ms: f64) -> Self {
        Self {
            started_ms: now_ms,
            previous_ms: now_ms,
            current_ms: now_ms,
            frame_ms: 0.0,
        }
    }

    pub(crate) fn begin_frame(&mut self, now_ms: f64) {
        self.frame_ms = (now_ms - self.previous_ms).max(0.0);
        self.previous_ms = now_ms;
        self.current_ms = now_ms;
    }

    pub(crate) fn frame_time(&self) -> f32 {
        (self.frame_ms / 1000.0) as f32
    }

    pub(crate) fn frame_spike_ms(&self) -> f32 {
        if self.frame_ms > 18.0 {
            self.frame_ms as f32
        } else {
            0.0
        }
    }

    pub(crate) fn elapsed(&self) -> f64 {
        (self.current_ms - self.started_ms) / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::WebClock;

    #[test]
    fn browser_clock_tracks_frame_delta_and_elapsed_time() {
        let mut clock = WebClock::new(1000.0);

        clock.begin_frame(1016.0);

        assert!((clock.frame_time() - 0.016).abs() < 0.00001);
        assert!((clock.elapsed() - 0.016).abs() < 0.00001);
        assert_eq!(clock.frame_spike_ms(), 0.0);
    }

    #[test]
    fn browser_clock_reports_spikes_above_eighteen_milliseconds() {
        let mut clock = WebClock::new(1000.0);

        clock.begin_frame(1024.0);

        assert_eq!(clock.frame_spike_ms(), 24.0);
    }
}
