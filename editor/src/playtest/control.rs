use crate::agents::write_seeded_agent_payload;
use crate::app::Editor;
use crate::editor_global::push_toast;
use crate::playtest::playtest_process::PlaytestProcess;
use crate::playtest::room_playtest::{resolve_playtest_binary, write_playtest_payload};
use engine_core::agents::visibility::AgentSnapshotRequest;
use engine_core::constants::agents;
use engine_core::payload;
use engine_core::task::BackgroundTask;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Playtest launch mode requested through the editor automation boundary.
pub enum AgentPlaytestMode {
    /// Launches a room playtest attached to the editor-selected room.
    EditorAttachedCurrentRoom,
    /// Launches a seeded headless agent playtest for the editor-selected room.
    SeededAgentCurrentRoom {
        /// Optional snapshot request written into the seeded payload.
        snapshot_request: Option<AgentSnapshotRequest>,
    },
}

enum PreparedPlaytestLaunch {
    DeferredEditorBuild(BackgroundTask<Result<(PathBuf, PathBuf), String>>),
    EditorAttached {
        exe_path: PathBuf,
        payload_path: PathBuf,
    },
    SeededAgent {
        exe_path: PathBuf,
        args: [OsString; 3],
    },
}

/// Agent-facing control surface for playtest lifecycle actions.
pub trait AgentPlaytestControl {
    /// Opens a playtest for the current room and game state.
    fn request_open_playtest(&mut self, mode: AgentPlaytestMode) -> Result<(), String>;

    /// Closes any running playtest or pending launch owned by the editor.
    #[expect(dead_code, reason = "automation boundary kept for external callers")]
    fn request_close_playtest(&mut self) -> Result<(), String>;
}

impl AgentPlaytestControl for Editor {
    fn request_open_playtest(&mut self, mode: AgentPlaytestMode) -> Result<(), String> {
        self.launch_playtest(mode)
    }

    fn request_close_playtest(&mut self) -> Result<(), String> {
        self.close_playtest();
        Ok(())
    }
}

impl Editor {
    /// Opens playtest using the current room selection.
    pub fn open_playtest_for_current_room(&mut self) -> Result<(), String> {
        self.request_open_playtest(AgentPlaytestMode::EditorAttachedCurrentRoom)
    }

    /// Opens a headless agent playtest for the current room selection.
    pub fn open_agent_playtest_for_current_room(&mut self) -> Result<(), String> {
        self.request_open_playtest(AgentPlaytestMode::SeededAgentCurrentRoom {
            snapshot_request: Some(self.default_seeded_agent_snapshot_request()),
        })
    }

    fn launch_playtest(&mut self, mode: AgentPlaytestMode) -> Result<(), String> {
        if self.pending_playtest_build.is_some() {
            return Err("Playtest build already in progress.".to_string());
        }

        let launch = self.prepare_playtest_launch(mode)?;
        self.clear_running_playtest_process();

        match launch {
            PreparedPlaytestLaunch::DeferredEditorBuild(task) => {
                push_toast("Building playtest...", 30.0);
                self.pending_playtest_build = Some(task);
                Ok(())
            }
            PreparedPlaytestLaunch::EditorAttached {
                exe_path,
                payload_path,
            } => {
                let process = PlaytestProcess::spawn(&exe_path, &payload_path)
                    .map_err(|error| format!("Failed to launch playtest: {error}"))?;
                self.replace_playtest_process(process);
                Ok(())
            }
            PreparedPlaytestLaunch::SeededAgent { exe_path, args } => {
                let arg_refs: [&OsStr; 3] = [
                    args[0].as_os_str(),
                    args[1].as_os_str(),
                    args[2].as_os_str(),
                ];
                let process = PlaytestProcess::spawn_with_args(&exe_path, &arg_refs)
                    .map_err(|error| format!("Failed to launch playtest: {error}"))?;
                self.replace_playtest_process(process);
                Ok(())
            }
        }
    }

