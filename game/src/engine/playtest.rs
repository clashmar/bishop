use super::{Engine, GameState};
use crate::game_global::advance_active_playtest_control_timeline;
use crate::playtest::control::{apply_camera_control_frame, RuntimeControlFrame};

impl Engine {
    /// Clears any runtime camera overrides applied by playtest control.
    pub fn clear_playtest_camera_overrides(&mut self) {
        self.camera_manager.clear_runtime_overrides();
    }

    pub(super) fn apply_playtest_camera_control(&mut self, frame: RuntimeControlFrame) {
        if let Some(camera_frame) = frame.camera_frame.as_ref() {
            apply_camera_control_frame(&mut self.camera_manager, camera_frame);
        }
    }
}

pub(crate) fn advance_playtest_control_for_game_state(
    game_state: &GameState,
) -> Option<RuntimeControlFrame> {
    if *game_state != GameState::Playing {
        return None;
    }

    advance_active_playtest_control_timeline()
}
