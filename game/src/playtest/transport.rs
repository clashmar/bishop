use engine_core::constants::agents;
use engine_core::playtest::{PlaytestSessionManifest, PlaytestSessionTransport, PlaytestSnapshot};
use std::fs::{self, File};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// File-backed transport for playtest session data.
///
/// Manifest, snapshot, request, control-request, and expanded-control files live
/// under the session directory.
#[derive(Clone)]
pub struct FilePlaytestSessionTransport {
    session_dir: PathBuf,
}

impl FilePlaytestSessionTransport {
    /// Creates a new transport rooted at `session_dir`.
    pub fn new(session_dir: PathBuf) -> Self {
        Self { session_dir }
    }

    /// Returns the session-rooted manifest file path.
    pub fn manifest_path(&self) -> PathBuf {
        self.session_dir.join(agents::MANIFEST_FILENAME)
    }

    /// Returns the session-rooted snapshot file path.
    pub fn snapshot_path(&self) -> PathBuf {
        self.session_dir.join(agents::SNAPSHOT_FILENAME)
    }

    /// Returns the session-rooted runtime request inbox path.
    pub fn request_path(&self) -> PathBuf {
        self.session_dir.join(agents::REQUEST_FILENAME)
    }

    /// Returns the session-rooted runtime control request inbox path.
    pub fn control_request_path(&self) -> PathBuf {
        self.session_dir.join(agents::CONTROL_REQUEST_FILENAME)
    }

    /// Returns the session-rooted persisted expanded control timeline path.
    pub fn expanded_control_path(&self) -> PathBuf {
        self.session_dir.join(agents::EXPANDED_CONTROL_FILENAME)
    }

    /// Ensures the transport directory exists.
    pub fn ensure_ready(&self) -> io::Result<()> {
        fs::create_dir_all(&self.session_dir)
    }
}

impl PlaytestSessionTransport for FilePlaytestSessionTransport {
    fn write_manifest(&self, manifest: &PlaytestSessionManifest) -> io::Result<()> {
        self.ensure_ready()?;
        write_ron_file(&self.manifest_path(), manifest)
    }

    fn write_snapshot(&self, snapshot: &PlaytestSnapshot) -> io::Result<()> {
        self.ensure_ready()?;
        write_ron_file(&self.snapshot_path(), snapshot)
    }
}

fn write_ron_file<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let ron = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|error| io::Error::other(format!("Could not serialize playtest data: {error}")))?;
    let temp_path = temporary_ron_path(path);
    let mut file = File::options()
        .create_new(true)
        .write(true)
        .open(&temp_path)?;
    file.write_all(ron.as_bytes())?;
    file.sync_all()?;
    drop(file);

    if fs::rename(&temp_path, path).is_ok() {
        return Ok(());
    }

    if !path.exists() {
        let _ = fs::remove_file(&temp_path);
        return Err(io::Error::other(format!(
            "Could not publish playtest data to {}",
            path.display()
        )));
    }

    let backup_path = backup_ron_path(path);
    fs::rename(path, &backup_path)?;

    if let Err(error) = fs::rename(&temp_path, path) {
        let restore_result = fs::rename(&backup_path, path);
        let _ = fs::remove_file(&temp_path);

        return match restore_result {
            Ok(()) => Err(error),
            Err(restore_error) => Err(io::Error::other(format!(
                "Could not publish playtest data to {}: {error}; restore failed: {restore_error}",
                path.display()
            ))),
        };
    }

    if let Err(error) = fs::remove_file(&backup_path) {
        if path.exists() {
            return Err(error);
        }
    }

    Ok(())
}

fn temporary_ron_path(path: &Path) -> PathBuf {
    sibling_ron_path(path, "tmp")
}

fn backup_ron_path(path: &Path) -> PathBuf {
    sibling_ron_path(path, "bak")
}

fn sibling_ron_path(path: &Path, suffix: &str) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "agent-data.ron".to_string());

    parent.join(format!(".{}.{}.{}", file_name, Uuid::new_v4(), suffix))
}

