use super::*;
use crate::game_global::{
    get_virtual_input_state, in_input_control, is_menu_active, push_command,
    reset_engine_session_state, set_menu_active, set_virtual_input_state, take_input_control,
};
use engine_core::audio::{push_audio_command, runtime, AudioCommand};
use engine_core::input::input_constants;
use engine_core::menu::{
    drain_menu_events, drain_slider_events, push_slider_event, GameMenuHandler, MenuActionHandler,
};

struct TestCommand;

impl crate::scripting::commands::lua_command::LuaCommand for TestCommand {
    fn execute(&mut self, _engine: &mut Engine) {}
}

#[test]
fn start_menu_entry_opens_the_root_menu_and_sets_front_end_policy() {
    let mut menu_manager = MenuManager::new();

    let game_state = apply_entry_mode(
        &mut menu_manager,
        EngineEntryMode::StartMenu {
            menu_id: "settings".to_string(),
        },
    );

    assert_eq!(game_state, GameState::StartMenu);
    assert_eq!(menu_manager.active_menu_id(), Some("settings"));
    assert_eq!(menu_manager.input_policy(), &MenuInputPolicy::FrontEnd);
}

#[test]
fn start_menu_session_stays_frozen_while_the_root_menu_is_open() {
    let mut menu_manager = MenuManager::new();
    menu_manager.set_input_policy(MenuInputPolicy::FrontEnd);
    menu_manager.open_menu("pause");

    assert_eq!(
        resolve_game_state(GameState::StartMenu, &menu_manager),
        GameState::StartMenu
    );
}

#[test]
fn start_menu_session_becomes_playing_when_the_root_menu_closes() {
    let menu_manager = MenuManager::new();

    assert_eq!(
        resolve_game_state(GameState::StartMenu, &menu_manager),
        GameState::Playing
    );
}

#[test]
fn start_menu_entry_syncs_global_menu_state_immediately() {
    reset_engine_session_state();

    let mut menu_manager = MenuManager::new();
    let game_state = apply_entry_mode(
        &mut menu_manager,
        EngineEntryMode::StartMenu {
            menu_id: "settings".to_string(),
        },
    );

    sync_global_menu_state(&menu_manager);

    assert_eq!(game_state, GameState::StartMenu);
    assert!(is_menu_active());
    assert!(in_input_control(input_constants::MENU));
}

#[test]
fn gameplay_pause_session_uses_the_paused_state() {
    let mut menu_manager = MenuManager::new();
    menu_manager.open_menu("pause");

    assert_eq!(
        resolve_game_state(GameState::Playing, &menu_manager),
        GameState::Paused
    );
}

#[test]
fn resetting_engine_session_state_clears_thread_local_runtime_state() {
    set_virtual_input_state(crate::input::VirtualInputState::single_frame_press(
        input_constants::RIGHT,
    ));
    set_menu_active(true);
    take_input_control("dialogue", crate::input::focus_priority::DIALOGUE);
    push_command(Box::new(TestCommand));
    GameMenuHandler.handle_action(input_constants::MENU);
    push_slider_event("volume".to_string(), 0.5);
    push_audio_command(AudioCommand::SetMasterVolume(0.5));
    runtime::set_music_playing(true);
    runtime::push_music_stopped_event(runtime::MusicStoppedEvent {
        id: "track".to_string(),
        reason: runtime::MusicStopReason::Stopped,
        next_id: None,
    });

    reset_engine_session_state();

    assert!(get_virtual_input_state().down().is_empty());
    assert!(get_virtual_input_state().pressed().is_empty());
    assert!(get_virtual_input_state().released().is_empty());
    assert!(!is_menu_active());
    assert!(in_input_control("player"));
    assert!(!in_input_control("dialogue"));
    assert!(drain_menu_events().is_empty());
    assert!(drain_slider_events().is_empty());
    assert!(runtime::drain_audio_events().is_empty());
    assert!(!runtime::is_music_playing());
}
