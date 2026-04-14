use crate::agents::{build_seeded_agent_playtest_launch, write_seeded_agent_payload};
use crate::app::Editor;
use crate::editor_global::push_toast;
use crate::playtest::playtest_process::PlaytestProcess;
use crate::playtest::room_playtest::{resolve_playtest_binary, write_playtest_payload};
use engine_core::agents::visibility::{AgentSnapshotRequest, SnapshotProfile};
use engine_core::payload;
use engine_core::task::BackgroundTask;

/// Agent-facing control surface for playtest lifecycle actions.
#[allow(dead_code)]
pub trait AgentPlaytestControl {
    /// Opens a playtest for the current room and game state.
    fn request_open_playtest(&mut self) -> Result<(), String>;

    /// Closes any running playtest.
    fn request_close_playtest(&mut self) -> Result<(), String>;
}

impl AgentPlaytestControl for Editor {
    fn request_open_playtest(&mut self) -> Result<(), String> {
        self.open_playtest_for_current_room()
    }

    fn request_close_playtest(&mut self) -> Result<(), String> {
        self.close_playtest();
        Ok(())
    }
}

impl Editor {
    /// Opens playtest using the current room selection.
    pub fn open_playtest_for_current_room(&mut self) -> Result<(), String> {
        if self.pending_playtest_build.is_some() {
            return Err("Playtest build already in progress.".to_string());
        }

        let room_id = self
            .cur_room_id
            .ok_or_else(|| "No room is currently selected.".to_string())?;

        let room = self.get_room_from_id(&room_id);
        let payload_path = write_playtest_payload(room, &self.game)
            .map_err(|error| format!("Could not write playtest payload: {error}"))?;

        if cfg!(debug_assertions) {
            push_toast("Building playtest...", 30.0);
            self.pending_playtest_build = Some(BackgroundTask::spawn(move || {
                resolve_playtest_binary()
                    .map(|exe_path| (exe_path, payload_path))
                    .map_err(|error| error.to_string())
            }));
            return Ok(());
        }

        let exe_path = resolve_playtest_binary().map_err(|error| error.to_string())?;
        if let Some(ref mut old_process) = self.playtest_process {
            old_process.kill();
        }

        self.playtest_process = Some(
            PlaytestProcess::spawn(&exe_path, &payload_path)
                .map_err(|error| format!("Failed to launch playtest: {error}"))?,
        );
        Ok(())
    }

    /// Opens a headless agent playtest for the current room selection.
    pub fn open_agent_playtest_for_current_room(&mut self) -> Result<(), String> {
        if self.pending_playtest_build.is_some() {
            return Err("Playtest build already in progress.".to_string());
        }
        let payload_path = self.write_seeded_agent_playtest_payload_for_current_room()?;
        let launch = build_seeded_agent_playtest_launch(payload_path)?;
        if let Some(ref mut old_process) = self.playtest_process {
            old_process.kill();
        }

        self.playtest_process = Some(
            PlaytestProcess::spawn_with_args(&launch.exe_path, &launch.arg_refs())
                .map_err(|error| format!("Failed to launch playtest: {error}"))?,
        );
        Ok(())
    }

    fn write_seeded_agent_playtest_payload_for_current_room(
        &self,
    ) -> Result<std::path::PathBuf, String> {
        let room_id = self
            .cur_room_id
            .ok_or_else(|| "No room is currently selected.".to_string())?;

        write_seeded_agent_payload(
            self,
            room_id,
            Some(self.default_seeded_agent_snapshot_request()),
        )
        .map_err(|error| format!("Could not write agent payload: {error:?}"))
    }

    /// Builds the default snapshot request used for seeded agent playtest launches.
    fn default_seeded_agent_snapshot_request(&self) -> AgentSnapshotRequest {
        AgentSnapshotRequest {
            profile: SnapshotProfile::RuntimeDebug,
            extras: payload!(),
        }
    }

    /// Stops any running playtest process and clears pending build state.
    #[allow(dead_code)]
    pub fn close_playtest(&mut self) {
        self.pending_playtest_build = None;
        if let Some(ref mut playtest_process) = self.playtest_process {
            playtest_process.kill();
        }
        self.playtest_process = None;
    }
}

#[cfg(test)]
mod tests {
    use super::AgentPlaytestControl;
    use crate::agents::{build_seeded_agent_playtest_launch, write_seeded_agent_payload};
    use crate::app::{Editor, EditorMode};
    use crate::storage::editor_storage::create_new_game;
    use engine_core::agents::visibility::{AgentSnapshotRequest, SnapshotProfile};
    use engine_core::agents::{AgentSessionManifest, AgentVisibilitySnapshot};
    use engine_core::constants::agents;
    use engine_core::engine_global::set_game_name;
    use engine_core::payload;
    use engine_core::prelude::{BackgroundTask, CurrentRoom, Game, Name, Room, RoomId, Transform};
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    fn replace_seeded_room(game: &mut Game, room_id: RoomId) {
        let world_id = game.current_world_id;
        let (original_room_id, grid_size) = {
            let world = game.get_world_mut(world_id);
            (world.starting_room_id.unwrap(), world.grid_size)
        };

        let mut room = Room::new(&mut game.ecs, room_id, grid_size);
        room.name = "Seeded Room".to_string();

        let world = game.get_world_mut(world_id);
        world.rooms.clear();
        world.rooms.push(room);
        world.current_room_id = Some(room_id);
        world.starting_room_id = Some(room_id);
        game.next_room_id = game.next_room_id.max(room_id.0);

        if let Some(proxy) = game.ecs.get_player_proxy(original_room_id) {
            game.ecs
                .add_component_to_entity(proxy, CurrentRoom(room_id));
        }
    }

