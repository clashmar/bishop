use crate::app::Editor;
use crate::editor_global::push_toast;
use crate::playtest::playtest_process::PlaytestProcess;
use crate::playtest::room_playtest::{resolve_playtest_binary, write_playtest_payload};
use engine_core::task::BackgroundTask;
use std::path::PathBuf;

enum PreparedPlaytestLaunch {
    DeferredEditorBuild(BackgroundTask<Result<(PathBuf, PathBuf), String>>),
    EditorAttached {
        exe_path: PathBuf,
        payload_path: PathBuf,
    },
}

impl Editor {
    /// Opens playtest using the current room selection.
    pub fn launch_playtest_for_current_room(&mut self) -> Result<(), String> {
        self.ensure_no_pending_playtest_build()?;
        let launch = self.prepare_playtest_launch()?;
        self.launch_prepared_playtest(launch)
    }

    fn ensure_no_pending_playtest_build(&self) -> Result<(), String> {
        if self.pending_playtest_build.is_some() {
            return Err("Playtest build already in progress.".to_string());
        }

        Ok(())
    }

    fn launch_prepared_playtest(&mut self, launch: PreparedPlaytestLaunch) -> Result<(), String> {
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
        }
    }

    fn prepare_playtest_launch(&self) -> Result<PreparedPlaytestLaunch, String> {
        let room_id = self
            .cur_room_id
            .ok_or_else(|| "No room is currently selected.".to_string())?;

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
}

#[cfg(test)]
#[path = "tests/playtest_control_tests.rs"]
mod playtest_control_tests;
