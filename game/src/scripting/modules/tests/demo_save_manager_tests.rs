use super::demo_save_flow_tests::load_demo_script;
use engine_core::scripting::lua_constants::{lua_engine, lua_fields, lua_save};
use mlua::{Lua, Table};

const TEST_MODULES_KEY: &str = "bishop:demo_save_test_modules";

fn register_test_module(lua: &Lua, module_name: &str, module: Table) -> mlua::Result<()> {
    let modules = match lua.named_registry_value::<Option<Table>>(TEST_MODULES_KEY)? {
        Some(modules) => modules,
        None => {
            let modules = lua.create_table()?;
            lua.set_named_registry_value(TEST_MODULES_KEY, modules.clone())?;
            let require = lua.create_function(|lua, name: String| {
                let modules: Table = lua.named_registry_value(TEST_MODULES_KEY)?;
                modules.get::<Table>(name)
            })?;
            lua.globals().set("require", require)?;
            modules
        }
    };
    modules.set(module_name, module)
}

#[test]
fn save_manager_capture_when_provider_invoked_uses_save_flow_capture_location() -> mlua::Result<()> {
    let lua = Lua::new();
    let engine = lua.create_table()?;
    let save = lua.create_table()?;
    let provider_slot = lua.create_table()?;
    let player_public = lua.create_table()?;
    player_public.set("health", 80)?;
    let transform_position = lua.create_table()?;
    transform_position.set(lua_fields::X, 12.0)?;
    transform_position.set(lua_fields::Y, 34.0)?;
    let transform = lua.create_table()?;
    transform.set(lua_fields::POSITION, transform_position)?;
    let entity = lua.create_table()?;
    entity
        .set(
            "get",
            lua.create_function(move |_, (_entity, _component): (Table, mlua::Value)| {
                Ok(transform.clone())
            })
            ?,
        )
        ?;
    entity
        .set(
            "current_room",
            lua.create_function(|_, (_entity,): (Table,)| Ok(7))
                ?,
        )
        ?;
    let player = lua.create_table()?;
    player.set("entity", entity)?;
    player.set(lua_fields::PUBLIC, player_public)?;
    let current_world = lua.create_table()?;
    current_world.set(lua_fields::ID, 42)?;
    let game_manager_public = lua.create_table()?;
    game_manager_public.set("level", 3)?;
    let game_manager = lua.create_table()?;
    game_manager.set(lua_fields::PUBLIC, game_manager_public)?;
    game_manager
        .set("get_score", lua.create_function(|_, (_gm,): (Table,)| Ok(99))?)
        ?;
    let tags = lua.create_table()?;
    tags.set("Autosave", "Autosave")?;
    let log = lua.create_table()?;
    log.set("info", lua.create_function(|_, _msg: String| Ok(()))?)
        ?;
    let menu = lua.create_table()?;
    menu.set("close", lua.create_function(|_, ()| Ok(()))?)
        ?;

    let provider_slot_for_register = provider_slot.clone();
    save.set(
        lua_save::REGISTER_PROVIDER,
        lua.create_function(move |_, provider: Table| {
            provider_slot_for_register.set("provider", provider)?;
            Ok(())
        })
        ?,
    )
    ?;
    save.set("to_string", lua.create_function(|_, doc: Table| Ok(doc))?)
        ?;
    engine.set(lua_save::SAVE, save)?;
    engine.set("player", lua.create_function(move |_, ()| Ok(player.clone()))?)?;
    engine
        .set(
            lua_engine::CURRENT_WORLD,
            lua.create_function(move |_, ()| Ok(current_world.clone()))?,
        )
        ?;
    engine.set("game_manager", game_manager)?;
    engine.set("tags", tags)?;
    engine.set(lua_engine::ON, lua.create_function(|_, (_event, _handler): (String, mlua::Function)| Ok(()))?)
        ?;
    engine.set(lua_engine::QUIT_TO_TITLE, lua.create_function(|_, ()| Ok(()))?)?;
    engine.set(lua_engine::LOG, log)?;
    engine.set("menu", menu)?;
    lua.globals().set(lua_engine::ENGINE, engine)?;
    let components = lua.create_table()?;
    components.set("Transform", "Transform")?;
    lua.globals().set("Components", components)?;

    let capture_calls = lua.create_table()?;
    let build_calls = lua.create_table()?;
    let save_flow = lua.create_table()?;
    let capture_calls_for_capture = capture_calls.clone();
    save_flow
        .set(
            "capture_location",
            lua.create_function(move |lua, (transform, room_id): (Table, i64)| {
                capture_calls_for_capture.set("transform", transform)?;
                capture_calls_for_capture.set(lua_fields::ROOM_ID, room_id)?;
                let snapshot = lua.create_table()?;
                snapshot.set("source", "capture_location")?;
                Ok(snapshot)
            })
            ?,
        )
        ?;
    let build_calls_for_build = build_calls.clone();
    save_flow
        .set(
            "build_save_document",
            lua.create_function(move |lua, (progress, snapshot): (Table, Table)| {
                build_calls_for_build.set("progress", progress)?;
                build_calls_for_build.set("snapshot", snapshot.clone())?;
                let doc = lua.create_table()?;
                doc.set("snapshot", snapshot)?;
                Ok(doc)
            })
            ?,
        )
        ?;
    save_flow.set("bind_runtime_handlers", lua.create_function(|_, ()| Ok(()))?)?;
    save_flow.set("request_manual", lua.create_function(|_, ()| Ok(()))?)?;
    save_flow.set("set_active_anchor", lua.create_function(|_, _: mlua::Value| Ok(()))?)?;
    save_flow.set("resolve_restore_target", lua.create_function(|_, _: mlua::Value| Ok(mlua::Value::Nil))?)?;
    save_flow.set("active_anchor", lua.create_function(|_, ()| Ok(mlua::Value::Nil))?)?;
    register_test_module(&lua, "save_flow", save_flow)?;

    let autosave = lua.create_table()?;
    autosave
        .set("configure", lua.create_function(|_, _: mlua::Value| Ok(()))?)
        ?;
    register_test_module(&lua, "autosave", autosave)?;

    load_demo_script(&lua, "save_manager.lua");

    let provider: Table = provider_slot.get("provider")?;
    let capture: mlua::Function = provider.get("capture")?;
    let saved: Table = capture.call(())?;
    let snapshot: Table = saved.get("snapshot")?;

    assert_eq!(capture_calls.get::<i64>(lua_fields::ROOM_ID)?, 7);
    assert_eq!(snapshot.get::<String>("source")?, "capture_location");
    let built_snapshot: Table = build_calls.get("snapshot")?;
    assert_eq!(built_snapshot.get::<String>("source")?, "capture_location");
    Ok(())
}

