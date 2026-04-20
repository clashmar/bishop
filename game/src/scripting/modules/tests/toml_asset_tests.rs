use crate::engine::game_instance::GameInstance;
use crate::game_global::drain_commands;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::engine_module::EngineModule;
use crate::scripting::modules::entity_module::EntityHandle;
use crate::scripting::modules::entity_module::EntityModule;
use crate::scripting::script_system::ScriptSystem;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::lua_engine;
use engine_core::scripting::lua_constants::lua_fields;
use engine_core::scripting::lua_constants::{lua_dirs, lua_files};
use engine_core::scripting::modules::lua_module::LuaModule;
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter};
use mlua::{Lua, Table};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

const DEMO_GAME_NAME: &str = "Demo";
const DIALOGUE_KEY: &str = "greeting";
const DIALOGUE_FIELD: &str = "dialogue_id";

fn game_name_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn setup_engine_lua() -> Lua {
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();
    lua
}

fn setup_game_lua() -> (Lua, Rc<RefCell<GameInstance>>) {
    let _ = drain_commands().count();
    let lua = setup_engine_lua();
    let mut game = Game::default();
    game.name = DEMO_GAME_NAME.to_string();
    set_game_name(DEMO_GAME_NAME);
    game.init_text_manager();
    game.worlds.push(World::default());

    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
    }));

    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .unwrap();

    (lua, game_instance)
}

fn setup_entity_lua() -> (Lua, Rc<RefCell<GameInstance>>, Entity) {
    let (lua, game_instance) = setup_game_lua();
    let entity = game_instance
        .borrow_mut()
        .game
        .ecs
        .create_entity()
        .with(Transform::default())
        .finish();

    let entity_handle = lua.create_userdata(EntityHandle { entity }).unwrap();
    lua.globals().set("entity", entity_handle).unwrap();

    (lua, game_instance, entity)
}

#[test]
fn engine_asset_toml_without_argument_returns_unset_toml_id() {
    let _lock = game_name_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let lua = setup_engine_lua();
    EngineModule.register(&lua).unwrap();

    let toml_id = lua
        .load("return engine.asset.toml()")
        .eval::<TomlId>()
        .unwrap();

    assert_eq!(toml_id, TomlId(0));
}

#[test]
fn script_load_reads_toml_constructor_output_into_typed_field() {
    let _lock = game_name_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let (lua, _game_instance) = setup_game_lua();
    EngineModule.register(&lua).unwrap();
    let mut script_manager = ScriptManager::default();
    let def = lua
        .load(format!(
            r#"return {{ public = {{ {DIALOGUE_FIELD} = engine.asset.toml() }} }}"#
        ))
        .eval::<Table>()
        .unwrap();
    script_manager.table_defs.insert(ScriptId(1), def);

    let mut script = Script {
        script_id: ScriptId(1),
        data: ScriptData::default(),
    };
    let mut asset_registry = AssetRegistry::default();
    script
        .load(&lua, &mut asset_registry, &mut script_manager, Entity(7))
        .unwrap();

    assert!(matches!(
        script.data.fields.get(DIALOGUE_FIELD),
        Some(ScriptField::Toml(TomlId(_)))
    ));
}

#[test]
fn script_sync_to_lua_writes_toml_field_as_typed_userdata() {
    let _lock = game_name_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let (lua, _game_instance) = setup_game_lua();
    let public = lua.create_table().unwrap();
    let instance = lua.create_table().unwrap();
    instance.set("public", public.clone()).unwrap();

    let script = Script {
        script_id: ScriptId(1),
        data: ScriptData {
            fields: [(DIALOGUE_FIELD.to_string(), ScriptField::Toml(TomlId(3)))]
                .into_iter()
                .collect(),
        },
    };

    script.sync_to_lua_with_instance(&lua, &instance).unwrap();

    let written = public.get::<TomlId>(DIALOGUE_FIELD).unwrap();
    assert_eq!(written, TomlId(3));
}

#[test]
fn entity_say_accepts_toml_id_dialogue_fields() {
    let _lock = game_name_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let (lua, game_instance, _entity) = setup_entity_lua();
    EngineModule.register(&lua).unwrap();

    {
        let mut game_instance = game_instance.borrow_mut();
        game_instance
            .game
            .asset_registry
            .register_asset_relative_path(TomlId(1), "dialogue/npcs/npc.toml")
            .unwrap();
    }
    lua.globals().set("dialogue_id", TomlId(1)).unwrap();

    lua.load(format!("entity:say(dialogue_id, \"{DIALOGUE_KEY}\")"))
        .exec()
        .unwrap();

    let commands = drain_commands().collect::<Vec<_>>();
    assert_eq!(commands.len(), 1);
}

#[test]
fn entity_say_skips_queueing_when_toml_lookup_fails() {
    let _lock = game_name_test_lock()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let (lua, game_instance, _entity) = setup_entity_lua();
    EngineModule.register(&lua).unwrap();

    {
        let mut game_instance = game_instance.borrow_mut();
        game_instance
            .game
            .asset_registry
            .register_asset_relative_path(TomlId(1), "dialogue/npcs/npc.toml")
            .unwrap();
    }
    lua.globals().set("dialogue_id", TomlId(1)).unwrap();

    lua.load(format!(
        "entity:say(dialogue_id, \"missing_{DIALOGUE_KEY}\")"
    ))
    .exec()
    .unwrap();

    let commands = drain_commands().collect::<Vec<_>>();
    assert_eq!(commands.len(), 0);
}
