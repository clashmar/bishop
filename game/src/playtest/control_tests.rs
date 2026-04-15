use crate::engine::{advance_playtest_control_for_game_state, GameState};
use crate::game_global::{
    clear_active_playtest_control_timeline, clear_virtual_input_state, get_virtual_input_state,
    install_active_playtest_control_timeline, reset_engine_session_state,
    tick_active_playtest_control_timeline,
};
use crate::playtest::control::{
    apply_camera_control_frame, expand_chaos_profile, resolve_control_profile,
    ActiveControlTimeline,
};
use bishop::prelude::*;
use engine_core::agents::{
    AgentChaosConfig, AgentControlProfile, AgentControlProfileRef, CameraControlFrame,
    MovementControlFrame, BUILTIN_PROFILE_GROUNDED_WALK_RIGHT,
};
use engine_core::engine_global::{set_engine_mode, set_game_name, EngineMode};
use engine_core::input::input_constants;
use engine_core::prelude::CameraManager;
use engine_core::storage::path_utils::playtest_control_profiles_folder;
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use std::fs;
use std::sync::MutexGuard;

struct ControlStateGuard {
    game_name: String,
    engine_mode: EngineMode,
    _fs_lock: MutexGuard<'static, ()>,
}

impl ControlStateGuard {
    fn new() -> Self {
        let fs_lock = game_fs_test_lock().lock().unwrap();
        let guard = Self {
            game_name: engine_core::engine_global::game_name(),
            engine_mode: engine_core::engine_global::get_engine_mode(),
            _fs_lock: fs_lock,
        };
        clear_virtual_input_state();
        clear_active_playtest_control_timeline();
        guard
    }
}

impl Drop for ControlStateGuard {
    fn drop(&mut self) {
        clear_virtual_input_state();
        clear_active_playtest_control_timeline();
        set_game_name(self.game_name.clone());
        set_engine_mode(self.engine_mode);
    }
}

fn profile_with_down_input(input: &str) -> AgentControlProfile {
    AgentControlProfile {
        movement_frames: vec![MovementControlFrame {
            frame_count: 1,
            down_inputs: vec![input.to_string()],
            pressed_inputs: Vec::new(),
            released_inputs: Vec::new(),
        }],
        camera_frames: Vec::<CameraControlFrame>::new(),
    }
}

#[test]
fn named_profile_resolution_uses_canonical_builtin_constant() {
    let resolved = resolve_control_profile(&AgentControlProfileRef::Named(
        BUILTIN_PROFILE_GROUNDED_WALK_RIGHT.to_string(),
    ))
    .unwrap();

    assert_eq!(
        resolved.movement_frames[0].down_inputs,
        vec![input_constants::RIGHT.to_string()]
    );
}

#[test]
fn named_profile_resolution_prefers_inline_then_file_then_builtin() {
    let _guard = ControlStateGuard::new();
    let test_game = TestGameFolder::new("playtest_control_resolution");
    set_engine_mode(EngineMode::Playtest);
    set_game_name(test_game.name());

    let inline_profile = profile_with_down_input(input_constants::UP);
    let file_profile = profile_with_down_input(input_constants::LEFT);
    let profiles_folder = playtest_control_profiles_folder();
    fs::create_dir_all(&profiles_folder).unwrap();
    fs::write(
        profiles_folder.join(format!("{BUILTIN_PROFILE_GROUNDED_WALK_RIGHT}.ron")),
        ron::ser::to_string_pretty(&file_profile, ron::ser::PrettyConfig::default()).unwrap(),
    )
    .unwrap();

    let resolved_inline =
        resolve_control_profile(&AgentControlProfileRef::Inline(inline_profile.clone())).unwrap();
    let resolved_file = resolve_control_profile(&AgentControlProfileRef::Named(
        BUILTIN_PROFILE_GROUNDED_WALK_RIGHT.to_string(),
    ))
    .unwrap();

    fs::remove_file(profiles_folder.join(format!("{BUILTIN_PROFILE_GROUNDED_WALK_RIGHT}.ron")))
        .unwrap();

    let resolved_builtin = resolve_control_profile(&AgentControlProfileRef::Named(
        BUILTIN_PROFILE_GROUNDED_WALK_RIGHT.to_string(),
    ))
    .unwrap();

    assert_eq!(resolved_inline, inline_profile);
    assert_eq!(resolved_file, file_profile);
    assert_ne!(resolved_builtin, file_profile);
    assert_eq!(
        resolved_builtin.movement_frames[0].down_inputs,
        vec![input_constants::RIGHT.to_string()]
    );
}

