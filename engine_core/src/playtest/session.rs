use ron::Value;
use serde::{Deserialize, Serialize};

use crate::agents::AgentPlaytestControlRequest;

/// Errors returned while converting typed payload data into `ron::Value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaytestPayloadError {
    /// Serialization into an intermediate value failed.
    Serialize(String),
    /// Conversion into a `ron::Value` failed.
    Parse(String),
}

/// High-level state of a persisted playtest session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaytestSessionState {
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
pub enum PlaytestSessionRole {
    /// Session belongs to the editor.
    Editor,
    /// Session belongs to a playtest runtime.
    Playtest,
}

/// Request for extra serialized snapshot payload fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaytestSnapshotRequest {
    /// Additional ad-hoc fields serialized into the request payload.
    pub extras: Value,
}

impl Default for PlaytestSnapshotRequest {
    fn default() -> Self {
        Self {
            extras: Value::Map(ron::Map::new()),
        }
    }
}

/// Latest accepted runtime playtest control state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaytestActiveControl {
    /// Latest accepted runtime control request.
    pub request: AgentPlaytestControlRequest,
}

/// Generic persisted runtime evidence for playtest diagnosis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaytestSnapshot {
    /// Current lifecycle state of the observed session.
    pub session_state: PlaytestSessionState,
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

/// Converts typed payload data into `ron::Value` for persisted playtest snapshots.
pub fn payload_value<T: Serialize>(payload: T) -> Result<Value, PlaytestPayloadError> {
    let value = serde_value::to_value(payload)
        .map_err(|error| PlaytestPayloadError::Serialize(error.to_string()))?;
    let value = Value::deserialize(value)
        .map_err(|error| PlaytestPayloadError::Parse(error.to_string()))?;
    let ron = ron::to_string(&value)
        .map_err(|error| PlaytestPayloadError::Serialize(error.to_string()))?;
    ron::from_str(&ron).map_err(|error| PlaytestPayloadError::Parse(error.to_string()))
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

pub fn merged_snapshot_payload(request: &PlaytestSnapshotRequest, profile_payload: Value) -> Value {
    merge_payload_values(profile_payload, request.extras.clone())
}

/// Builds snapshot payload from the active request and current runtime payload.
pub fn build_snapshot_payload(
    request: &PlaytestSnapshotRequest,
    runtime_payload: Option<Value>,
) -> Option<Value> {
    match (runtime_payload, request.extras.clone()) {
        (Some(profile_payload), Value::Map(extras)) if extras.is_empty() => Some(profile_payload),
        (Some(profile_payload), extras) => Some(merged_snapshot_payload(
            &PlaytestSnapshotRequest { extras },
            profile_payload,
        )),
        (None, Value::Map(extras)) if extras.is_empty() => None,
        (None, Value::Map(extras)) => Some(Value::Map(extras)),
        (None, _) => None,
    }
}

/// Session metadata written alongside snapshots for discovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaytestSessionManifest {
    /// Unique identifier for the observed session.
    pub session_id: String,
    /// High-level role of the observed session.
    pub role: PlaytestSessionRole,
    /// Current lifecycle state of the session.
    pub state: PlaytestSessionState,
    /// Optional path to the latest snapshot payload.
    pub payload_path: Option<String>,
    /// Latest accepted snapshot request for this session.
    pub snapshot_request: Option<PlaytestSnapshotRequest>,
    /// Latest accepted runtime control state for this session.
    pub active_control: Option<PlaytestActiveControl>,
}

/// Transport for session manifests and snapshots.
pub trait PlaytestSessionTransport {
    /// Writes session discovery metadata to the transport.
    fn write_manifest(&self, manifest: &PlaytestSessionManifest) -> std::io::Result<()>;

    /// Writes a snapshot to the transport.
    fn write_snapshot(&self, snapshot: &PlaytestSnapshot) -> std::io::Result<()>;
}