    fn prepare_playtest_launch(
        &self,
        mode: AgentPlaytestMode,
    ) -> Result<PreparedPlaytestLaunch, String> {
        let room_id = self
            .cur_room_id
            .ok_or_else(|| "No room is currently selected.".to_string())?;

        match mode {
            AgentPlaytestMode::EditorAttachedCurrentRoom => {
                let room = self.get_room_from_id(&room_id);
                let payload_path = write_playtest_payload(room, &self.game)
                    .map_err(|error| format!("Could not write playtest payload: {error}"))?;

                if cfg!(debug_assertions) {
                    return Ok(PreparedPlaytestLaunch::DeferredEditorBuild(
                        BackgroundTask::spawn(move || {
                            resolve_playtest_binary()
                                .map(|exe_path| (exe_path, payload_path))
                                .map_err(|error| error.to_string())
                        }),
                    ));
                }

                let exe_path = resolve_playtest_binary().map_err(|error| error.to_string())?;
                Ok(PreparedPlaytestLaunch::EditorAttached {
                    exe_path,
                    payload_path,
                })
            }
            AgentPlaytestMode::SeededAgentCurrentRoom { snapshot_request } => {
                let payload_path =
                    self.write_seeded_agent_playtest_payload(room_id, snapshot_request)?;
                let exe_path = resolve_playtest_binary().map_err(|error| error.to_string())?;
                let args = [
                    OsString::from(agents::HEADLESS_FLAG),
                    OsString::from(agents::PAYLOAD_FLAG),
                    payload_path.as_os_str().to_os_string(),
                ];
                Ok(PreparedPlaytestLaunch::SeededAgent { exe_path, args })
            }
        }
    }

    fn write_seeded_agent_playtest_payload(
        &self,
        room_id: engine_core::prelude::RoomId,
        snapshot_request: Option<AgentSnapshotRequest>,
    ) -> Result<std::path::PathBuf, String> {
        write_seeded_agent_payload(self, room_id, snapshot_request)
            .map_err(|error| format!("Could not write agent payload: {error:?}"))
    }

    /// Builds the default snapshot request used for seeded agent playtest launches.
    fn default_seeded_agent_snapshot_request(&self) -> AgentSnapshotRequest {
        AgentSnapshotRequest { extras: payload!() }
    }

    fn replace_playtest_process(&mut self, process: PlaytestProcess) {
        self.playtest_process = Some(process);
    }

    fn clear_running_playtest_process(&mut self) {
        if let Some(mut playtest_process) = self.playtest_process.take() {
            playtest_process.kill();
        }
    }

    pub(crate) fn complete_pending_playtest_build(
        &mut self,
        result: Result<(PathBuf, PathBuf), String>,
    ) -> Result<(), String> {
        self.pending_playtest_build = None;

        let (exe_path, payload_path) = result?;
        let process = PlaytestProcess::spawn(&exe_path, &payload_path)
            .map_err(|error| format!("Failed to launch playtest: {error}"))?;
        self.replace_playtest_process(process);
        Ok(())
    }

    /// Stops any running playtest process and clears pending build state.
    pub fn close_playtest(&mut self) {
        self.pending_playtest_build = None;
        if let Some(ref mut playtest_process) = self.playtest_process {
            playtest_process.kill();
        }
        self.playtest_process = None;
    }
}

#[cfg(test)]
mod agent_playtest_control_tests {
    use super::{AgentPlaytestControl, AgentPlaytestMode};
    use crate::agents::test_helpers::{seeded_agent_session_dir, seeded_editor_fixture};
    use crate::agents::write_seeded_agent_payload;
    use crate::app::Editor;
    use crate::playtest::room_playtest::resolve_playtest_binary;
    use engine_core::agents::visibility::AgentSnapshotRequest;
    use engine_core::agents::{AgentSessionManifest, AgentVisibilitySnapshot};
    use engine_core::constants::agents;
    use engine_core::engine_global::set_game_name;
    use engine_core::payload;
    use engine_core::prelude::{BackgroundTask, RoomId};
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    fn read_manifest(path: &std::path::Path) -> Option<AgentSessionManifest> {
        let ron = fs::read_to_string(path).ok()?;
        ron::from_str(&ron).ok()
    }

    fn read_snapshot(path: &std::path::Path) -> Option<AgentVisibilitySnapshot> {
        let ron = fs::read_to_string(path).ok()?;
        ron::from_str(&ron).ok()
    }

