use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::lua_entity_handle;
use crate::scripting::script_system::ScriptSystem;
use bishop::prelude::*;
use engine_core::prelude::*;
use engine_core::scripting::lua_constants::{
    ENGINE, ENGINE_FILE, POSITION, PREFAB, SPAWN, X, Y,
};
use engine_core::{register_lua_api, register_lua_module};
use mlua::prelude::LuaResult;
use mlua::{Lua, MultiValue, Table, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
struct SpawnOptions {
    position: Vec2,
}

/// Lua module that exposes runtime prefab spawning under `engine.prefab`.
#[derive(Default)]
pub struct PrefabModule;
register_lua_module!(PrefabModule);
register_lua_api!(PrefabModule, ENGINE_FILE);

impl LuaModule for PrefabModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(ENGINE)?;
        let prefab_tbl = lua.create_table()?;

        let spawn_fn = lua.create_function(
            |lua, (prefab_name, position, init): (String, Table, Option<Table>)| {
                let (spawn, spawn_args) = parse_spawn_args(lua, &prefab_name, position, init)?;
                let spawned_entity =
                    spawn_prefab(lua, &prefab_name, spawn.position, spawn_args)?;
                lua_entity_handle(lua, spawned_entity)
            },
        )?;

        prefab_tbl.set(SPAWN, spawn_fn)?;
        engine_tbl.set(PREFAB, prefab_tbl)?;
        Ok(())
    }
}

impl LuaApi for PrefabModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("engine.prefab = {}");
        out.line("");
        out.line("---@param prefab_name PrefabId");
        out.line("---@param position vec2");
        out.line("---@param init? table");
        out.line("---@return Entity");
        out.line("function engine.prefab.spawn(prefab_name, position, init) end");
        out.line("");
    }
}

fn parse_spawn_args(
    _lua: &Lua,
    prefab_name: &str,
    position: Table,
    init: Option<Table>,
) -> LuaResult<(SpawnOptions, Option<Value>)> {
    let x = position.get::<f32>(X).map_err(|_| {
        mlua::Error::RuntimeError(format!(
            "engine.prefab.spawn({prefab_name}) requires {POSITION} = {{ {X} = number, {Y} = number }}"
        ))
    })?;
    let y = position.get::<f32>(Y).map_err(|_| {
        mlua::Error::RuntimeError(format!(
            "engine.prefab.spawn({prefab_name}) requires {POSITION} = {{ {X} = number, {Y} = number }}"
        ))
    })?;

    if (1..=3).any(|index| {
        matches!(
            position.get::<Value>(index).ok(),
            Some(Value::Number(_) | Value::Integer(_))
        )
    }) {
        return Err(mlua::Error::RuntimeError(format!(
            "engine.prefab.spawn({prefab_name}) requires {POSITION} = {{ {X} = number, {Y} = number }}"
        )));
    }

    let spawn_args = init.map(Value::Table);

    Ok((
        SpawnOptions {
            position: Vec2::new(x, y),
        },
        spawn_args,
    ))
}

fn prefab_root_supports_spawn_args(prefab: &PrefabAsset) -> bool {
    let Some(root_node) = prefab
        .nodes
        .iter()
        .find(|node| node.node_id == prefab.root_node_id)
    else {
        return false;
    };

    root_node
        .components
        .iter()
        .any(|component| component.type_name == "Script")
}

