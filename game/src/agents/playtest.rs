use engine_core::agents::{AgentSessionManifest, AgentSessionTransport, AgentVisibilitySnapshot};
use std::fs::{self, File};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

/// File-backed transport for playtest-only agent session data.
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
        AgentSessionManifest, AgentSessionRole, AgentSessionState, AgentSessionTransport,
        AgentVisibilitySnapshot,
    };
    use engine_core::constants::agents;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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
            log_path: Some("/tmp/logs/playtest.log".to_string()),
        };
        let snapshot = AgentVisibilitySnapshot {
            session_state: AgentSessionState::Running,
            frame_time_ms: Some(16.7),
            smoothed_frame_time_ms: Some(15.2),
            mode: Some("playtest".to_string()),
            recent_log_count: 2,
        };

        assert!(transport.write_manifest(&manifest).is_ok());
        assert!(transport.write_snapshot(&snapshot).is_ok());

        assert!(transport.manifest_path().exists());
        assert!(transport.snapshot_path().exists());

        let _ = fs::remove_dir_all(session_dir);
    }
}
