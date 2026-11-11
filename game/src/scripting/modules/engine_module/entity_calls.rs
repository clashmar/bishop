use crate::scripting::lua_ctx::LuaGameCtx;
use engine_core::ecs::{CurrentRoom, Entity, Global, Name, Script};
use engine_core::game::Game;
use engine_core::scripting::lua_constants::lua_engine;
use mlua::prelude::LuaResult;
use mlua::{Function, Lua, MultiValue, Table, Value, Variadic};

pub(super) fn register_entity_calls(lua: &Lua, engine: &Table) -> LuaResult<()> {
    let call_named = lua.create_function(|lua, args: Variadic<Value>| {
        let mut args = args.into_iter();
        let (name, method) = name_and_method(&mut args)?;
        let ctx = LuaGameCtx::borrow_ctx(lua)?;
        let game_instance = ctx.game_instance.borrow();
        let game = &game_instance.game;
        let entity = entity_by_name(game, &name)?;
        call_script(game, entity, &name, &method, args.collect())
    })?;
    engine.set(lua_engine::CALL, call_named)?;

    Ok(())
}

fn name_and_method(args: &mut impl Iterator<Item = Value>) -> LuaResult<(String, String)> {
    let name = match args.next() {
        Some(Value::String(value)) => value.to_str()?.to_string(),
        _ => {
            return Err(mlua::Error::RuntimeError(
                "Entity name must be a string".to_string(),
            ))
        }
    };
    let method = match args.next() {
        Some(Value::String(value)) => value.to_str()?.to_string(),
        _ => {
            return Err(mlua::Error::RuntimeError(
                "Method name must be a string".to_string(),
            ))
        }
    };
    Ok((name, method))
}

fn entity_by_name(game: &Game, name: &str) -> LuaResult<Entity> {
    let ecs = &game.ecs;
    let name_store = ecs.get_store::<Name>();
    let room_store = ecs.get_store::<CurrentRoom>();
    let global_store = ecs.get_store::<Global>();
    let active_room_id = game.current_world().current_room_id;
    let mut matches = name_store.data.keys().copied().filter(|entity| {
        name_store.get(*entity).is_some_and(|entry| entry.0 == name)
            && (global_store.get(*entity).is_some()
                || active_room_id.is_some_and(|room_id| {
                    room_store
                        .get(*entity)
                        .is_some_and(|current| current.room_id == room_id)
                }))
    });
    let entity = matches.next().ok_or_else(|| {
        mlua::Error::RuntimeError(format!("Entity '{name}' not found in active room or globals"))
    })?;
    if matches.next().is_some() {
        return Err(mlua::Error::RuntimeError(format!(
            "Entity '{name}' is ambiguous in active room and globals"
        )));
    }
    Ok(entity)
}

fn call_script(
    game: &Game,
    entity: Entity,
    name: &str,
    method: &str,
    args: Vec<Value>,
) -> LuaResult<MultiValue> {
    let script_id = game
        .ecs
        .get::<Script>(entity)
        .map(|script| script.script_id)
        .ok_or_else(|| {
            mlua::Error::RuntimeError(format!("Entity '{name}' has no script"))
        })?;
    let instance = game
        .script_manager
        .instances
        .get(&(entity, script_id))
        .ok_or_else(|| {
            mlua::Error::RuntimeError(format!(
                "Script instance not found for entity '{name}'"
            ))
        })?;
    let func = instance.get::<Function>(method).map_err(|_| {
        mlua::Error::RuntimeError(format!("Method '{method}' not found on entity '{name}'"))
    })?;
    let mut call_args = Vec::with_capacity(args.len() + 1);
    call_args.push(Value::Table(instance.clone()));
    call_args.extend(args);
    func.call::<MultiValue>(MultiValue::from_vec(call_args))
}
