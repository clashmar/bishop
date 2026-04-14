use engine_core::agents::{
    AgentSessionManifest, AgentSessionTransport, AgentVisibilitySink, AgentVisibilitySnapshot,
};
use engine_core::constants::agents;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

/// File-backed transport for playtest-only agent session data.
#[derive(Clone)]
pub struct FileAgentSessionTransport {
    session_dir: PathBuf,
}

impl FileAgentSessionTransport {
    /// Creates a new transport rooted at `session_dir`.
    pub fn new(session_dir: PathBuf) -> Self {
        Self { session_dir }
    }

    /// Returns the manifest file path.
    pub fn manifest_path(&self) -> PathBuf {
        self.session_dir.join("agent-session.ron")
    }

    /// Returns the snapshot file path.
    pub fn snapshot_path(&self) -> PathBuf {
        self.session_dir.join("agent-snapshot.ron")
    }

    /// Returns the runtime request inbox path.
    pub fn request_path(&self) -> PathBuf {
        self.session_dir.join(agents::REQUEST_FILENAME)
    }

    /// Ensures the transport directory exists.
    pub fn ensure_ready(&self) -> io::Result<()> {
        fs::create_dir_all(&self.session_dir)
    }
}

impl AgentSessionTransport for FileAgentSessionTransport {
    fn write_manifest(&self, manifest: &AgentSessionManifest) -> io::Result<()> {
        self.ensure_ready()?;
        write_ron_file(&self.manifest_path(), manifest)
    }

    fn write_snapshot(&self, snapshot: &AgentVisibilitySnapshot) -> io::Result<()> {
        self.ensure_ready()?;
        write_ron_file(&self.snapshot_path(), snapshot)
    }
}

impl AgentVisibilitySink for FileAgentSessionTransport {
    fn publish_snapshot(&mut self, snapshot: AgentVisibilitySnapshot) {
        if let Err(error) = self.ensure_ready() {
            eprintln!("Failed to prepare agent snapshot directory: {error}");
            return;
        }

        if let Err(error) = self.write_snapshot(&snapshot) {
            eprintln!("Failed to write agent snapshot: {error}");
        }
    }

    fn publish_log(&mut self, level: log::Level, message: &str) {
        let log_path = PathBuf::from(agents::PLAYTEST_LOG_PATH);
        if let Some(parent) = log_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!("Failed to prepare agent log directory: {error}");
                return;
            }
        }

        let entry = format!("[{level}] {message}\n");
        match OpenOptions::new().create(true).append(true).open(&log_path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(entry.as_bytes()) {
                    eprintln!("Failed to write agent log: {error}");
                }
            }
            Err(error) => eprintln!("Failed to open agent log: {error}"),
        }
    }
}

fn write_ron_file<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let ron = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|error| io::Error::other(format!("Could not serialize agent data: {error}")))?;
    let mut file = File::create(path)?;
    file.write_all(ron.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::FileAgentSessionTransport;
    use engine_core::agents::{
        payload_value, AgentSessionManifest, AgentSessionRole, AgentSessionState,
        AgentSessionTransport, AgentVisibilitySnapshot,
    };
    use engine_core::constants::agents;
    use serde::Serialize;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Serialize)]
    struct TestRuntimePayload {
        accumulator_ms: f32,
        player_velocity_x: f32,
    }

    #[test]
    fn file_agent_transport_writes_manifest_and_snapshot() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let session_dir = std::env::temp_dir().join(format!("agent_transport_{unique}"));
        let _ = fs::remove_dir_all(&session_dir);

        let transport = FileAgentSessionTransport::new(PathBuf::from(&session_dir));
        let manifest = AgentSessionManifest {
            session_id: "session-1".to_string(),
            role: AgentSessionRole::Playtest,
            state: AgentSessionState::Starting,
            payload_path: Some(format!("/tmp/{}", agents::PAYLOAD_FILENAME)),
            log_path: Some(agents::PLAYTEST_LOG_PATH.to_string()),
            snapshot_request: None,
        };
        let snapshot = AgentVisibilitySnapshot {
            session_state: AgentSessionState::Running,
            frame_time_ms: Some(16.7),
            smoothed_frame_time_ms: Some(15.2),
            mode: Some(agents::PLAYTEST_MODE.to_string()),
            recent_log_count: 2,
            frame_index: Some(0),
            topic: Some(agents::PLAYTEST_RUNTIME_TOPIC.to_string()),
            label: Some(agents::PLAYTEST_FRAME_LABEL.to_string()),
            payload: payload_value(TestRuntimePayload {
                accumulator_ms: 16.7,
                player_velocity_x: 0.0,
            })
            .ok(),
        };

        assert!(transport.write_manifest(&manifest).is_ok());
        assert!(transport.write_snapshot(&snapshot).is_ok());

        assert!(transport.manifest_path().exists());
        assert!(transport.snapshot_path().exists());

        let snapshot_ron = fs::read_to_string(transport.snapshot_path()).unwrap();
        assert!(snapshot_ron.contains("accumulator_ms"));
        assert!(snapshot_ron.contains("player_velocity_x"));

        let _ = fs::remove_dir_all(session_dir);
    }

    #[test]
    fn file_agent_transport_exposes_request_inbox_path() {
        let session_dir = PathBuf::from("/tmp/agent_transport_request_path");
        let transport = FileAgentSessionTransport::new(session_dir.clone());

        assert_eq!(
            transport.request_path(),
            session_dir.join(agents::REQUEST_FILENAME)
        );
    }
}
