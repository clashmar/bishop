use crate::agents::write_seeded_agent_payload;
use crate::app::Editor;
use crate::editor_global::push_toast;
use crate::playtest::playtest_process::PlaytestProcess;
use crate::playtest::room_playtest::{resolve_playtest_binary, write_playtest_payload};
use engine_core::constants::agents;
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

        let room_id = self
            .cur_room_id
            .ok_or_else(|| "No room is currently selected.".to_string())?;

        let payload_path = write_seeded_agent_payload(self, room_id)
            .map_err(|error| format!("Could not write agent payload: {error:?}"))?;

        let exe_path = resolve_playtest_binary().map_err(|error| error.to_string())?;
        if let Some(ref mut old_process) = self.playtest_process {
            old_process.kill();
        }

        let args = [
            std::ffi::OsStr::new(agents::HEADLESS_FLAG),
            std::ffi::OsStr::new(agents::PAYLOAD_FLAG),
            payload_path.as_os_str(),
        ];

        self.playtest_process = Some(
            PlaytestProcess::spawn_with_args(&exe_path, &args)
                .map_err(|error| format!("Failed to launch playtest: {error}"))?,
        );
        Ok(())
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
    use crate::agents::write_seeded_agent_payload;
    use crate::app::{Editor, EditorMode};
    use crate::playtest::room_playtest::resolve_playtest_binary;
    use crate::storage::editor_storage::create_new_game;
    use engine_core::constants::agents;
    use engine_core::engine_global::set_game_name;
    use engine_core::prelude::{BackgroundTask, CurrentRoom, Name, RoomId, Transform};
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::ffi::OsStr;
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    fn seeded_editor(test_game: &TestGameFolder, _room_id: RoomId) -> Editor {
        set_game_name(test_game.name());

        let mut game = create_new_game(test_game.name().to_string());
        let world_id = game.current_world_id;
        let world = game.get_world_mut(world_id);
        let room = world
            .rooms
            .first_mut()
            .expect("new game should create an initial room");
        room.name = "Seeded Room".to_string();
        let room_id = room.id;
        world.current_room_id = Some(room_id);
        world.starting_room_id = Some(room_id);

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

        let mut editor = Editor::default();
        editor.pending_playtest_build = Some(BackgroundTask::spawn(|| {
            Ok((PathBuf::new(), PathBuf::new()))
        }));

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
    fn seeded_agent_playtest_writes_manifest_log_and_snapshot_from_same_run() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_end_to_end");
        let editor = seeded_editor(&test_game, RoomId(51));
        let room_id = editor.cur_room_id.unwrap();
        let payload_path = write_seeded_agent_payload(&editor, room_id).unwrap();
        let session_dir = payload_path.parent().unwrap().join(format!(
            "{}_agent",
            payload_path.file_stem().unwrap().to_string_lossy()
        ));
        let manifest_path = session_dir.join("agent-session.ron");
        let snapshot_path = session_dir.join("agent-snapshot.ron");
        let log_path = PathBuf::from(agents::PLAYTEST_LOG_PATH);
        let exe_path = resolve_playtest_binary().unwrap();

        let _ = fs::remove_dir_all(&session_dir);
        let _ = fs::remove_file(&log_path);

        let args = [
            OsStr::new(agents::HEADLESS_FLAG),
            OsStr::new(agents::PAYLOAD_FLAG),
            payload_path.as_os_str(),
        ];

        let mut child = Command::new(&exe_path)
            .args(args)
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
        assert!(snapshot.contains(agents::PLAYTEST_RUNTIME_TOPIC));
        assert!(snapshot.contains(agents::PLAYTEST_FRAME_LABEL));
        assert!(log.contains("package.path loaded from"));

        let _ = fs::remove_file(payload_path);
        let _ = fs::remove_dir_all(session_dir);
        let _ = fs::remove_file(log_path);
    }
}
