use ron::Value;
use serde::{Deserialize, Serialize};

/// Builds an inline RON payload map from serializable key-value pairs.
#[macro_export]
macro_rules! payload {
    () => {{
        $crate::agents::payload_map_finish($crate::agents::payload_map_new())
    }};
    ($($key:ident : $value:expr),* $(,)?) => {{
        let mut map = $crate::agents::payload_map_new();
        $(
            $crate::agents::payload_map_insert(
                &mut map,
                stringify!($key).to_string(),
                $crate::agents::payload_value($value)
                    .expect("payload! values must be serializable"),
            );
        )*
        $crate::agents::payload_map_finish(map)
    }};
}

/// Errors returned while converting typed payload data into `ron::Value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentVisibilityPayloadError {
    /// Serialization into an intermediate value failed.
    Serialize(String),
    /// Conversion into a `ron::Value` failed.
    Parse(String),
}

/// High-level state of an agent-visible session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSessionState {
    /// Session is initializing.
    Starting,
    /// Session is actively running.
    Running,
    /// Session is shutting down.
    Stopping,
    /// Session is no longer running.
    Stopped,
}

/// High-level role of the session being observed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSessionRole {
    /// Session belongs to the editor.
    Editor,
    /// Session belongs to a playtest runtime.
    Playtest,
}

/// Agent request for extra payload fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentSnapshotRequest {
    /// Additional ad-hoc fields serialized into the request payload.
    pub extras: Value,
}

/// Generic runtime evidence used by agents for diagnosis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentVisibilitySnapshot {
    /// Current lifecycle state of the observed session.
    pub session_state: AgentSessionState,
    /// Most recent frame duration in milliseconds.
    pub frame_time_ms: Option<f32>,
    /// Smoothed frame duration in milliseconds.
    pub smoothed_frame_time_ms: Option<f32>,
    /// Active high-level mode label, if known.
    pub mode: Option<String>,
    /// Count of recent log lines included in the snapshot context.
    pub recent_log_count: usize,
    /// Current frame index, if available.
    pub frame_index: Option<u64>,
    /// Topic label describing the snapshot context.
    pub topic: Option<String>,
    /// Human-readable label for the snapshot.
    pub label: Option<String>,
    /// Additional structured payload data.
    pub payload: Option<Value>,
}

/// Converts typed payload data into `ron::Value` for agent-visible snapshots.
pub fn payload_value<T: Serialize>(payload: T) -> Result<Value, AgentVisibilityPayloadError> {
    let value = serde_value::to_value(payload)
        .map_err(|error| AgentVisibilityPayloadError::Serialize(error.to_string()))?;
    let value = Value::deserialize(value)
        .map_err(|error| AgentVisibilityPayloadError::Parse(error.to_string()))?;
    let ron = ron::to_string(&value)
        .map_err(|error| AgentVisibilityPayloadError::Serialize(error.to_string()))?;
    ron::from_str(&ron).map_err(|error| AgentVisibilityPayloadError::Parse(error.to_string()))
}

/// Internal helper for the exported `payload!` macro.
#[doc(hidden)]
pub fn payload_map_new() -> ron::Map {
    ron::Map::new()
}

/// Internal helper for the exported `payload!` macro.
#[doc(hidden)]
pub fn payload_map_insert(map: &mut ron::Map, key: String, value: Value) {
    map.insert(Value::String(key), value);
}

/// Internal helper for the exported `payload!` macro.
#[doc(hidden)]
pub fn payload_map_finish(map: ron::Map) -> Value {
    Value::Map(map)
}

fn merge_payload_values(profile_payload: Value, extras: Value) -> Value {
    match (profile_payload, extras) {
        (Value::Map(mut profile_map), Value::Map(extras_map)) => {
            for (key, value) in extras_map {
                profile_map.insert(key, value);
            }
            Value::Map(profile_map)
        }
        (profile_payload, _) => profile_payload,
    }
}

pub(crate) fn merged_snapshot_payload(
    request: &AgentSnapshotRequest,
    profile_payload: Value,
) -> Value {
    merge_payload_values(profile_payload, request.extras.clone())
}

/// Builds snapshot payload from the active request and current runtime payload.
pub fn build_snapshot_payload(
    request: &AgentSnapshotRequest,
    runtime_payload: Option<Value>,
) -> Option<Value> {
    match (runtime_payload, request.extras.clone()) {
        (Some(profile_payload), Value::Map(extras)) if extras.is_empty() => Some(profile_payload),
        (Some(profile_payload), extras) => Some(merged_snapshot_payload(
            &AgentSnapshotRequest { extras },
            profile_payload,
        )),
        (None, Value::Map(extras)) if extras.is_empty() => None,
        (None, Value::Map(extras)) => Some(Value::Map(extras)),
        (None, _) => None,
    }
}

/// Session metadata written alongside snapshots for discovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentSessionManifest {
    /// Unique identifier for the observed session.
    pub session_id: String,
    /// High-level role of the observed session.
    pub role: AgentSessionRole,
    /// Current lifecycle state of the session.
    pub state: AgentSessionState,
    /// Optional path to the latest snapshot payload.
    pub payload_path: Option<String>,
    /// Optional path to the session log output.
    pub log_path: Option<String>,
    /// Latest accepted snapshot request for this session.
    pub snapshot_request: Option<AgentSnapshotRequest>,
}

/// Sink for forwarding logs and snapshots to an agent-visible transport.
pub trait AgentVisibilitySink: Send {
    /// Publishes a snapshot to the sink.
    fn publish_snapshot(&mut self, snapshot: AgentVisibilitySnapshot);

    /// Publishes a log line to the sink.
    fn publish_log(&mut self, level: log::Level, message: &str);
}

/// Transport for session manifests and snapshots.
pub trait AgentSessionTransport {
    /// Writes session discovery metadata to the transport.
    fn write_manifest(&self, manifest: &AgentSessionManifest) -> std::io::Result<()>;

    /// Writes a snapshot to the transport.
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
