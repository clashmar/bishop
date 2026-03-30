use super::*;

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
fn gameplay_pause_session_uses_the_paused_state() {
    let mut menu_manager = MenuManager::new();
    menu_manager.open_menu("pause");

    assert_eq!(
        resolve_game_state(GameState::Playing, &menu_manager),
        GameState::Paused
    );
}
