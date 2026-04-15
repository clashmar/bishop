use super::*;
use engine_core::constants::playtest_artifacts;
use engine_core::playtest::{
    PlaytestSessionManifest, PlaytestSessionRole, PlaytestSessionState, PlaytestSnapshot,
};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

struct CleanupFixture {
    root_dir: PathBuf,
    payload_path: PathBuf,
    session_dir: PathBuf,
}

impl CleanupFixture {
    fn new() -> Self {
        let root_dir =
            std::env::temp_dir().join(format!("playtest-flight-test-{}", Uuid::new_v4()));
        let payload_path = root_dir.join("playtest_case.ron");
        let session_dir = session_dir_for_payload(&payload_path);

        fs::create_dir_all(&session_dir).unwrap();
        fs::write(&payload_path, "payload").unwrap();
        fs::write(
            session_dir.join(playtest_artifacts::SESSION_FILENAME),
            "session",
        )
        .unwrap();
        fs::write(
            session_dir.join(playtest_artifacts::SNAPSHOT_FILENAME),
            "snapshot",
        )
        .unwrap();
        fs::write(
            session_dir.join(format!(".{}.tmp", playtest_artifacts::SNAPSHOT_FILENAME)),
            "tmp",
        )
        .unwrap();

        Self {
            root_dir,
            payload_path,
            session_dir,
        }
    }
}

impl Drop for CleanupFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root_dir);
    }
}

fn path_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}

#[test]
fn cleanup_playtest_artifacts_removes_payload_session_log_and_empty_log_dir() {
    let fixture = CleanupFixture::new();

    let outcome = cleanup_playtest_artifacts(Some(&fixture.session_dir));

    assert!(outcome.payload_removed);
    assert!(outcome.session_dir_removed);
    assert!(!path_exists(&fixture.payload_path));
    assert!(!path_exists(&fixture.session_dir));
}

#[test]
fn session_dir_can_be_derived_from_game_playtest_command_line() {
    let session_dir = PathBuf::from(format!(
        "/tmp/playtest_example{}",
        playtest_artifacts::SESSION_DIR_SUFFIX
    ));
    let payload_path = payload_path_for_session_dir(&session_dir).unwrap();
    let command = format!("12345 /tmp/game-playtest {}", payload_path.display());

    let derived = session_dir_from_pgrep_line(&command);

    assert_eq!(derived, Some(session_dir));
}

#[test]
fn runtime_snapshot_payload_view_deserializes_direct_camera_fields() {
    let payload = ron::Value::Map(ron::Map::from_iter(vec![
        (
            ron::Value::String("camera_target".to_string()),
            ron::Value::Seq(vec![
                ron::Value::Number(15.0.into()),
                ron::Value::Number(18.0.into()),
            ]),
        ),
        (
            ron::Value::String("camera_zoom".to_string()),
            ron::Value::Seq(vec![
                ron::Value::Number(1.25.into()),
                ron::Value::Number(1.25.into()),
            ]),
        ),
        (
            ron::Value::String("camera_follow_enabled".to_string()),
            ron::Value::Bool(false),
        ),
        (
            ron::Value::String("camera_override_active".to_string()),
            ron::Value::Bool(true),
        ),
    ]));

    let view = decode_snapshot_payload(&payload).unwrap();

    assert_eq!(view.camera_target, Some((15.0, 18.0)));
    assert_eq!(view.camera_zoom, Some((1.25, 1.25)));
    assert_eq!(view.camera_follow_enabled, Some(false));
    assert_eq!(view.camera_override_active, Some(true));
}

#[test]
fn read_snapshot_summary_exposes_direct_camera_fields() {
    let fixture = CleanupFixture::new();
    let manifest = PlaytestSessionManifest {
        session_id: "session-1".to_string(),
        role: PlaytestSessionRole::Playtest,
        state: PlaytestSessionState::Running,
        payload_path: Some(fixture.payload_path.display().to_string()),
        snapshot_request: None,
        active_control: None,
    };
    let snapshot = PlaytestSnapshot {
        session_state: PlaytestSessionState::Running,
        frame_time_ms: Some(16.7),
        smoothed_frame_time_ms: Some(16.7),
        mode: Some("playtest".to_string()),
        recent_log_count: 0,
        frame_index: Some(42),
        topic: Some("playtest-runtime".to_string()),
        label: Some("frame".to_string()),
        payload: Some(ron::Value::Map(ron::Map::from_iter(vec![
            (
                ron::Value::String("camera_target".to_string()),
                ron::Value::Seq(vec![
                    ron::Value::Number(15.0_f32.into()),
                    ron::Value::Number(18.0_f32.into()),
                ]),
            ),
            (
                ron::Value::String("camera_zoom".to_string()),
                ron::Value::Seq(vec![
                    ron::Value::Number(1.25_f32.into()),
                    ron::Value::Number(1.25_f32.into()),
                ]),
            ),
            (
                ron::Value::String("camera_follow_enabled".to_string()),
                ron::Value::Bool(false),
            ),
            (
                ron::Value::String("camera_override_active".to_string()),
                ron::Value::Bool(true),
            ),
        ]))),
    };

    fs::write(
        fixture
            .session_dir
            .join(playtest_artifacts::SESSION_FILENAME),
        ron::to_string(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        fixture
            .session_dir
            .join(playtest_artifacts::SNAPSHOT_FILENAME),
        ron::to_string(&snapshot).unwrap(),
    )
    .unwrap();

    let summary = read_snapshot_summary(&fixture.session_dir).unwrap();

    assert_eq!(summary.frame_index, Some(42));
    assert_eq!(summary.camera_target, Some((15.0, 18.0)));
    assert_eq!(summary.camera_zoom, Some((1.25, 1.25)));
    assert_eq!(summary.camera_follow_enabled, Some(false));
    assert_eq!(summary.camera_override_active, Some(true));
}
