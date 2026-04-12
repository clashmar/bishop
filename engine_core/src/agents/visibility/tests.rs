use super::{AgentSessionState, AgentVisibilitySink, AgentVisibilitySnapshot, RecordingAgentSink};
use crate::logging::{clear_agent_visibility_sink, set_agent_visibility_sink};
use std::sync::{Arc, Mutex};

#[test]
fn agent_visibility_snapshot_includes_frame_timing_and_session_state() {
    let snapshot = AgentVisibilitySnapshot {
        session_state: AgentSessionState::Running,
        frame_time_ms: Some(16.7),
        smoothed_frame_time_ms: Some(14.2),
        mode: Some("playtest".to_string()),
        recent_log_count: 3,
    };

    let ron = match ron::to_string(&snapshot) {
        Ok(ron) => ron,
        Err(err) => panic!("failed to serialize snapshot: {err}"),
    };

    assert!(ron.contains("Running"));
    assert!(ron.contains("16.7"));
    assert!(ron.contains("14.2"));
}

#[test]
fn recording_agent_sink_captures_logs_and_snapshots() {
    let mut sink = RecordingAgentSink::default();
    sink.publish_log(log::Level::Info, "hello agent");
    sink.publish_snapshot(AgentVisibilitySnapshot {
        session_state: AgentSessionState::Starting,
        frame_time_ms: None,
        smoothed_frame_time_ms: None,
        mode: None,
        recent_log_count: 0,
    });

    assert_eq!(sink.logs().len(), 1);
    assert_eq!(sink.snapshots().len(), 1);
}

#[test]
fn onscreen_log_forwards_to_agent_sink_when_installed() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    set_agent_visibility_sink(Box::new(RecordingForwardingSink {
        captured: Arc::clone(&captured),
    }));

    crate::onscreen_info!("hello agent");

    let logs = match captured.lock() {
        Ok(logs) => logs.clone(),
        Err(_) => Vec::new(),
    };
    assert!(logs.iter().any(|line| line.contains("hello agent")));
    clear_agent_visibility_sink();
}

struct RecordingForwardingSink {
    captured: Arc<Mutex<Vec<String>>>,
}

impl super::AgentVisibilitySink for RecordingForwardingSink {
    fn publish_snapshot(&mut self, snapshot: AgentVisibilitySnapshot) {
        let _ = snapshot;
    }

    fn publish_log(&mut self, level: log::Level, message: &str) {
        if let Ok(mut captured) = self.captured.lock() {
            captured.push(format!("[{level}] {message}"));
        }
    }
}
