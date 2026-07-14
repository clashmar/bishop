use super::*;
use engine_core::constants::paths;
use engine_core::engine_global::set_game_name;
use engine_core::hydration::Hydratable;
use engine_core::scripting::lua_constants::{lua_dirs, lua_fields, lua_files};
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use engine_core::worlds::*;
use std::fs;

fn install_callback_module(lua: &Lua) {
    lua.load(
        r#"
        callback_hits = 0
        package.preload["save_manager"] = function()
            return {
                on_title_menu_open = function()
                    callback_hits = callback_hits + 1
                end,
                nested = {
                    on_open = function()
                        callback_hits = callback_hits + 1
                    end,
                },
            }
        end
        "#,
    )
    .exec()
    .unwrap();
}

fn game_with_two_worlds() -> Game {
    let mut game = Game::default();
    for id in 1..=2 {
        let mut world = World::new(WorldId(id), String::new(), 16.0);
        world.add_room(Room {
            id: RoomId(id),
            ..Default::default()
        });
        game.add_world(world);
    }
    game.select_world(WorldId(1));
    game
}

fn script_fixture(script_source: &str) -> (Lua, Ecs, ScriptManager, Entity, ScriptId) {
    let lua = Lua::new();
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().with(Active::default()).finish();
    let script_id = ScriptId(1);
    let mut script_manager = ScriptManager::default();
    let def: Table = lua.load(script_source).eval().unwrap();
    script_manager.table_defs.insert(script_id, def);
    script_manager.increment_ref(script_id);
    ecs.add_component_to_entity(
        entity,
        Script {
            script_id,
            ..Default::default()
        },
    );
    (lua, ecs, script_manager, entity, script_id)
}

#[test]
fn activate_entity_scripts_skips_inactive_entities() {
    let (lua, mut ecs, mut script_manager, entity, script_id) =
        script_fixture("return { init = function(self) end }");
    ecs.get_mut::<Active>(entity).unwrap().active = false;

    ScriptSystem::activate_entity_scripts(&lua, &mut ecs, &mut script_manager).unwrap();

    assert!(!script_manager.instances.contains_key(&(entity, script_id)));
    assert!(!script_manager.pending_inits.contains(&(entity, script_id)));
}

#[test]
fn activate_entity_scripts_keeps_pinned_inactive_entities_resident() {
    let (lua, mut ecs, mut script_manager, entity, script_id) =
        script_fixture("return { init = function(self) end }");
    let active = ecs.get_mut::<Active>(entity).unwrap();
    active.active = false;
    active.pin();

    ScriptSystem::activate_entity_scripts(&lua, &mut ecs, &mut script_manager).unwrap();

    assert!(script_manager.instances.contains_key(&(entity, script_id)));
    assert!(script_manager.pending_inits.contains(&(entity, script_id)));
}

#[test]
fn update_eligibility_rejects_entities_in_inactive_worlds() {
    let mut game = game_with_two_worlds();
    let script = Script {
        script_id: ScriptId(1),
        ..Default::default()
    };

    let in_active = game
        .ecs
        .create_entity()
        .with(Active::default())
        .with(script.clone())
        .with_current_room(RoomId(1))
        .finish();
    let in_inactive = game
        .ecs
        .create_entity()
        .with(Active::default())
        .with(script.clone())
        .with_current_room(RoomId(2))
        .finish();
    let unroomed = game
        .ecs
        .create_entity()
        .with(Active::default())
        .with(script.clone())
        .finish();

    assert!(update_eligible(&game, in_active, &script));
    assert!(!update_eligible(&game, in_inactive, &script));
    assert!(update_eligible(&game, unroomed, &script));
}

#[test]
fn update_eligibility_rejects_empty_script_ids() {
    let game = Game::default();
    let entity = Entity(1);
    let script = Script {
        script_id: ScriptId(0),
        ..Default::default()
    };

    assert!(!update_eligible(&game, entity, &script));
}

