use bishop::time::Time;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const TIMING_TRACE_FILE_NAME: &str = "bishop-dt-trace.csv";
const TIMING_TRACE_FLUSH_INTERVAL: u16 = 120;

#[derive(Default, Clone, Copy, Debug)]
pub struct TimingTraceSample {
    pub raw_dt: f32,
    pub sim_dt: f32,
    pub redraw_interval: f32,
    pub acquire_wait: f32,
    pub present_wait: f32,
    pub fixed_steps: u8,
    pub accumulator: f32,
    pub alpha: f32,
}

impl TimingTraceSample {
    pub fn new(raw_dt: f32, accumulator_dt: f32, time: &impl Time) -> Self {
        Self {
            raw_dt,
            sim_dt: accumulator_dt,
            redraw_interval: time.get_redraw_interval(),
            acquire_wait: time.get_acquire_wait(),
            present_wait: time.get_present_wait(),
            fixed_steps: 0,
            accumulator: 0.0,
            alpha: 0.0,
        }
    }

    pub fn with_frame_state(self, fixed_steps: u8, accumulator: f32, alpha: f32) -> Self {
        Self {
            fixed_steps,
            accumulator,
            alpha,
            ..self
        }
    }
}

pub struct TimingTraceLogger {
    path: PathBuf,
    writer: Option<BufWriter<File>>,
    frame_index: u64,
    frames_since_flush: u16,
    disabled: bool,
}

impl TimingTraceLogger {
    pub fn new() -> Self {
        Self {
            path: env::temp_dir().join(TIMING_TRACE_FILE_NAME),
            writer: None,
            frame_index: 0,
            frames_since_flush: 0,
            disabled: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn record(&mut self, sample: TimingTraceSample) {
        if self.disabled {
            return;
        }

        let frame_index = self.frame_index;

        let Some(writer) = self.writer_mut() else {
            return;
        };

        if writeln!(
            writer,
            "{},{:.4},{:.4},{:.4},{:.4},{:.4},{},{:.6},{:.6}",
            frame_index,
            sample.raw_dt * 1000.0,
            sample.sim_dt * 1000.0,
            sample.redraw_interval * 1000.0,
            sample.acquire_wait * 1000.0,
            sample.present_wait * 1000.0,
            sample.fixed_steps,
            sample.accumulator,
            sample.alpha,
        )
        .is_err()
        {
            self.writer = None;
            self.disabled = true;
            return;
        }

        self.frame_index += 1;
        self.frames_since_flush = self.frames_since_flush.saturating_add(1);
        if self.frames_since_flush >= TIMING_TRACE_FLUSH_INTERVAL {
            if let Some(writer) = self.writer.as_mut() {
                if writer.flush().is_err() {
                    self.writer = None;
                    self.disabled = true;
                    return;
                }
            }
            self.frames_since_flush = 0;
        }
    }

    fn writer_mut(&mut self) -> Option<&mut BufWriter<File>> {
        if self.writer.is_none() {
            self.writer = Self::open_writer(&self.path);
            if self.writer.is_none() {
                self.disabled = true;
                return None;
            }
        }

        self.writer.as_mut()
    }

    fn open_writer(path: &Path) -> Option<BufWriter<File>> {
        let file = File::create(path).ok()?;
        let mut writer = BufWriter::new(file);
        writeln!(
            writer,
            "frame,raw_dt_ms,sim_dt_ms,redraw_interval_ms,acquire_wait_ms,present_wait_ms,fixed_steps,accumulator,alpha"
        )
        .ok()?;
        Some(writer)
    }
}

impl Drop for TimingTraceLogger {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            let _ = writer.flush();
        }
    }
}
