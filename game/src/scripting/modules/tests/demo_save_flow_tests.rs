use engine_core::constants::paths;
use engine_core::scripting::EventBus;
use engine_core::scripting::lua_constants::{
    lua_engine, lua_events, lua_fields, lua_globals, lua_save,
};
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
    engine
        .set(
            lua_engine::CURRENT_WORLD,
            lua.create_function(|lua, ()| {
                let world = lua.create_table()?;
                world.set(lua_fields::ID, 42)?;
                Ok(world)
            })
            .unwrap(),
        )
        .unwrap();
    engine
        .set(
            lua_engine::RESTORE_LOCATION,
            lua.create_function(|lua, payload: Table| {
                lua.globals().set("captured_restore", payload)?;
                Ok(())
            })
            .unwrap(),
        )
        .unwrap();
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

fn demo_scripts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(paths::GAME_SAVE_ROOT)
        .join(paths::DEMO_GAME)
        .join(paths::RESOURCES_FOLDER)
        .join(paths::SCRIPTS_FOLDER)
}

pub(super) fn load_demo_script(lua: &Lua, file_name: &str) -> Table {
    let path = demo_scripts_dir().join(file_name);
    let src = fs::read_to_string(&path).unwrap();
    lua.load(&src)
        .set_name(file_name)
        .eval::<Table>()
        .unwrap()
}

fn load_save_flow(lua: &Lua) -> Table {
    load_demo_script(lua, "save_flow.lua")
}

#[test]
fn save_flow_capture_location_when_transform_and_room_present_returns_world_room_and_position() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let position = lua.create_table().unwrap();
    position.set(lua_fields::X, 12.0).unwrap();
    position.set(lua_fields::Y, 34.0).unwrap();
    let transform = lua.create_table().unwrap();
    transform.set(lua_fields::POSITION, position).unwrap();

    let capture: mlua::Function = flow.get("capture_location").unwrap();
    let captured: Table = capture.call((transform, 7)).unwrap();

    assert_eq!(captured.get::<i64>(lua_fields::WORLD_ID).unwrap(), 42);
    assert_eq!(captured.get::<i64>(lua_fields::ROOM_ID).unwrap(), 7);
    assert_eq!(captured.get::<f32>(lua_fields::X).unwrap(), 12.0);
    assert_eq!(captured.get::<f32>(lua_fields::Y).unwrap(), 34.0);
}

#[test]
fn save_flow_apply_restore_target_when_called_forwards_single_payload_to_engine_restore_location() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let restore = lua.create_table().unwrap();
    restore.set(lua_fields::WORLD_ID, 3).unwrap();
    restore.set(lua_fields::ROOM_ID, 8).unwrap();
    restore.set(lua_fields::X, 50.0).unwrap();
    restore.set(lua_fields::Y, 60.0).unwrap();

    let apply: mlua::Function = flow.get("apply_restore_target").unwrap();
    apply.call::<()>(restore).unwrap();

    let captured: Table = lua.globals().get("captured_restore").unwrap();
    assert_eq!(captured.get::<i64>(lua_fields::WORLD_ID).unwrap(), 3);
    assert_eq!(captured.get::<i64>(lua_fields::ROOM_ID).unwrap(), 8);
    assert_eq!(captured.get::<f32>(lua_fields::X).unwrap(), 50.0);
    assert_eq!(captured.get::<f32>(lua_fields::Y).unwrap(), 60.0);
}

#[test]
fn save_flow_checkpoint_success_commits_pending_anchor() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let anchor = lua.create_table().unwrap();
    anchor.set(lua_fields::ROOM_ID, 7).unwrap();
    anchor.set(lua_fields::X, 12.0).unwrap();
    anchor.set(lua_fields::Y, 34.0).unwrap();

    let begin: mlua::Function = flow.get("begin_checkpoint").unwrap();
    let succeed: mlua::Function = flow.get("handle_save_succeeded").unwrap();
    let active: mlua::Function = flow.get("active_anchor").unwrap();

    begin.call::<()>(anchor.clone()).unwrap();
    succeed.call::<()>(lua_save::CHECKPOINT).unwrap();

    let committed: Table = active.call(()).unwrap();
    assert_eq!(committed.get::<i64>(lua_fields::ROOM_ID).unwrap(), 7);
}