#[cfg(test)]
mod tests {
    use super::FilePlaytestSessionTransport;
    use engine_core::constants::{agents, PLAYTEST_PAYLOAD_RON};
    use engine_core::playtest::{
        payload_value, PlaytestSessionManifest, PlaytestSessionRole, PlaytestSessionState,
        PlaytestSessionTransport, PlaytestSnapshot,
    };
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
    fn file_playtest_transport_writes_manifest_and_snapshot() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let session_dir = std::env::temp_dir().join(format!("playtest_transport_{unique}"));
        let _ = fs::remove_dir_all(&session_dir);

        let transport = FilePlaytestSessionTransport::new(PathBuf::from(&session_dir));
        let manifest = PlaytestSessionManifest {
            session_id: "session-1".to_string(),
            role: PlaytestSessionRole::Playtest,
            state: PlaytestSessionState::Starting,
            payload_path: Some(format!("/tmp/{PLAYTEST_PAYLOAD_RON}")),
            snapshot_request: None,
            active_control: None,
        };
        let updated_manifest = PlaytestSessionManifest {
            state: PlaytestSessionState::Running,
            ..manifest.clone()
        };
        let snapshot = PlaytestSnapshot {
            session_state: PlaytestSessionState::Running,
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
        let updated_snapshot = PlaytestSnapshot {
            frame_index: Some(1),
            payload: payload_value(TestRuntimePayload {
                accumulator_ms: 33.4,
                player_velocity_x: 4.0,
            })
            .ok(),
            ..snapshot.clone()
        };

        assert!(transport.write_manifest(&manifest).is_ok());
        assert!(transport.write_snapshot(&snapshot).is_ok());
        assert!(transport.write_manifest(&updated_manifest).is_ok());
        assert!(transport.write_snapshot(&updated_snapshot).is_ok());

        assert_eq!(
            transport.manifest_path(),
            session_dir.join(agents::MANIFEST_FILENAME)
        );
        assert_eq!(
            transport.snapshot_path(),
            session_dir.join(agents::SNAPSHOT_FILENAME)
        );
        assert!(transport.manifest_path().exists());
        assert!(transport.snapshot_path().exists());

        let manifest_ron = fs::read_to_string(transport.manifest_path()).unwrap();
        let snapshot_ron = fs::read_to_string(transport.snapshot_path()).unwrap();
        assert!(manifest_ron.contains("Running"));
        assert!(snapshot_ron.contains("accumulator_ms"));
        assert!(snapshot_ron.contains("player_velocity_x"));
        assert!(snapshot_ron.contains("33.4"));
        assert!(snapshot_ron.contains("4.0"));

        let _ = fs::remove_dir_all(session_dir);
    }

    #[test]
    fn file_playtest_transport_exposes_request_inbox_path() {
        let session_dir = PathBuf::from("/tmp/playtest_transport_request_path");
        let transport = FilePlaytestSessionTransport::new(session_dir.clone());

        assert_eq!(
            transport.request_path(),
            session_dir.join(agents::REQUEST_FILENAME)
        );
    }

    #[test]
    fn file_playtest_transport_exposes_control_request_path() {
        let session_dir = PathBuf::from("/tmp/playtest_transport_control_request_path");
        let transport = FilePlaytestSessionTransport::new(session_dir.clone());

        assert_eq!(
            transport.control_request_path(),
            session_dir.join(agents::CONTROL_REQUEST_FILENAME)
        );
    }

    #[test]
    fn file_playtest_transport_exposes_expanded_control_path() {
        let session_dir = PathBuf::from("/tmp/playtest_transport_control_timeline_path");
        let transport = FilePlaytestSessionTransport::new(session_dir.clone());

        assert_eq!(
            transport.expanded_control_path(),
            session_dir.join(agents::EXPANDED_CONTROL_FILENAME)
        );
    }
}
