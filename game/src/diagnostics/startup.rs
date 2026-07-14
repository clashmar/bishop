use std::time::{Duration, Instant};

/// Stable telemetry stage labels for runtime startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupTelemetryStage {
    /// Reads startup payload files from disk.
    ReadFiles,
    /// Parses startup payload data into runtime structs.
    ParseData,
    /// Builds the runtime engine from loaded startup data.
    BuildEngine,
}

impl StartupTelemetryStage {
    /// Returns the stable label for this startup stage.
    pub fn label(self) -> &'static str {
        match self {
            StartupTelemetryStage::ReadFiles => "read_files",
            StartupTelemetryStage::ParseData => "parse_data",
            StartupTelemetryStage::BuildEngine => "build_engine",
        }
    }
}

/// One measured startup telemetry row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupTelemetryRow {
    /// The startup stage that was measured.
    pub stage: StartupTelemetryStage,
    /// The elapsed duration for the measured stage.
    pub duration: Duration,
}

/// An ordered snapshot of startup telemetry rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartupTelemetrySnapshot {
    /// The ordered startup telemetry rows.
    pub rows: Vec<StartupTelemetryRow>,
}

/// Records startup telemetry in phase order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartupTelemetryRecorder {
    rows: Vec<StartupTelemetryRow>,
}

impl StartupTelemetryRecorder {
    /// Records one measured startup stage.
    pub fn record(&mut self, stage: StartupTelemetryStage, duration: Duration) {
        self.rows.push(StartupTelemetryRow { stage, duration });
    }

    /// Measures a fallible startup stage and records its duration.
    pub fn measure<T, E>(
        &mut self,
        stage: StartupTelemetryStage,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let started = Instant::now();
        let result = f();
        self.record(stage, started.elapsed());
        result
    }

    /// Returns the recorded startup telemetry rows.
    pub(crate) fn rows(&self) -> &[StartupTelemetryRow] {
        &self.rows
    }

    /// Returns an ordered snapshot of the recorded rows.
    pub fn snapshot(&self) -> StartupTelemetrySnapshot {
        StartupTelemetrySnapshot {
            rows: self.rows.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn startup_telemetry_stage_labels_are_stable() {
        assert_eq!(StartupTelemetryStage::ReadFiles.label(), "read_files");
        assert_eq!(StartupTelemetryStage::ParseData.label(), "parse_data");
        assert_eq!(StartupTelemetryStage::BuildEngine.label(), "build_engine");
    }

    #[test]
    fn startup_telemetry_recorder_preserves_stage_order() {
        let mut recorder = StartupTelemetryRecorder::default();
        recorder.record(StartupTelemetryStage::ReadFiles, Duration::from_millis(8));
        recorder.record(StartupTelemetryStage::ParseData, Duration::from_millis(3));

        let snapshot = recorder.snapshot();
        assert_eq!(snapshot.rows.len(), 2);
        assert_eq!(snapshot.rows[0].stage, StartupTelemetryStage::ReadFiles);
        assert_eq!(snapshot.rows[1].stage, StartupTelemetryStage::ParseData);
    }

    #[test]
    fn startup_telemetry_recorder_records_error_stage() {
        let mut recorder = StartupTelemetryRecorder::default();
        let result: Result<(), &str> =
            recorder.measure(StartupTelemetryStage::ReadFiles, || Err("fail"));

        assert!(result.is_err());
        let snapshot = recorder.snapshot();
        assert_eq!(snapshot.rows.len(), 1);
        assert_eq!(snapshot.rows[0].stage, StartupTelemetryStage::ReadFiles);
    }
}