#[test]
fn save_manager_apply_when_restore_target_exists_uses_save_flow_apply_restore_target() -> mlua::Result<()> {
    let lua = Lua::new();
    let engine = lua.create_table()?;
    let save = lua.create_table()?;
    let provider_slot = lua.create_table()?;
    let player_public = lua.create_table()?;
    player_public.set("health", 10)?;
    let entity = lua.create_table()?;
    let player = lua.create_table()?;
    player.set("entity", entity)?;
    player.set(lua_fields::PUBLIC, player_public.clone())?;
    let game_manager_public = lua.create_table()?;
    game_manager_public.set("score", 0)?;
    game_manager_public.set("level", 0)?;
    let game_manager = lua.create_table()?;
    game_manager.set(lua_fields::PUBLIC, game_manager_public)?;
    let log = lua.create_table()?;
    log.set("info", lua.create_function(|_, _msg: String| Ok(()))?)
        ?;
    let menu = lua.create_table()?;
    menu.set("close", lua.create_function(|_, ()| Ok(()))?)
        ?;
    let tags = lua.create_table()?;
    tags.set("Autosave", "Autosave")?;

    let provider_slot_for_register = provider_slot.clone();
    save.set(
        lua_save::REGISTER_PROVIDER,
        lua.create_function(move |_, provider: Table| {
            provider_slot_for_register.set("provider", provider)?;
            Ok(())
        })
        ?,
    )
    ?;
    save.set("from_string", lua.create_function(|lua, _data: String| {
        let progress = lua.create_table()?;
        progress.set("score", 7)?;
        progress.set("level", 2)?;
        progress.set("health", 90)?;
        let saved = lua.create_table()?;
        saved.set("progress", progress)?;
        Ok(saved)
    })?)?;
    engine.set(lua_save::SAVE, save)?;
    engine.set("player", lua.create_function(move |_, ()| Ok(player.clone()))?)?;
    engine.set("game_manager", game_manager)?;
    engine.set("tags", tags)?;
    engine.set(lua_engine::ON, lua.create_function(|_, (_event, _handler): (String, mlua::Function)| Ok(()))?)
        ?;
    engine.set(lua_engine::QUIT_TO_TITLE, lua.create_function(|_, ()| Ok(()))?)?;
    engine.set(lua_engine::LOG, log)?;
    engine.set("menu", menu)?;
    lua.globals().set(lua_engine::ENGINE, engine)?;

    let applied = lua.create_table()?;
    let save_flow = lua.create_table()?;
    save_flow.set("bind_runtime_handlers", lua.create_function(|_, ()| Ok(()))?)?;
    save_flow.set("request_manual", lua.create_function(|_, ()| Ok(()))?)?;
    save_flow.set("set_active_anchor", lua.create_function(|_, _: mlua::Value| Ok(()))?)?;
    save_flow.set("capture_location", lua.create_function(|lua, _: mlua::MultiValue| lua.create_table())?)?;
    save_flow.set("build_save_document", lua.create_function(|lua, _: mlua::MultiValue| lua.create_table())?)?;
    save_flow.set(
        "resolve_restore_target",
        lua.create_function(|lua, _: mlua::Value| {
            let restore = lua.create_table()?;
            restore.set(lua_fields::WORLD_ID, 3)?;
            restore.set(lua_fields::ROOM_ID, 8)?;
            restore.set(lua_fields::X, 50.0)?;
            restore.set(lua_fields::Y, 60.0)?;
            Ok(restore)
        })
        ?,
    )
    ?;
    let applied_for_restore = applied.clone();
    save_flow
        .set(
            "apply_restore_target",
            lua.create_function(move |_, target: Table| {
                applied_for_restore.set("target", target)?;
                Ok(())
            })
            ?,
        )
        ?;
    register_test_module(&lua, "save_flow", save_flow)?;

    let autosave = lua.create_table()?;
    autosave
        .set("configure", lua.create_function(|_, _: mlua::Value| Ok(()))?)
        ?;
    register_test_module(&lua, "autosave", autosave)?;

    load_demo_script(&lua, "save_manager.lua");

    let provider: Table = provider_slot.get("provider")?;
    let apply: mlua::Function = provider.get("apply")?;
    apply.call::<()>(String::new())?;

    let target: Table = applied.get("target")?;
    assert_eq!(target.get::<i64>(lua_fields::WORLD_ID)?, 3);
    assert_eq!(target.get::<i64>(lua_fields::ROOM_ID)?, 8);
    Ok(())
}

