use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::lua_entity_handle;
use crate::scripting::script_system::ScriptSystem;
use bishop::prelude::*;
use engine_core::prelude::*;
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

        let spawn_fn = lua.create_function(|lua, (prefab_name, options): (String, Table)| {
            let (spawn, spawn_args) = parse_spawn_options(lua, options)?;
            let spawned_entity = spawn_prefab(lua, &prefab_name, spawn.position, spawn_args)?;
            lua_entity_handle(lua, spawned_entity)
        })?;

        prefab_tbl.set("spawn", spawn_fn)?;
        engine_tbl.set("prefab", prefab_tbl)?;
        Ok(())
    }
}

impl LuaApi for PrefabModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("engine.prefab = {}");
        out.line("");
        out.line("---@param prefab_name PrefabId|string");
        out.line("---@param opts { position: { x: number, y: number }, args?: table }");
        out.line("---@return Entity");
        out.line("function engine.prefab.spawn(prefab_name, opts) end");
        out.line("");
    }
}

fn parse_spawn_options(_lua: &Lua, options: Table) -> LuaResult<(SpawnOptions, Option<Value>)> {
    let position: Table = options.get("position").map_err(|_| {
        mlua::Error::RuntimeError(
            "engine.prefab.spawn requires opts.position = { x = number, y = number }".into(),
        )
    })?;
    let x = position.get::<f32>("x").map_err(|_| {
        mlua::Error::RuntimeError("engine.prefab.spawn opts.position.x must be a number".into())
    })?;
    let y = position.get::<f32>("y").map_err(|_| {
        mlua::Error::RuntimeError("engine.prefab.spawn opts.position.y must be a number".into())
    })?;

    let spawn_args = match options.get::<Value>("args")? {
        Value::Nil => None,
        Value::Table(table) => Some(Value::Table(table)),
        _ => {
            return Err(mlua::Error::RuntimeError(
                "engine.prefab.spawn opts.args must be a table when provided".into(),
            ))
        }
    };

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
                "engine.prefab.spawn opts.args requires a Script on the prefab root".into(),
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

    let inits_to_run = {
        let mut game_instance = ctx.game_instance.borrow_mut();
        let game = &mut game_instance.game;
        ScriptSystem::prepare_spawned_script_inits(
            lua,
            &mut game.ecs,
            &mut game.script_manager,
            root_entity,
            spawn_args,
        )?
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
    fn parse_spawn_options_reads_position_and_args_table() {
        let lua = Lua::new();
        let options = lua.create_table().unwrap();
        let position = lua.create_table().unwrap();
        let args = lua.create_table().unwrap();
        position.set("x", 12.5).unwrap();
        position.set("y", -3.0).unwrap();
        args.set("direction", "left").unwrap();
        options.set("position", position).unwrap();
        options.set("args", args.clone()).unwrap();

        let (spawn, parsed_args) = parse_spawn_options(&lua, options).unwrap();

        assert_eq!(spawn.position, Vec2::new(12.5, -3.0));
        assert!(matches!(parsed_args, Some(Value::Table(_))));
    }

    #[test]
    fn parse_spawn_options_requires_position() {
        let lua = Lua::new();
        let options = lua.create_table().unwrap();

        let error = parse_spawn_options(&lua, options).unwrap_err();

        assert!(error.to_string().contains("position"));
    }

    #[test]
    fn parse_spawn_options_rejects_non_table_args() {
        let lua = Lua::new();
        let options = lua.create_table().unwrap();
        let position = lua.create_table().unwrap();
        position.set("x", 1.0).unwrap();
        position.set("y", 2.0).unwrap();
        options.set("position", position).unwrap();
        options.set("args", 7).unwrap();

        let error = parse_spawn_options(&lua, options).unwrap_err();

        assert!(error.to_string().contains("args"));
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
