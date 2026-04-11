use serde::{Deserialize, Serialize};

/// High-level state of an agent-visible session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSessionState {
    Starting,
    Running,
    Stopping,
    Stopped,
}

/// High-level role of the session being observed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSessionRole {
    Editor,
    Playtest,
}

/// Minimal read-only snapshot used by agents for diagnosis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentVisibilitySnapshot {
    pub session_state: AgentSessionState,
    pub frame_time_ms: Option<f32>,
    pub smoothed_frame_time_ms: Option<f32>,
    pub mode: Option<String>,
    pub recent_log_count: usize,
}

/// Session metadata written alongside snapshots for discovery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSessionManifest {
    pub session_id: String,
    pub role: AgentSessionRole,
    pub state: AgentSessionState,
    pub payload_path: Option<String>,
    pub log_path: Option<String>,
}

/// Sink for forwarding logs and snapshots to an agent-visible transport.
pub trait AgentVisibilitySink: Send {
    fn publish_snapshot(&mut self, snapshot: AgentVisibilitySnapshot);

    fn publish_log(&mut self, level: log::Level, message: &str);
}

/// Transport for session manifests and snapshots.
pub trait AgentSessionTransport {
    fn write_manifest(&self, manifest: &AgentSessionManifest) -> std::io::Result<()>;

    fn write_snapshot(&self, snapshot: &AgentVisibilitySnapshot) -> std::io::Result<()>;
}

/// Simple in-memory sink used by tests and lightweight adapters.
#[derive(Default)]
pub struct RecordingAgentSink {
    logs: Vec<String>,
    snapshots: Vec<AgentVisibilitySnapshot>,
}

impl RecordingAgentSink {
    /// Returns the recorded log messages.
    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    /// Returns the recorded snapshots.
    pub fn snapshots(&self) -> &[AgentVisibilitySnapshot] {
        &self.snapshots
    }
}

impl AgentVisibilitySink for RecordingAgentSink {
    fn publish_snapshot(&mut self, snapshot: AgentVisibilitySnapshot) {
        self.snapshots.push(snapshot);
    }

    fn publish_log(&mut self, level: log::Level, message: &str) {
        self.logs.push(format!("[{level}] {message}"));
    }
}

#[cfg(test)]
mod tests;