#[test]
fn checkpoint_interact_when_transform_and_room_present_requests_canonical_location() -> mlua::Result<()> {
    let lua = Lua::new();
    let capture_calls = lua.create_table()?;
    let request_calls = lua.create_table()?;
    let save_flow = lua.create_table()?;
    let capture_calls_for_location = capture_calls.clone();
    save_flow
        .set(
            "capture_location",
            lua.create_function(move |lua, (transform, room_id): (Table, i64)| {
                capture_calls_for_location.set("transform", transform)?;
                capture_calls_for_location.set(lua_fields::ROOM_ID, room_id)?;
                let location = lua.create_table()?;
                location.set("source", "capture_location")?;
                Ok(location)
            })
            ?,
        )
        ?;
    let request_calls_for_checkpoint = request_calls.clone();
    save_flow
        .set(
            "request_checkpoint",
            lua.create_function(move |_, location: Table| {
                request_calls_for_checkpoint.set("location", location)?;
                Ok(())
            })
            ?,
        )
        ?;
    register_test_module(&lua, "save_flow", save_flow)?;

    let engine = lua.create_table()?;
    let log = lua.create_table()?;
    log.set("info", lua.create_function(|_, _msg: String| Ok(()))?)
        ?;
    engine.set(lua_engine::LOG, log)?;
    lua.globals().set(lua_engine::ENGINE, engine)?;

    let components = lua.create_table()?;
    components.set("Transform", "Transform")?;
    lua.globals().set("Components", components)?;

    let position = lua.create_table()?;
    position.set(lua_fields::X, 12.0)?;
    position.set(lua_fields::Y, 24.0)?;
    let transform = lua.create_table()?;
    transform.set(lua_fields::POSITION, position)?;
    let entity = lua.create_table()?;
    entity
        .set(
            "get",
            lua.create_function(move |_, (_entity, _component): (Table, mlua::Value)| {
                Ok(transform.clone())
            })
            ?,
        )
        ?;
    entity
        .set(
            "current_room",
            lua.create_function(|_, (_entity,): (Table,)| Ok(5))
                ?,
        )
        ?;

    let script = load_demo_script(&lua, "checkpoint.lua");
    script.set("entity", entity)?;

    let interact: mlua::Function = script.get("interact")?;
    interact.call::<()>(script.clone())?;

    assert_eq!(capture_calls.get::<i64>(lua_fields::ROOM_ID)?, 5);
    let location: Table = request_calls.get("location")?;
    assert_eq!(location.get::<String>("source")?, "capture_location");
    Ok(())
}
