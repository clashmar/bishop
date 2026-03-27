// engine_core/src/diagnostics/metrics.rs
//! Metric types for engine diagnostics.

use std::collections::HashMap;
use std::collections::VecDeque;

/// Number of frames to keep for rolling average calculations.
const FRAME_HISTORY_SIZE: usize = 120;

/// Frame timing and FPS metrics with rolling window averaging.
#[derive(Clone, Debug)]
pub struct FrameMetrics {
    /// Recent frame times in seconds.
    frame_times: VecDeque<f32>,
    /// Current FPS based on rolling average.
    pub fps: f32,
    /// Average frame time in milliseconds.
    pub avg_frame_time_ms: f32,
    /// Minimum frame time in the window (ms).
    pub min_frame_time_ms: f32,
    /// Maximum frame time in the window (ms).
    pub max_frame_time_ms: f32,
}

impl Default for FrameMetrics {
    fn default() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(FRAME_HISTORY_SIZE),
            fps: 0.0,
            avg_frame_time_ms: 0.0,
            min_frame_time_ms: 0.0,
            max_frame_time_ms: 0.0,
        }
    }
}

impl FrameMetrics {
    /// Record a new frame time and update metrics.
    pub fn record_frame(&mut self, dt: f32) {
        // Add new frame time
        self.frame_times.push_back(dt);

        // Remove old frame times if over capacity
        while self.frame_times.len() > FRAME_HISTORY_SIZE {
            self.frame_times.pop_front();
        }

        // Calculate metrics
        if !self.frame_times.is_empty() {
            let sum: f32 = self.frame_times.iter().sum();
            let avg = sum / self.frame_times.len() as f32;

            self.avg_frame_time_ms = avg * 1000.0;
            self.fps = if avg > 0.0 { 1.0 / avg } else { 0.0 };

            self.min_frame_time_ms =
                self.frame_times.iter().copied().fold(f32::MAX, f32::min) * 1000.0;
            self.max_frame_time_ms = self.frame_times.iter().copied().fold(0.0, f32::max) * 1000.0;
        }
    }
}

/// Asset-related metrics.
#[derive(Clone, Debug, Default)]
pub struct AssetMetrics {
    /// Number of loaded textures.
    pub texture_count: usize,
    /// Number of tile definitions.
    pub tile_def_count: usize,
    /// Number of sprite ID mappings.
    pub sprite_id_count: usize,
    /// Number of script ID mappings.
    pub script_id_count: usize,
}

/// Script system metrics.
#[derive(Clone, Debug, Default)]
pub struct ScriptMetrics {
    /// Number of loaded script definitions.
    pub loaded_count: usize,
    /// Number of active script instances.
    pub instance_count: usize,
    /// Number of registered event listeners.
    pub event_listener_count: usize,
    /// Reference counts per script ID.
    pub ref_counts: HashMap<usize, usize>,
}

/// ECS metrics.
#[derive(Clone, Debug, Default)]
pub struct EcsMetrics {
    /// Total number of entities.
    pub entity_count: usize,
    /// Number of component stores.
    pub component_store_count: usize,
    /// Component counts by type name.
    pub components_by_type: HashMap<String, usize>,
}

/// Editor command stack metrics.
#[derive(Clone, Debug, Default)]
pub struct CommandMetrics {
    /// Size of the undo stack.
    pub undo_stack_size: usize,
    /// Size of the redo stack.
    pub redo_stack_size: usize,
    /// Number of pending commands.
    pub pending_size: usize,
}

/// Combined snapshot of all diagnostic metrics.
#[derive(Clone, Debug, Default)]
pub struct DiagnosticsSnapshot {
    pub frame: FrameMetrics,
    pub assets: AssetMetrics,
    pub scripts: ScriptMetrics,
    pub ecs: EcsMetrics,
    pub commands: CommandMetrics,
}

/// Warning types for threshold violations.
#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticWarning {
    /// FPS dropped below threshold.
    LowFps(f32),
    /// Event listener count is growing (possible leak).
    EventListenerGrowth { current: usize, previous: usize },
    /// High entity count.
    HighEntityCount(usize),
    /// Large undo stack.
    LargeUndoStack(usize),
    /// Script instance leak detected (instances without matching entities).
    ScriptInstanceLeak { orphaned: usize },
}

impl DiagnosticWarning {
    /// Returns a human-readable description of the warning.
    pub fn description(&self) -> String {
        match self {
            DiagnosticWarning::LowFps(fps) => {
                format!("Low FPS: {:.1}", fps)
            }
            DiagnosticWarning::EventListenerGrowth { current, previous } => {
                format!("Event listeners growing: {} -> {}", previous, current)
            }
            DiagnosticWarning::HighEntityCount(count) => {
                format!("High entity count: {}", count)
            }
            DiagnosticWarning::LargeUndoStack(size) => {
                format!("Large undo stack: {}", size)
            }
            DiagnosticWarning::ScriptInstanceLeak { orphaned } => {
                format!("Script instance leak: {} orphaned", orphaned)
            }
        }
    }
}