#[test]
fn scripted_entity_must_be_active_to_update() {
    let mut game = Game::default();
    let script = Script {
        script_id: ScriptId(7),
        ..Default::default()
    };
    let entity = game.ecs.create_entity()
        .with(Active::new(false))
        .with(script.clone())
        .finish();

    assert!(!update_eligible(&game, entity, &script));
}

#[test]
fn script_update_eligibility_rejects_entities_without_the_same_script_component() {
    let mut ecs = Ecs::default();
    let entity = ecs.create_entity().finish();

    assert!(!script_update_is_still_valid(&ecs, entity, ScriptId(7)));

    ecs.add_component_to_entity(
        entity,
        Script {
            script_id: ScriptId(3),
            ..Default::default()
        },
    );

    assert!(!script_update_is_still_valid(&ecs, entity, ScriptId(7)));
    assert!(script_update_is_still_valid(&ecs, entity, ScriptId(3)));
}

#[test]
fn invoke_menu_callback_calls_exported_function() {
    let lua = Lua::new();
    install_callback_module(&lua);

    invoke_menu_callback(&lua, "save_manager.on_title_menu_open").unwrap();

    assert_eq!(lua.globals().get::<i64>("callback_hits").unwrap(), 1);
}

#[test]
fn invoke_menu_callback_supports_nested_table_paths() {
    let lua = Lua::new();
    install_callback_module(&lua);

    invoke_menu_callback(&lua, "save_manager.nested.on_open").unwrap();

    assert_eq!(lua.globals().get::<i64>("callback_hits").unwrap(), 1);
}

#[test]
fn init_executes_globals_prelude_before_main() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("script_system_globals_prelude");
    set_game_name(test_game.name());

    let scripts_dir = game_folder(test_game.name())
        .join(paths::RESOURCES_FOLDER)
        .join(paths::SCRIPTS_FOLDER);
    let engine_dir = scripts_dir.join(lua_dirs::ENGINE);
    fs::create_dir_all(&engine_dir).unwrap();
    fs::write(
        engine_dir.join(lua_files::GLOBALS),
        "bootstrap_order = (bootstrap_order or \"\") .. \"g\"\nInput = { Space = \"space\" }\n",
    )
    .unwrap();
    fs::write(
        scripts_dir.join(lua_files::MAIN),
        "bootstrap_order = bootstrap_order .. \"m\"\nsaw_input = Input.Space\n",
    )
    .unwrap();

    set_game_name(test_game.name());
    let lua = Lua::new();
    let event_bus = EventBus::default();
    ScriptSystem::init(&lua, &event_bus);

    assert_eq!(lua.globals().get::<String>("bootstrap_order").unwrap(), "gm");
    assert_eq!(lua.globals().get::<String>("saw_input").unwrap(), "space");
}

#[test]
fn prepare_spawned_script_inits_rejects_root_args_without_root_init() {
    let lua = Lua::new();
    let mut ecs = Ecs::default();
    let root = ecs.create_entity().finish();
    let mut script_manager = ScriptManager::default();
    let def = lua.create_table().unwrap();
    let public = lua.create_table().unwrap();
    let init_args = lua.create_table().unwrap();

    public.set("speed", 120).unwrap();
    def.set(lua_fields::PUBLIC, public).unwrap();
    init_args.set("direction", "left").unwrap();
    script_manager.table_defs.insert(ScriptId(1), def);
    ecs.add_component_to_entity(
        root,
        Script {
            script_id: ScriptId(1),
            ..Default::default()
        },
    );

    let error = ScriptSystem::prepare_spawned_script_inits(
        &lua,
        &mut ecs,
        &mut script_manager,
        root,
        Some(Value::Table(init_args)),
    )
    .unwrap_err();

    assert!(error.to_string().contains("root script init"));
}