#[test]
fn save_flow_checkpoint_failure_clears_pending_anchor_without_advancing_active_anchor() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let first = lua.create_table().unwrap();
    first.set(lua_fields::ROOM_ID, 2).unwrap();
    first.set(lua_fields::X, 1.0).unwrap();
    first.set(lua_fields::Y, 2.0).unwrap();
    let second = lua.create_table().unwrap();
    second.set(lua_fields::ROOM_ID, 9).unwrap();
    second.set(lua_fields::X, 8.0).unwrap();
    second.set(lua_fields::Y, 7.0).unwrap();

    let set_active: mlua::Function = flow.get("set_active_anchor").unwrap();
    let begin: mlua::Function = flow.get("begin_checkpoint").unwrap();
    let fail: mlua::Function = flow.get("handle_save_failed").unwrap();
    let active: mlua::Function = flow.get("active_anchor").unwrap();

    set_active.call::<()>(first.clone()).unwrap();
    begin.call::<()>(second.clone()).unwrap();
    fail.call::<()>(lua_save::CHECKPOINT).unwrap();

    let committed: Table = active.call(()).unwrap();
    assert_eq!(committed.get::<i64>(lua_fields::ROOM_ID).unwrap(), 2);
}

#[test]
fn save_flow_build_save_document_keeps_snapshot_and_anchor_separate() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);

    let anchor = lua.create_table().unwrap();
    anchor.set(lua_fields::ROOM_ID, 11).unwrap();
    anchor.set(lua_fields::X, 1.0).unwrap();
    anchor.set(lua_fields::Y, 2.0).unwrap();

    let snapshot = lua.create_table().unwrap();
    snapshot.set(lua_fields::ROOM_ID, 17).unwrap();
    snapshot.set(lua_fields::X, 30.0).unwrap();
    snapshot.set(lua_fields::Y, 40.0).unwrap();

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

    assert_eq!(saved_snapshot.get::<i64>(lua_fields::ROOM_ID).unwrap(), 17);
    assert_eq!(saved_anchor.get::<i64>(lua_fields::ROOM_ID).unwrap(), 11);
}

#[test]
fn save_flow_bind_runtime_handlers_when_save_succeeded_event_emitted_commits_pending_anchor() {
    let (lua, event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);

    let anchor = lua.create_table().unwrap();
    anchor.set(lua_fields::ROOM_ID, 21).unwrap();
    anchor.set(lua_fields::X, 4.0).unwrap();
    anchor.set(lua_fields::Y, 5.0).unwrap();

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
    assert_eq!(committed.get::<i64>(lua_fields::ROOM_ID).unwrap(), 21);
}

#[test]
fn save_flow_resolve_restore_target_prefers_committed_anchor_then_snapshot() {
    let (lua, _event_bus) = setup_save_flow_lua();
    let flow = load_save_flow(&lua);
    let resolve: mlua::Function = flow.get("resolve_restore_target").unwrap();

    let snapshot = lua.create_table().unwrap();
    snapshot.set(lua_fields::ROOM_ID, 3).unwrap();
    snapshot.set(lua_fields::X, 30.0).unwrap();
    snapshot.set(lua_fields::Y, 40.0).unwrap();

    let with_snapshot_only = lua.create_table().unwrap();
    with_snapshot_only.set("snapshot", snapshot.clone()).unwrap();
    let chosen_snapshot: Table = resolve.call(with_snapshot_only).unwrap();
    assert_eq!(chosen_snapshot.get::<i64>(lua_fields::ROOM_ID).unwrap(), 3);

    let anchor = lua.create_table().unwrap();
    anchor.set(lua_fields::ROOM_ID, 5).unwrap();
    anchor.set(lua_fields::X, 50.0).unwrap();
    anchor.set(lua_fields::Y, 60.0).unwrap();

    let with_anchor = lua.create_table().unwrap();
    with_anchor.set("snapshot", snapshot).unwrap();
    with_anchor.set("active_anchor", anchor).unwrap();
    let chosen_anchor: Table = resolve.call(with_anchor).unwrap();
    assert_eq!(chosen_anchor.get::<i64>(lua_fields::ROOM_ID).unwrap(), 5);
}
