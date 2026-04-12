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
    use crate::app::Editor;
    use engine_core::engine_global::set_game_name;
    use engine_core::prelude::BackgroundTask;
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
    use std::path::PathBuf;

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

        assert!(editor
            .open_agent_playtest_for_current_room()
            .is_err());
    }
}