    fn seeded_editor(test_game: &TestGameFolder, room_id: RoomId) -> Editor {
        set_game_name(test_game.name());

        let mut game = create_new_game(test_game.name().to_string());
        let world_id = game.current_world_id;
        replace_seeded_room(&mut game, room_id);

        game.ecs
            .create_entity()
            .with(Transform::default())
            .with(CurrentRoom(room_id))
            .with(Name("Seeded Entity".to_string()))
            .finish();

        Editor {
            game,
            mode: EditorMode::Room(room_id),
            cur_world_id: Some(world_id),
            cur_room_id: Some(room_id),
            ..Default::default()
        }
    }

    fn session_dir_for_payload_path(payload_path: &std::path::Path) -> PathBuf {
        payload_path.parent().unwrap().join(format!(
            "{}_agent",
            payload_path.file_stem().unwrap().to_string_lossy()
        ))
    }

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

        assert!(editor.request_close_playtest().is_ok());
        assert!(editor.request_open_playtest().is_err());
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

        editor.request_close_playtest().unwrap();

        assert!(editor.pending_playtest_build.is_none());
    }

    #[test]
    fn headless_agent_playtest_command_requires_room_selection() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_headless_playtest_control");
        set_game_name(test_game.name());

        let mut editor = Editor::default();

        assert!(editor.open_agent_playtest_for_current_room().is_err());
    }

    #[test]
    fn seeded_agent_playtest_payload_writer_uses_default_snapshot_request() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_headless_playtest_default_snapshot_request");
        let editor = seeded_editor(&test_game, RoomId(52));

        let payload_path = editor
            .write_seeded_agent_playtest_payload_for_current_room()
            .unwrap();
        let loaded = game_lib::agents::load_agent_payload(payload_path.to_str().unwrap()).unwrap();
        let expected_request = AgentSnapshotRequest {
            profile: SnapshotProfile::RuntimeDebug,
            extras: payload!(),
        };

        assert_eq!(loaded.snapshot_request, Some(expected_request));

        let _ = fs::remove_file(payload_path);
    }

    #[test]
    fn seeded_agent_playtest_accepts_runtime_snapshot_request_from_session_dir() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_runtime_snapshot_request");
        let editor = seeded_editor(&test_game, RoomId(53));
        let default_request = AgentSnapshotRequest {
            profile: SnapshotProfile::RuntimeDebug,
            extras: payload!(),
        };
        let override_request = AgentSnapshotRequest {
            profile: SnapshotProfile::RuntimeDebug,
            extras: payload!(accumulator_ms: 123.0),
        };

        let payload_path = editor
            .write_seeded_agent_playtest_payload_for_current_room()
            .unwrap();
        let launch = build_seeded_agent_playtest_launch(payload_path).unwrap();
        let session_dir = session_dir_for_payload_path(&launch.payload_path);
        let manifest_path = session_dir.join("agent-session.ron");
        let snapshot_path = session_dir.join("agent-snapshot.ron");
        let request_path = session_dir.join(agents::REQUEST_FILENAME);
        let log_path = PathBuf::from(agents::PLAYTEST_LOG_PATH);

        let _ = fs::remove_dir_all(&session_dir);
        let _ = fs::remove_file(&log_path);

        let mut child = Command::new(&launch.exe_path)
            .args(launch.arg_refs())
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

        let _ = fs::remove_file(&launch.payload_path);
        let _ = fs::remove_dir_all(session_dir);
        let _ = fs::remove_file(log_path);
    }

    #[test]
    fn seeded_agent_playtest_writes_manifest_log_and_snapshot_from_same_run() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_end_to_end");
        let editor = seeded_editor(&test_game, RoomId(51));
        let room_id = editor.cur_room_id.unwrap();
        let payload_path = write_seeded_agent_payload(&editor, room_id, None).unwrap();
        let launch = build_seeded_agent_playtest_launch(payload_path).unwrap();
        let session_dir = session_dir_for_payload_path(&launch.payload_path);
        let manifest_path = session_dir.join("agent-session.ron");
        let snapshot_path = session_dir.join("agent-snapshot.ron");
        let log_path = PathBuf::from(agents::PLAYTEST_LOG_PATH);

        let _ = fs::remove_dir_all(&session_dir);
        let _ = fs::remove_file(&log_path);
        let launched_at = std::time::SystemTime::now();

        let mut child = Command::new(&launch.exe_path)
            .args(launch.arg_refs())
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

        assert!(manifest.contains(launch.payload_path.to_string_lossy().as_ref()));
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

        let _ = fs::remove_file(&launch.payload_path);
        let _ = fs::remove_dir_all(session_dir);
        let _ = fs::remove_file(log_path);
    }
}
