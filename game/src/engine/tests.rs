use super::*;
use crate::game_global::drain_commands;
use crate::physics::events::{KinematicContactEvent, PhysicsEvents};
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::lua_entity_handle;
use engine_core::ecs::{Script, ScriptData, ScriptId, Transform};
use engine_core::game::Game;
use engine_core::scripting::lua_constants::{lua_entity, lua_globals, lua_kinematic};
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod game_instance_tests;

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

#[test]
fn resolve_requested_session_action_returns_quit_to_title_when_enabled() {
    assert_eq!(
        resolve_requested_session_action(MenuSessionAction::QuitToMainMenu, true),
        RequestedSessionAction::QuitToTitle
    );
}

#[test]
fn resolve_requested_session_action_returns_close_app_for_skip_playtest_quit() {
    assert_eq!(
        resolve_requested_session_action(MenuSessionAction::QuitToMainMenu, false),
        RequestedSessionAction::CloseApp
    );
}

#[test]
fn resolve_requested_session_action_returns_close_app_for_quit_game() {
    assert_eq!(
        resolve_requested_session_action(MenuSessionAction::QuitGame, true),
        RequestedSessionAction::CloseApp
    );
}

#[test]
fn retained_physics_event_callbacks_can_queue_entity_commands() {
    let _ = drain_commands();
    let lua = Lua::new();
    let mut game = Game::default();
    let kinematic = game.ecs.create_entity().with(Transform::default()).finish();
    let other = game.ecs.create_entity().with(Transform::default()).finish();
    let script_id = ScriptId(1);
    let instance: mlua::Table = lua
        .load(format!(
            "return {{ {} = function(self, event) self.entity:{}({{ x = 3, y = -2 }}) end }}",
            lua_kinematic::CALLBACK_KINEMATIC_CONTACT,
            lua_entity::MOVE_BY,
        ))
        .eval()
        .unwrap();
    instance
        .set(lua_globals::ENTITY_HANDLE, lua_entity_handle(&lua, other).unwrap())
        .unwrap();
    game.ecs.replace_component(
        other,
        Script {
            script_id,
            data: ScriptData::default(),
        },
    );
    game.script_manager.instances.insert((other, script_id), instance);

    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    }));
    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .unwrap();

    let mut events = PhysicsEvents::default();
    events.push_kinematic_contact(KinematicContactEvent::Contact {
        kinematic,
        dynamic: other,
    });

    emit_retained_physics_events(&lua, &game_instance, &mut events);

    assert_eq!(drain_commands().count(), 1);
}
