use engine_core::playtest::{
    CameraControlFrame, MovementControlFrame, PlaytestChaosConfig, PlaytestControlProfile,
    PlaytestControlProfileRef,
};
use engine_core::prelude::*;
use engine_core::storage::path_utils::playtest_control_profiles_folder;
use std::fs;

/// Resolves inline, file-defined, or built-in control profiles.
pub fn resolve_control_profile(
    profile_ref: &PlaytestControlProfileRef,
) -> Option<PlaytestControlProfile> {
    match profile_ref {
        PlaytestControlProfileRef::Inline(profile) => Some(profile.clone()),
        PlaytestControlProfileRef::Named(name) => {
            load_file_profile(name).or_else(|| PlaytestControlProfile::builtin_named(name))
        }
    }
}

/// Expands deterministic chaos settings into a concrete replayable control profile.
pub fn expand_chaos_profile(config: &PlaytestChaosConfig) -> PlaytestControlProfile {
    let direction_inputs = [
        input_constants::LEFT,
        input_constants::RIGHT,
        input_constants::UP,
        input_constants::DOWN,
    ];
    let seed = config.seed;
    let movement_input = direction_inputs[(seed as usize) % direction_inputs.len()];
    let movement_frames = vec![MovementControlFrame {
        frame_count: 15 + (seed % 46) as u32,
        down_inputs: vec![movement_input.to_string()],
        pressed_inputs: Vec::new(),
        released_inputs: Vec::new(),
    }];
    let pan_sign = if seed & 1 == 0 { 1.0 } else { -1.0 };
    let camera_frames = vec![CameraControlFrame {
        frame_count: 10 + ((seed / 2) % 21) as u32,
        pan_delta_x: pan_sign * (1.0 + ((seed % 3) as f32)),
        pan_delta_y: 0.0,
        zoom_delta: ((seed % 5) as f32) * 0.05,
        follow_enabled: Some(seed & 1 == 0),
    }];

    PlaytestControlProfile {
        movement_frames,
        camera_frames,
    }
}

fn load_file_profile(name: &str) -> Option<PlaytestControlProfile> {
    let path = playtest_control_profiles_folder().join(format!("{name}.ron"));
    let ron = fs::read_to_string(path).ok()?;
    ron::from_str(&ron).ok()
}

pub(super) fn canonical_input_name(input: &str) -> Option<&'static str> {
    engine_core::input::input_table::KEY_TABLE
        .iter()
        .map(|(name, _)| *name)
        .chain(
            engine_core::input::input_table::MOUSE_TABLE
                .iter()
                .map(|(name, _)| *name),
        )
        .find(|name| *name == input)
}
