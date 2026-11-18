use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::{EntityHandle, lua_entity_handle};
use engine_core::ecs::{CurrentRoom, Transform};
use engine_core::physics::{RaycastHit, RaycastQuery, raycast};
use engine_core::scripting::helpers::{parse_named_vec2, write_named_vec2_table};
use engine_core::scripting::lua_constants::{lua_engine, lua_files, lua_spatial};
use engine_core::scripting::{LuaApi, LuaApiWriter, LuaModule};
use engine_core::{register_lua_api, register_lua_module};
use mlua::prelude::LuaResult;
use mlua::{AnyUserData, Lua, Table};
use std::collections::HashSet;

/// Lua module that exposes spatial world queries.
#[derive(Default)]
pub struct SpatialModule;
register_lua_module!(SpatialModule);
register_lua_api!(SpatialModule, lua_files::SPATIAL);

impl LuaModule for SpatialModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
        let spatial_tbl = lua.create_table()?;

        let raycast_fn = lua.create_function(|lua, options: Table| {
            let query = raycast_query_from_lua(options)?;
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let hit = raycast(&game_instance.game.ecs, &query);
            raycast_result_to_lua(lua, hit)
        })?;
        spatial_tbl.set(lua_spatial::RAYCAST, raycast_fn)?;

        let line_of_sight_fn = lua.create_function(
            |lua, (from_entity, to_entity, options): (AnyUserData, AnyUserData, Option<Table>)| {
                let from_entity = from_entity.borrow::<EntityHandle>()?.clone();
                let to_entity = to_entity.borrow::<EntityHandle>()?.clone();
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                let ecs = &game_instance.game.ecs;
                let from_transform = ecs.get::<Transform>(from_entity.entity).ok_or_else(|| {
                    mlua::Error::RuntimeError("from_entity has no Transform component".into())
                })?;
                let to_transform = ecs.get::<Transform>(to_entity.entity).ok_or_else(|| {
                    mlua::Error::RuntimeError("to_entity has no Transform component".into())
                })?;
                let current_room = ecs.get::<CurrentRoom>(from_entity.entity);
                let mut ignore = HashSet::new();
                ignore.insert(from_entity.entity);
                if let Some(options) = options {
                    if let Some(extra_ignored) = ignored_entity(&options)? {
                        ignore.insert(extra_ignored.entity);
                    }
                }
                let query = RaycastQuery {
                    from: from_transform.position,
                    to: to_transform.position,
                    room_id: current_room.map(|room| room.room_id),
                    layer: current_room.map(|room| room.layer),
                    ignore,
                    include_sensors: false,
                };

                Ok(raycast(ecs, &query).is_none_or(|hit| hit.entity == to_entity.entity))
            },
        )?;
        spatial_tbl.set(lua_spatial::HAS_LINE_OF_SIGHT, line_of_sight_fn)?;

        engine_tbl.set(lua_engine::SPATIAL, spatial_tbl)?;
        Ok(())
    }
}

impl LuaApi for SpatialModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("engine.spatial = {}");
        out.line("");
        out.line("---@param options table");
        out.line("---@return table");
        out.line(&format!(
            "function engine.{}.{}(options) end",
            lua_engine::SPATIAL,
            lua_spatial::RAYCAST
        ));
        out.line("");
        out.line("---@param from_entity Entity");
        out.line("---@param to_entity Entity");
        out.line("---@param options? table");
        out.line("---@return boolean");
        out.line(&format!(
            "function engine.{}.{}(from_entity, to_entity, options) end",
            lua_engine::SPATIAL,
            lua_spatial::HAS_LINE_OF_SIGHT
        ));
    }
}

fn raycast_query_from_lua(options: Table) -> LuaResult<RaycastQuery> {
    let from_table: Table = options.get(lua_spatial::FROM)?;
    let to_table: Table = options.get(lua_spatial::TO)?;
    let mut ignore = HashSet::new();
    if let Some(ignored) = ignored_entity(&options)? {
        ignore.insert(ignored.entity);
    }

    Ok(RaycastQuery {
        from: parse_named_vec2(&from_table, "engine.spatial.raycast from")?,
        to: parse_named_vec2(&to_table, "engine.spatial.raycast to")?,
        room_id: None,
        layer: None,
        ignore,
        include_sensors: false,
    })
}

fn ignored_entity(options: &Table) -> LuaResult<Option<EntityHandle>> {
    let Some(user_data) = options.get::<Option<AnyUserData>>(lua_spatial::IGNORE)? else {
        return Ok(None);
    };
    Ok(Some(user_data.borrow::<EntityHandle>()?.clone()))
}

fn raycast_result_to_lua(lua: &Lua, hit: Option<RaycastHit>) -> LuaResult<Table> {
    let table = lua.create_table()?;
    let Some(hit) = hit else {
        table.set(lua_spatial::HIT, false)?;
        return Ok(table);
    };

    table.set(lua_spatial::HIT, true)?;
    table.set(lua_spatial::ENTITY, lua_entity_handle(lua, hit.entity)?)?;
    table.set(lua_spatial::POINT, write_named_vec2_table(lua, hit.point)?)?;
    table.set(lua_spatial::NORMAL, write_named_vec2_table(lua, hit.normal)?)?;
    table.set(lua_spatial::DISTANCE, hit.distance)?;
    Ok(table)
}