    fn snapshot_number(snapshot: &AgentVisibilitySnapshot, key: &str) -> Option<f64> {
        let ron::Value::Map(map) = snapshot.payload.as_ref()? else {
            return None;
        };
        let ron::Value::Number(number) = map.get(&ron::Value::String(key.to_string()))? else {
            return None;
        };
        Some(number.into_f64())
    }

    #[test]
    fn agent_playtest_command_closes_cleanly_when_idle() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_control");
        set_game_name(test_game.name());

        let mut editor = Editor::default();

        editor.close_playtest();
        assert!(editor
            .request_open_playtest(AgentPlaytestMode::EditorAttachedCurrentRoom)
            .is_err());
    }

    #[test]
    fn request_open_playtest_rejects_missing_room_for_editor_attached_mode() {
        let mut editor = Editor::default();

        assert!(editor
            .request_open_playtest(AgentPlaytestMode::EditorAttachedCurrentRoom)
            .is_err());
    }

    #[test]
    fn request_open_playtest_rejects_missing_room_for_seeded_agent_mode() {
        let mut editor = Editor::default();

        assert!(editor
            .request_open_playtest(AgentPlaytestMode::SeededAgentCurrentRoom {
                snapshot_request: None,
            })
            .is_err());
    }

    #[test]
    fn close_playtest_clears_pending_build_state() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_control_pending");
        set_game_name(test_game.name());

        let mut editor = Editor {
            pending_playtest_build: Some(BackgroundTask::spawn(|| {
                Ok((PathBuf::new(), PathBuf::new()))
            })),
            ..Default::default()
        };

        editor.close_playtest();

        assert!(editor.pending_playtest_build.is_none());
    }

    #[test]
    fn headless_agent_playtest_command_requires_room_selection() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_headless_playtest_control");
        set_game_name(test_game.name());

        let mut editor = Editor::default();

        assert!(editor
            .request_open_playtest(AgentPlaytestMode::SeededAgentCurrentRoom {
                snapshot_request: None,
            })
            .is_err());
    }

    #[test]
    fn seeded_agent_playtest_payload_writer_uses_default_snapshot_request() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_headless_playtest_default_snapshot_request");
        let editor = seeded_editor_fixture(&test_game, RoomId(52));

        let payload_path = editor
            .write_seeded_agent_playtest_payload(
                RoomId(52),
                Some(editor.default_seeded_agent_snapshot_request()),
            )
            .unwrap();
        let loaded = game_lib::agents::load_agent_payload(&payload_path).unwrap();
        let expected_request = AgentSnapshotRequest { extras: payload!() };

        assert_eq!(loaded.snapshot_request, Some(expected_request));

        let _ = fs::remove_file(payload_path);
    }

    #[test]
    fn seeded_agent_playtest_accepts_runtime_snapshot_request_from_session_dir() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_runtime_snapshot_request");
        let editor = seeded_editor_fixture(&test_game, RoomId(53));
        let default_request = AgentSnapshotRequest { extras: payload!() };
        let override_request = AgentSnapshotRequest {
            extras: payload!(accumulator_ms: 123.0),
        };

        let payload_path = editor
            .write_seeded_agent_playtest_payload(
                RoomId(53),
                Some(editor.default_seeded_agent_snapshot_request()),
            )
            .unwrap();
        let exe_path = resolve_playtest_binary().unwrap();
        let args = [
            OsString::from(agents::HEADLESS_FLAG),
            OsString::from(agents::PAYLOAD_FLAG),
            payload_path.as_os_str().to_os_string(),
        ];
        let session_dir = seeded_agent_session_dir(&payload_path);
        let manifest_path = session_dir.join("agent-session.ron");
        let snapshot_path = session_dir.join("agent-snapshot.ron");
        let request_path = session_dir.join(agents::REQUEST_FILENAME);
        let log_path = PathBuf::from(agents::PLAYTEST_LOG_PATH);

        let _ = fs::remove_dir_all(&session_dir);
        let _ = fs::remove_file(&log_path);

        let mut child = Command::new(&exe_path)
            .args([
                args[0].as_os_str(),
                args[1].as_os_str(),
                args[2].as_os_str(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20) {
            let manifest = read_manifest(&manifest_path);
            let snapshot = read_snapshot(&snapshot_path);
            if manifest
                .as_ref()
                .is_some_and(|manifest| manifest.snapshot_request == Some(default_request.clone()))
                && snapshot.is_some()
            {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }

        assert!(
            manifest_path.exists(),
            "manifest missing: {}",
            manifest_path.display()
        );
        assert!(
            snapshot_path.exists(),
            "snapshot missing: {}",
            snapshot_path.display()
        );

        let initial_snapshot = read_snapshot(&snapshot_path).unwrap();
        assert_ne!(
            snapshot_number(&initial_snapshot, "accumulator_ms"),
            Some(123.0)
        );

        fs::write(
            &request_path,
            ron::ser::to_string_pretty(&override_request, ron::ser::PrettyConfig::default())
                .unwrap(),
        )
        .unwrap();

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20) {
            let manifest = read_manifest(&manifest_path);
            let snapshot = read_snapshot(&snapshot_path);
            if manifest
                .as_ref()
                .is_some_and(|manifest| manifest.snapshot_request == Some(override_request.clone()))
                && snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot_number(snapshot, "accumulator_ms"))
                    .is_some_and(|value| (value - 123.0).abs() < f64::EPSILON)
            {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }

        let _ = child.kill();
        let _ = child.wait();

        let manifest = read_manifest(&manifest_path).unwrap();
        let snapshot = read_snapshot(&snapshot_path).unwrap();
        assert_eq!(manifest.snapshot_request, Some(override_request));
        assert!(snapshot_number(&snapshot, "accumulator_ms")
            .is_some_and(|value| (value - 123.0).abs() < f64::EPSILON));
        assert!(!request_path.exists(), "request file was not consumed");

        let _ = fs::remove_file(&payload_path);
        let _ = fs::remove_dir_all(session_dir);
        let _ = fs::remove_file(log_path);
    }

    #[test]
    fn seeded_agent_playtest_writes_manifest_log_and_snapshot_from_same_run() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_end_to_end");
        let editor = seeded_editor_fixture(&test_game, RoomId(51));
        let room_id = editor.cur_room_id.unwrap();
        let payload_path = write_seeded_agent_payload(&editor, room_id, None).unwrap();
        let exe_path = resolve_playtest_binary().unwrap();
        let args = [
            OsString::from(agents::HEADLESS_FLAG),
            OsString::from(agents::PAYLOAD_FLAG),
            payload_path.as_os_str().to_os_string(),
        ];
        let session_dir = seeded_agent_session_dir(&payload_path);
        let manifest_path = session_dir.join("agent-session.ron");
        let snapshot_path = session_dir.join("agent-snapshot.ron");
        let log_path = PathBuf::from(agents::PLAYTEST_LOG_PATH);

        let _ = fs::remove_dir_all(&session_dir);
        let _ = fs::remove_file(&log_path);
        let launched_at = std::time::SystemTime::now();

        let mut child = Command::new(&exe_path)
            .args([
                args[0].as_os_str(),
                args[1].as_os_str(),
                args[2].as_os_str(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20)
            && (!manifest_path.exists() || !snapshot_path.exists() || !log_path.exists())
        {
            thread::sleep(Duration::from_millis(200));
        }

        let _ = child.kill();
        let _ = child.wait();

        assert!(
            manifest_path.exists(),
            "manifest missing: {}",
            manifest_path.display()
        );
        assert!(
            snapshot_path.exists(),
            "snapshot missing: {}",
            snapshot_path.display()
        );
        assert!(log_path.exists(), "log missing: {}", log_path.display());

        let manifest = fs::read_to_string(&manifest_path).unwrap();
        let snapshot = fs::read_to_string(&snapshot_path).unwrap();
        let log = fs::read_to_string(&log_path).unwrap();

        assert!(manifest.contains(payload_path.to_string_lossy().as_ref()));
        assert!(manifest.contains(agents::PLAYTEST_LOG_PATH));
        assert!(snapshot.contains(agents::PLAYTEST_RUNTIME_TOPIC));
        assert!(snapshot.contains(agents::PLAYTEST_FRAME_LABEL));
        assert!(!log.trim().is_empty());
        assert!(
            fs::metadata(&log_path)
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(launched_at)
                .is_ok(),
            "log was not refreshed by launched run: {}",
            log_path.display()
        );

        let _ = fs::remove_file(&payload_path);
        let _ = fs::remove_dir_all(session_dir);
        let _ = fs::remove_file(log_path);
    }
}
