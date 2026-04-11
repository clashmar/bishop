use crate::app::Editor;
use crate::editor_global::push_toast;
use crate::playtest::playtest_process::PlaytestProcess;
use crate::playtest::room_playtest::{resolve_playtest_binary, write_playtest_payload};
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
    use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};

    #[test]
    fn agent_playtest_command_closes_cleanly_when_idle() {
        let _lock = game_fs_test_lock().lock().unwrap();
        let test_game = TestGameFolder::new("agent_playtest_control");
        set_game_name(test_game.name());

        let mut editor = Editor::default();

        assert!(editor.request_close_playtest().is_ok());
        assert!(editor.request_open_playtest().is_err());
    }
}