#[test]
fn deactivate_payload_scripts_removes_instances_pending_inits_and_entity_listeners() {
    let (lua, mut ecs, mut script_manager, entity, script_id) =
        script_fixture("return { init = function(self) end }");

    ScriptSystem::activate_payload_scripts(&lua, &mut ecs, &mut script_manager, &[entity])
        .unwrap();
    let listener = lua.create_function(|_, ()| Ok(())).unwrap();
    script_manager
        .event_bus
        .on_for_entity("tick".to_string(), entity, listener);

    assert!(script_manager.instances.contains_key(&(entity, script_id)));
    assert!(script_manager.pending_inits.contains(&(entity, script_id)));
    assert_eq!(script_manager.event_bus.listener_count_for_entity(entity), 1);

    ScriptSystem::deactivate_payload_scripts(&ecs, &mut script_manager, &[entity]);

    assert!(!script_manager.instances.contains_key(&(entity, script_id)));
    assert_eq!(script_manager.event_bus.listener_count_for_entity(entity), 0);
    assert!(!script_manager.pending_inits.iter().any(|(queued, _)| *queued == entity));
}

#[test]
fn activate_payload_scripts_requeues_init_exactly_once_after_rehydrate() {
    let (lua, mut ecs, mut script_manager, entity, script_id) = script_fixture(
        "return { init = function(self) self.public = self.public or {} end }",
    );

    ScriptSystem::activate_payload_scripts(&lua, &mut ecs, &mut script_manager, &[entity])
        .unwrap();
    ScriptSystem::activate_payload_scripts(&lua, &mut ecs, &mut script_manager, &[entity])
        .unwrap();

    let queued = script_manager
        .pending_inits
        .iter()
        .filter(|(queued, id)| *queued == entity && *id == script_id)
        .count();
    assert_eq!(queued, 1);
}

#[test]
fn failing_init_does_not_abort_later_inits() {
    let lua = Lua::new();
    lua.globals().set("init_hits", 0).unwrap();

    let failing_init: Function = lua
        .load("return function(self) error('boom from init') end")
        .eval()
        .unwrap();
    let succeeding_init: Function = lua
        .load("return function(self) init_hits = init_hits + 1 end")
        .eval()
        .unwrap();

    ScriptSystem::run_entity_init_callbacks(
        vec![
            (
                Entity(1),
                ScriptId(1),
                Some("scripts/failing.lua".to_string()),
                failing_init,
                lua.create_table().unwrap(),
            ),
            (
                Entity(2),
                ScriptId(2),
                Some("scripts/ok.lua".to_string()),
                succeeding_init,
                lua.create_table().unwrap(),
            ),
        ],
        || {},
    );

    assert_eq!(lua.globals().get::<i64>("init_hits").unwrap(), 1);
}

#[test]
fn failing_update_does_not_abort_later_updates() {
    let lua = Lua::new();
    lua.globals().set("update_hits", 0.0).unwrap();

    let failing_update: Function = lua
        .load("return function(self, dt) error('boom from update') end")
        .eval()
        .unwrap();
    let succeeding_update: Function = lua
        .load("return function(self, dt) update_hits = update_hits + dt end")
        .eval()
        .unwrap();

    ScriptSystem::run_entity_update_callback(
        Entity(1),
        ScriptId(1),
        Some("scripts/failing.lua"),
        &failing_update,
        &lua.create_table().unwrap(),
        0.25,
        || {},
    );
    ScriptSystem::run_entity_update_callback(
        Entity(2),
        ScriptId(2),
        Some("scripts/ok.lua"),
        &succeeding_update,
        &lua.create_table().unwrap(),
        0.25,
        || {},
    );

    assert_eq!(lua.globals().get::<f64>("update_hits").unwrap(), 0.25);
}

#[test]
fn failing_global_update_still_runs_followup_processing() {
    let lua = Lua::new();
    let failing_global_update: Function = lua
        .load("return function(dt) error('boom from global update') end")
        .eval()
        .unwrap();
    let mut followup_calls = 0;

    ScriptSystem::run_global_update_callback(&failing_global_update, 0.25, || {
        followup_calls += 1;
    });

    assert_eq!(followup_calls, 1);
}