fn spawn_prefab(
    lua: &Lua,
    prefab_name: &str,
    position: Vec2,
    spawn_args: Option<Value>,
) -> LuaResult<Entity> {
    let ctx = LuaGameCtx::borrow_ctx(lua)?;
    let root_entity = {
        let mut game_instance = ctx.game_instance.borrow_mut();
        let prefab = game_instance
            .game
            .prefab_library
            .prefab_named(prefab_name)
            .cloned()
            .ok_or_else(|| {
                mlua::Error::RuntimeError(format!("Unknown prefab '{prefab_name}'"))
            })?;
        if spawn_args.is_some() && !prefab_root_supports_spawn_args(&prefab) {
            return Err(mlua::Error::RuntimeError(
                "engine.prefab.spawn init requires a Script on the prefab root".into(),
            ));
        }
        let room_id = game_instance
            .game
            .current_world()
            .current_room_id
            .ok_or_else(|| {
                mlua::Error::RuntimeError(
                    "engine.prefab.spawn requires an active current room".into(),
                )
            })?;
        let mut game_ctx = game_instance.game.ctx_mut();
        instantiate_prefab(&mut game_ctx, &prefab, position, Some(room_id))
    };

    if root_entity == Entity::null() {
        return Err(mlua::Error::RuntimeError(
            "Prefab instantiation failed".into(),
        ));
    }

    let inits_to_run_result = {
        let mut game_instance = ctx.game_instance.borrow_mut();
        let game = &mut game_instance.game;
        ScriptSystem::prepare_spawned_script_inits(
            lua,
            &mut game.ecs,
            &mut game.script_manager,
            root_entity,
            spawn_args,
        )
    };

    let inits_to_run = match inits_to_run_result {
        Ok(inits) => inits,
        Err(error) => {
            let mut game_instance = ctx.game_instance.borrow_mut();
            let mut game_ctx = game_instance.game.ctx_mut();
            Ecs::remove_entity(&mut game_ctx, root_entity);
            return Err(error);
        }
    };

    for (init_fn, instance, args) in inits_to_run {
        let mut call_args = vec![Value::Table(instance)];
        if let Some(value) = args {
            call_args.push(value);
        }
        if let Err(error) = init_fn.call::<()>(MultiValue::from_vec(call_args)) {
            let mut game_instance = ctx.game_instance.borrow_mut();
            let mut game_ctx = game_instance.game.ctx_mut();
            Ecs::remove_entity(&mut game_ctx, root_entity);
            return Err(error);
        }
    }

    Ok(root_entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spawn_args_reads_named_position_and_init_table() {
        let lua = Lua::new();
        let position = lua.create_table().unwrap();
        position.set(X, 12.5).unwrap();
        position.set(Y, -3.0).unwrap();
        let init = lua.create_table().unwrap();
        init.set("direction", "left").unwrap();

        let (spawn, parsed_init) =
            parse_spawn_args(&lua, "Bullet", position, Some(init)).unwrap();

        assert_eq!(spawn.position, Vec2::new(12.5, -3.0));
        assert!(matches!(parsed_init, Some(Value::Table(_))));
    }

    #[test]
    fn parse_spawn_args_rejects_indexed_position_tables() {
        let lua = Lua::new();
        let position = lua.create_table().unwrap();
        position.set(1, 12.5).unwrap();
        position.set(2, -3.0).unwrap();

        let error = parse_spawn_args(&lua, "Bullet", position, None).unwrap_err();

        assert!(error.to_string().contains("position"));
    }

    #[test]
    fn prefab_root_supports_spawn_args_only_when_root_has_script_component() {
        let prefab = PrefabAsset {
            id: PrefabId(1),
            name: "Bullet".to_string(),
            next_node_id: 2,
            root_node_id: 1,
            nodes: vec![PrefabNode {
                node_id: 1,
                parent_node_id: None,
                components: vec![],
            }],
        };
        let scripted_prefab = PrefabAsset {
            nodes: vec![PrefabNode {
                components: vec![ComponentSnapshot {
                    type_name: "Script".to_string(),
                    ron: "(script_id:1,data:(fields:{}))".to_string(),
                }],
                ..prefab.nodes[0].clone()
            }],
            ..prefab.clone()
        };

        assert!(!prefab_root_supports_spawn_args(&prefab));
        assert!(prefab_root_supports_spawn_args(&scripted_prefab));
    }
}
