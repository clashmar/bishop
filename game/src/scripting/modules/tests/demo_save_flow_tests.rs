use engine_core::constants::paths;
use engine_core::scripting::EventBus;
use engine_core::scripting::lua_constants::{lua_engine, lua_events, lua_globals, lua_save};
use mlua::{Lua, Table, Variadic};
use std::fs;
use std::path::PathBuf;

fn setup_save_flow_lua() -> (Lua, EventBus) {
    let lua = Lua::new();
    let event_bus = EventBus::default();
    let engine = lua.create_table().unwrap();
    let save = lua.create_table().unwrap();
    let triggers = lua.create_table().unwrap();
    let events = lua.create_table().unwrap();

    triggers.set(lua_save::CHECKPOINT, lua_save::CHECKPOINT).unwrap();
    save.set(lua_save::MANUAL, lua.create_function(|_, ()| Ok(())).unwrap()).unwrap();
    save.set(lua_save::AUTO, lua.create_function(|_, ()| Ok(())).unwrap()).unwrap();
    save.set(lua_save::CHECKPOINT, lua.create_function(|_, ()| Ok(())).unwrap()).unwrap();
    save.set(lua_save::TRIGGERS, triggers).unwrap();
    engine.set(lua_save::SAVE, save).unwrap();
    events
        .set(lua_events::SAVE_SUCCEEDED_FIELD, lua_events::SAVE_SUCCEEDED)
        .unwrap();
    events
        .set(lua_events::SAVE_FAILED_FIELD, lua_events::SAVE_FAILED)
        .unwrap();
    engine.set(lua_events::EVENTS, events).unwrap();
    lua.globals().set(lua_engine::ENGINE, engine).unwrap();
    lua.globals().set(lua_globals::LUA_EVENT_BUS, event_bus.clone()).unwrap();
    lua.globals()
        .get::<Table>(lua_engine::ENGINE)
        .unwrap()
        .set(
            lua_engine::ON,
            lua.create_function(|lua, (event, handler): (String, mlua::Function)| {
                let bus: mlua::AnyUserData = lua.globals().get(lua_globals::LUA_EVENT_BUS)?;
                bus.borrow::<EventBus>()?.on(event, handler);
                Ok(())
            })
            .unwrap(),
        )
        .unwrap();

    (lua, event_bus)
}

fn load_save_flow(lua: &Lua) -> Table {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let path = workspace_root
        .join(paths::GAME_SAVE_ROOT)
        .join(paths::DEMO_GAME)
        .join(paths::RESOURCES_FOLDER)
        .join(paths::SCRIPTS_FOLDER)
        .join("save_flow.lua");
    let src = fs::read_to_string(path).unwrap();
    lua.load(&src)
        .set_name("save_flow.lua")
        .eval::<Table>()
        .unwrap()
}

#[test]
fn save_flow_checkpoint_success_commits_pending_anchor() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let anchor = lua.create_table().unwrap();
    anchor.set("room_id", 7).unwrap();
    anchor.set("x", 12.0).unwrap();
    anchor.set("y", 34.0).unwrap();

    let begin: mlua::Function = flow.get("begin_checkpoint").unwrap();
    let succeed: mlua::Function = flow.get("handle_save_succeeded").unwrap();
    let active: mlua::Function = flow.get("active_anchor").unwrap();

    begin.call::<()>(anchor.clone()).unwrap();
    succeed.call::<()>(lua_save::CHECKPOINT).unwrap();

    let committed: Table = active.call(()).unwrap();
    assert_eq!(committed.get::<i64>("room_id").unwrap(), 7);
}

#[test]
fn save_flow_checkpoint_failure_clears_pending_anchor_without_advancing_active_anchor() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let first = lua.create_table().unwrap();
    first.set("room_id", 2).unwrap();
    first.set("x", 1.0).unwrap();
    first.set("y", 2.0).unwrap();
    let second = lua.create_table().unwrap();
    second.set("room_id", 9).unwrap();
    second.set("x", 8.0).unwrap();
    second.set("y", 7.0).unwrap();

    let set_active: mlua::Function = flow.get("set_active_anchor").unwrap();
    let begin: mlua::Function = flow.get("begin_checkpoint").unwrap();
    let fail: mlua::Function = flow.get("handle_save_failed").unwrap();
    let active: mlua::Function = flow.get("active_anchor").unwrap();

    set_active.call::<()>(first.clone()).unwrap();
    begin.call::<()>(second.clone()).unwrap();
    fail.call::<()>(lua_save::CHECKPOINT).unwrap();

    let committed: Table = active.call(()).unwrap();
    assert_eq!(committed.get::<i64>("room_id").unwrap(), 2);
}

