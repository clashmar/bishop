use super::canonical_input_name;
use engine_core::playtest::{CameraControlFrame, MovementControlFrame, PlaytestControlProfile};
use engine_core::prelude::*;

/// One resolved runtime frame of control data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeControlFrame {
    pub down_inputs: Vec<&'static str>,
    pub pressed_inputs: Vec<&'static str>,
    pub released_inputs: Vec<&'static str>,
    pub camera_frame: Option<CameraControlFrame>,
}

/// Active deterministic control timeline advanced once per engine frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveControlTimeline {
    profile: PlaytestControlProfile,
    movement_index: usize,
    movement_remaining: u32,
    camera_index: usize,
    camera_remaining: u32,
}

impl ActiveControlTimeline {
    /// Creates a runtime timeline from a resolved control profile.
    pub fn new(profile: PlaytestControlProfile) -> Self {
        Self {
            profile,
            movement_index: 0,
            movement_remaining: 0,
            camera_index: 0,
            camera_remaining: 0,
        }
    }

    /// Returns the resolved profile backing this timeline.
    pub fn profile(&self) -> &PlaytestControlProfile {
        &self.profile
    }

    /// Advances one engine frame and returns the virtual input overlay for that frame.
    pub fn tick(&mut self) -> Option<RuntimeControlFrame> {
        if self.is_complete() {
            return None;
        }

        let movement_frame = self.current_movement_frame().cloned();
        let camera_frame = self.current_camera_frame().cloned();
        let mut resolved = RuntimeControlFrame {
            camera_frame,
            ..RuntimeControlFrame::default()
        };

        if let Some(frame) = movement_frame {
            for input in frame
                .down_inputs
                .iter()
                .filter_map(|input| canonical_input_name(input))
            {
                resolved.down_inputs.push(input);
            }

            for input in frame
                .pressed_inputs
                .iter()
                .filter_map(|input| canonical_input_name(input))
            {
                resolved.down_inputs.push(input);
                resolved.pressed_inputs.push(input);
            }

            for input in frame
                .released_inputs
                .iter()
                .filter_map(|input| canonical_input_name(input))
            {
                resolved.released_inputs.push(input);
            }
        }

        self.advance_indices();
        Some(resolved)
    }

    /// Returns true when both movement and camera timelines have finished.
    pub fn is_complete(&self) -> bool {
        self.movement_index >= self.profile.movement_frames.len()
            && self.camera_index >= self.profile.camera_frames.len()
    }

    fn current_movement_frame(&mut self) -> Option<&MovementControlFrame> {
        while self.movement_index < self.profile.movement_frames.len() {
            if self.movement_remaining == 0 {
                self.movement_remaining =
                    self.profile.movement_frames[self.movement_index].frame_count;
            }

            if self.movement_remaining == 0 {
                self.movement_index += 1;
                continue;
            }

            return self.profile.movement_frames.get(self.movement_index);
        }

        None
    }

    fn current_camera_frame(&mut self) -> Option<&CameraControlFrame> {
        while self.camera_index < self.profile.camera_frames.len() {
            if self.camera_remaining == 0 {
                self.camera_remaining = self.profile.camera_frames[self.camera_index].frame_count;
            }

            if self.camera_remaining == 0 {
                self.camera_index += 1;
                continue;
            }

            return self.profile.camera_frames.get(self.camera_index);
        }

        None
    }

    fn advance_indices(&mut self) {
        if self.movement_remaining > 0 {
            self.movement_remaining -= 1;
            if self.movement_remaining == 0 {
                self.movement_index += 1;
            }
        }

        if self.camera_remaining > 0 {
            self.camera_remaining -= 1;
            if self.camera_remaining == 0 {
                self.camera_index += 1;
            }
        }
    }
}

/// Applies one runtime camera-control frame to the active camera.
pub fn apply_camera_control_frame(camera_manager: &mut CameraManager, frame: &CameraControlFrame) {
    if let Some(enabled) = frame.follow_enabled {
        camera_manager.set_follow_enabled(enabled);
    }

    camera_manager.apply_runtime_pan_delta(Vec2::new(frame.pan_delta_x, frame.pan_delta_y));
    camera_manager.apply_runtime_zoom_delta(frame.zoom_delta);
}