#[test]
fn active_control_timeline_applies_right_then_clears_when_complete() {
    let _guard = ControlStateGuard::new();
    let profile = profile_with_down_input(input_constants::RIGHT);

    install_active_playtest_control_timeline(ActiveControlTimeline::new(profile));
    tick_active_playtest_control_timeline();

    let active = get_virtual_input_state();
    assert_eq!(active.down().get(input_constants::RIGHT), Some(&true));

    tick_active_playtest_control_timeline();

    let cleared = get_virtual_input_state();
    assert!(cleared.down().is_empty());
    assert!(cleared.pressed().is_empty());
    assert!(cleared.released().is_empty());
}

#[test]
fn camera_control_frame_updates_active_camera_state() {
    let mut manager = CameraManager::default();
    manager.active.camera.target = vec2(10.0, 20.0);
    manager.active.camera.zoom = vec2(1.0, 1.0);

    apply_camera_control_frame(
        &mut manager,
        &CameraControlFrame {
            frame_count: 1,
            pan_delta_x: 5.0,
            pan_delta_y: -2.0,
            zoom_delta: 0.25,
            follow_enabled: Some(false),
        },
    );

    assert_eq!(manager.active.camera.target, vec2(15.0, 18.0));
    assert_eq!(manager.active.camera.zoom, vec2(1.25, 1.25));
    assert!(!manager.follow_is_enabled());
}

#[test]
fn clearing_runtime_camera_overrides_restores_default_follow_and_flags() {
    let mut manager = CameraManager::default();
    manager.active.camera.target = vec2(10.0, 20.0);
    manager.active.camera.zoom = vec2(1.0, 1.0);

    apply_camera_control_frame(
        &mut manager,
        &CameraControlFrame {
            frame_count: 1,
            pan_delta_x: 5.0,
            pan_delta_y: -2.0,
            zoom_delta: 0.25,
            follow_enabled: Some(false),
        },
    );
    manager.clear_runtime_overrides();

    assert!(manager.follow_is_enabled());
    assert!(!manager.runtime_override_is_active());
}

#[test]
fn playtest_control_waits_for_playing_state_before_advancing() {
    reset_engine_session_state();
    install_active_playtest_control_timeline(ActiveControlTimeline::new(
        resolve_control_profile(&AgentControlProfileRef::Named(
            BUILTIN_PROFILE_GROUNDED_WALK_RIGHT.to_string(),
        ))
        .unwrap(),
    ));

    let blocked = advance_playtest_control_for_game_state(&GameState::StartMenu);
    assert!(blocked.is_none());
    assert!(get_virtual_input_state().down().is_empty());

    let advanced = advance_playtest_control_for_game_state(&GameState::Playing);
    assert!(advanced.is_some());
    assert_eq!(
        get_virtual_input_state().down().get(input_constants::RIGHT),
        Some(&true)
    );
}

#[test]
fn chaos_expansion_is_deterministic_for_same_seed() {
    let config = AgentChaosConfig { seed: 7 };

    assert_eq!(expand_chaos_profile(&config), expand_chaos_profile(&config));
}

#[test]
fn expanded_chaos_profile_can_be_replayed_as_normal_profile() {
    let expanded = expand_chaos_profile(&AgentChaosConfig { seed: 11 });
    let timeline = ActiveControlTimeline::new(expanded.clone());

    assert_eq!(timeline.profile(), &expanded);
}