#[test]
fn save_flow_build_save_document_keeps_snapshot_and_anchor_separate() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);

    let anchor = lua.create_table().unwrap();
    anchor.set("room_id", 11).unwrap();
    anchor.set("x", 1.0).unwrap();
    anchor.set("y", 2.0).unwrap();

    let snapshot = lua.create_table().unwrap();
    snapshot.set("room_id", 17).unwrap();
    snapshot.set("x", 30.0).unwrap();
    snapshot.set("y", 40.0).unwrap();

    let progress = lua.create_table().unwrap();
    progress.set("score", 99).unwrap();
    progress.set("level", 3).unwrap();
    progress.set("health", 80).unwrap();

    let set_active: mlua::Function = flow.get("set_active_anchor").unwrap();
    let build: mlua::Function = flow.get("build_save_document").unwrap();

    set_active.call::<()>(anchor).unwrap();
    let saved: Table = build.call((progress, snapshot)).unwrap();

    let saved_snapshot: Table = saved.get("snapshot").unwrap();
    let saved_anchor: Table = saved.get("active_anchor").unwrap();

    assert_eq!(saved_snapshot.get::<i64>("room_id").unwrap(), 17);
    assert_eq!(saved_anchor.get::<i64>("room_id").unwrap(), 11);
}

#[test]
fn save_flow_bind_runtime_handlers_when_save_succeeded_event_emitted_commits_pending_anchor() {
    let (lua, event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);

    let anchor = lua.create_table().unwrap();
    anchor.set("room_id", 21).unwrap();
    anchor.set("x", 4.0).unwrap();
    anchor.set("y", 5.0).unwrap();

    let bind: mlua::Function = flow.get("bind_runtime_handlers").unwrap();
    let begin: mlua::Function = flow.get("begin_checkpoint").unwrap();
    let active: mlua::Function = flow.get("active_anchor").unwrap();

    bind.call::<()>(()).unwrap();
    begin.call::<()>(anchor).unwrap();

    let lane = lua.create_string("autosave").unwrap();
    let trigger = lua.create_string(lua_save::CHECKPOINT).unwrap();
    event_bus.emit(
        lua_events::SAVE_SUCCEEDED.to_string(),
        Variadic::from_iter([mlua::Value::String(lane), mlua::Value::String(trigger)]),
    );

    let committed: Table = active.call(()).unwrap();
    assert_eq!(committed.get::<i64>("room_id").unwrap(), 21);
}

#[test]
fn save_flow_resolve_restore_target_prefers_committed_anchor_then_snapshot() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let resolve: mlua::Function = flow.get("resolve_restore_target").unwrap();

    let snapshot = lua.create_table().unwrap();
    snapshot.set("room_id", 3).unwrap();
    snapshot.set("x", 30.0).unwrap();
    snapshot.set("y", 40.0).unwrap();

    let with_snapshot_only = lua.create_table().unwrap();
    with_snapshot_only.set("snapshot", snapshot.clone()).unwrap();
    let chosen_snapshot: Table = resolve.call(with_snapshot_only).unwrap();
    assert_eq!(chosen_snapshot.get::<i64>("room_id").unwrap(), 3);

    let anchor = lua.create_table().unwrap();
    anchor.set("room_id", 5).unwrap();
    anchor.set("x", 50.0).unwrap();
    anchor.set("y", 60.0).unwrap();

    let with_anchor = lua.create_table().unwrap();
    with_anchor.set("snapshot", snapshot).unwrap();
    with_anchor.set("active_anchor", anchor).unwrap();
    let chosen_anchor: Table = resolve.call(with_anchor).unwrap();
    assert_eq!(chosen_anchor.get::<i64>("room_id").unwrap(), 5);
}
