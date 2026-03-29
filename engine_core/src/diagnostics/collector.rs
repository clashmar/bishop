// engine_core/src/diagnostics/collector.rs
//! Collects and aggregates metrics from engine systems.

use super::metrics::*;

/// Thresholds for generating warnings.
pub struct DiagnosticsThresholds {
    /// FPS below this triggers a warning.
    pub low_fps: f32,
    /// Entity count above this triggers a warning.
    pub high_entity_count: usize,
    /// Undo stack size above this triggers a warning.
    pub large_undo_stack: usize,
    /// Event listener growth percentage that triggers a warning.
    pub listener_growth_threshold: f32,
}

impl Default for DiagnosticsThresholds {
    fn default() -> Self {
        Self {
            low_fps: 30.0,
            high_entity_count: 1000,
            large_undo_stack: 200,
            listener_growth_threshold: 0.1, // 10% growth
        }
    }
}

/// Aggregates metrics from all engine systems.
pub struct DiagnosticsCollector {
    /// Current frame metrics.
    pub frame_metrics: FrameMetrics,
    /// Previous snapshot for comparison.
    previous_snapshot: Option<DiagnosticsSnapshot>,
    /// Warning thresholds.
    pub thresholds: DiagnosticsThresholds,
}

impl Default for DiagnosticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsCollector {
    pub fn new() -> Self {
        Self {
            frame_metrics: FrameMetrics::default(),
            previous_snapshot: None,
            thresholds: DiagnosticsThresholds::default(),
        }
    }

    /// Record a frame time for FPS/timing calculations.
    pub fn record_frame(&mut self, dt: f32) {
        self.frame_metrics.record_frame(dt);
    }

    /// Create a full diagnostics snapshot.
    pub fn snapshot(
        &mut self,
        asset_metrics: AssetMetrics,
        script_metrics: ScriptMetrics,
        ecs_metrics: EcsMetrics,
        command_metrics: CommandMetrics,
    ) -> DiagnosticsSnapshot {
        let snapshot = DiagnosticsSnapshot {
            frame: self.frame_metrics.clone(),
            assets: asset_metrics,
            scripts: script_metrics,
            ecs: ecs_metrics,
            commands: command_metrics,
        };

        // Store for next comparison
        self.previous_snapshot = Some(snapshot.clone());

        snapshot
    }

    /// Generate warnings based on current and previous snapshots.
    pub fn generate_warnings(&self, current: &DiagnosticsSnapshot) -> Vec<DiagnosticWarning> {
        let mut warnings = Vec::new();

        // Low FPS warning
        if current.frame.fps > 0.0 && current.frame.fps < self.thresholds.low_fps {
            warnings.push(DiagnosticWarning::LowFps(current.frame.fps));
        }

        // High entity count warning
        if current.ecs.entity_count > self.thresholds.high_entity_count {
            warnings.push(DiagnosticWarning::HighEntityCount(current.ecs.entity_count));
        }

        // Large undo stack warning
        if current.commands.undo_stack_size > self.thresholds.large_undo_stack {
            warnings.push(DiagnosticWarning::LargeUndoStack(
                current.commands.undo_stack_size,
            ));
        }

        // Event listener growth warning
        if let Some(prev) = &self.previous_snapshot {
            let prev_count = prev.scripts.event_listener_count;
            let curr_count = current.scripts.event_listener_count;

            if prev_count > 0 && curr_count > prev_count {
                let growth = (curr_count - prev_count) as f32 / prev_count as f32;
                if growth > self.thresholds.listener_growth_threshold {
                    warnings.push(DiagnosticWarning::EventListenerGrowth {
                        current: curr_count,
                        previous: prev_count,
                    });
                }
            }
        }

        warnings
    }

    /// Get the previous snapshot for comparison.
    pub fn previous_snapshot(&self) -> Option<&DiagnosticsSnapshot> {
        self.previous_snapshot.as_ref()
    }
}
