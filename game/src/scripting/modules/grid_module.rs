use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::EntityHandle;
use engine_core::ecs::{CurrentRoom, Ecs, Entity, Pivot, Transform};
use engine_core::scripting::helpers::parse_named_vec2;
use engine_core::scripting::lua_constants::{lua_engine, lua_files, lua_grid};
use engine_core::scripting::{LuaApi, LuaApiWriter, LuaModule};
use engine_core::worlds::room::grid_navigation::{
    GridCell, GridPathResult, GridQueryStatus, can_step, cell_at, find_path,
};
use engine_core::worlds::{Room, RoomLayer, World};
use engine_core::{register_lua_api, register_lua_module};
use mlua::prelude::LuaResult;
use mlua::{AnyUserData, Lua, Table, Value};

const DEFAULT_MAX_STEPS: usize = 40;

/// Lua module that exposes grid occupancy and path queries.
#[derive(Default)]
pub struct GridModule;
register_lua_module!(GridModule);
register_lua_api!(GridModule, lua_files::GRID);

impl LuaModule for GridModule {
    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let engine_tbl: Table = lua.globals().get(lua_engine::ENGINE)?;
        let grid_tbl = lua.create_table()?;

        let cell_at_fn = lua.create_function(|lua, position: Table| {
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let grid_size = game_instance.game.current_world().grid_size;
            let position = parse_named_vec2(&position, "engine.grid.cell_at position")?;
            grid_cell_to_lua(lua, cell_at(position, grid_size))
        })?;
        grid_tbl.set(lua_grid::CELL_AT, cell_at_fn)?;

        let cell_for_entity_fn = lua.create_function(|lua, entity: Option<AnyUserData>| {
            let Some(entity) = entity else {
                return Ok(Value::Nil);
            };
            let entity = entity.borrow::<EntityHandle>()?.clone();
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let Some(transform) = ecs.get::<Transform>(entity.entity) else {
                return Ok(Value::Nil);
            };
            let grid_size = game_instance.game.current_world().grid_size;
            Ok(Value::Table(grid_cell_to_lua(
                lua,
                cell_for_transform(transform, grid_size),
            )?))
        })?;
        grid_tbl.set(lua_grid::CELL_FOR_ENTITY, cell_for_entity_fn)?;

        let is_blocked_fn = lua.create_function(|lua, (actor, cell): (AnyUserData, Table)| {
            let actor = actor.borrow::<EntityHandle>()?.clone();
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let world = game_instance.game.current_world();
            let Some((room, layer)) = room_for_actor(ecs, world, actor.entity) else {
                return Ok(false);
            };
            let cell = grid_cell_from_lua(cell)?;
            Ok(!can_step(ecs, room, world.grid_size, layer, cell))
        })?;
        grid_tbl.set(lua_grid::IS_BLOCKED, is_blocked_fn)?;

        let neighbors_fn = lua.create_function(|lua, cell: Table| {
            let cell = grid_cell_from_lua(cell)?;
            let table = lua.create_table()?;
            for (index, neighbor) in cardinal_neighbors(cell).into_iter().enumerate() {
                table.set(index + 1, grid_cell_to_lua(lua, neighbor)?)?;
            }
            Ok(table)
        })?;
        grid_tbl.set(lua_grid::NEIGHBORS, neighbors_fn)?;

        let can_step_fn = lua.create_function(
            |lua, (actor, _from_cell, to_cell): (AnyUserData, Table, Table)| {
                let actor = actor.borrow::<EntityHandle>()?.clone();
                let ctx = LuaGameCtx::borrow_ctx(lua)?;
                let game_instance = ctx.game_instance.borrow();
                let ecs = &game_instance.game.ecs;
                let world = game_instance.game.current_world();
                let Some((room, layer)) = room_for_actor(ecs, world, actor.entity) else {
                    return Ok(false);
                };
                Ok(can_step(
                    ecs,
                    room,
                    world.grid_size,
                    layer,
                    grid_cell_from_lua(to_cell)?,
                ))
            },
        )?;
        grid_tbl.set(lua_grid::CAN_STEP, can_step_fn)?;

        let next_step_fn = lua.create_function(|lua, (actor, target_cell, options): (AnyUserData, Table, Option<Table>)| {
            let actor = actor.borrow::<EntityHandle>()?.clone();
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let world = game_instance.game.current_world();
            let Some((room, layer)) = room_for_actor(ecs, world, actor.entity) else {
                return Ok(Value::Nil);
            };
            let Some(transform) = ecs.get::<Transform>(actor.entity) else {
                return Ok(Value::Nil);
            };
            let grid_size = world.grid_size;
            let max_steps = options
                .as_ref()
                .and_then(|table| table.get::<Option<usize>>(lua_grid::MAX_STEPS).ok().flatten())
                .unwrap_or(DEFAULT_MAX_STEPS);
            let result = find_path(
                ecs,
                room,
                grid_size,
                layer,
                cell_for_transform(transform, grid_size),
                grid_cell_from_lua(target_cell)?,
                max_steps,
            );
            if result.status != GridQueryStatus::Ok || result.path.len() < 2 {
                return Ok(Value::Nil);
            }
            Ok(Value::Table(grid_cell_to_lua(lua, result.path[1])?))
        })?;
        grid_tbl.set(lua_grid::NEXT_STEP_TOWARDS, next_step_fn)?;

        let find_path_fn = lua.create_function(|lua, options: Table| {
            let actor: AnyUserData = options.get(lua_grid::ACTOR)?;
            let actor = actor.borrow::<EntityHandle>()?.clone();
            let ctx = LuaGameCtx::borrow_ctx(lua)?;
            let game_instance = ctx.game_instance.borrow();
            let ecs = &game_instance.game.ecs;
            let world = game_instance.game.current_world();
            let Some((room, layer)) = room_for_actor(ecs, world, actor.entity) else {
                return path_result_to_lua(lua, GridPathResult {
                    status: GridQueryStatus::InvalidEndpoint,
                    path: Vec::new(),
                });
            };
            let from_cell: Table = options.get(lua_grid::FROM)?;
            let to_cell: Table = options.get(lua_grid::TO)?;
            let max_steps = options
                .get::<Option<usize>>(lua_grid::MAX_STEPS)?
                .unwrap_or(DEFAULT_MAX_STEPS);
            let result = find_path(
                ecs,
                room,
                world.grid_size,
                layer,
                grid_cell_from_lua(from_cell)?,
                grid_cell_from_lua(to_cell)?,
                max_steps,
            );
            path_result_to_lua(lua, result)
        })?;
        grid_tbl.set(lua_grid::FIND_PATH, find_path_fn)?;

        engine_tbl.set(lua_engine::GRID, grid_tbl)?;
        Ok(())
    }
}

impl LuaApi for GridModule {
    fn emit_api(&self, out: &mut LuaApiWriter) {
        out.line("engine.grid = {}");
        out.line("");
        out.line("---@param position vec2");
        out.line("---@return table");
        out.line(&format!(
            "function engine.{}.{}(position) end",
            lua_engine::GRID,
            lua_grid::CELL_AT
        ));
        out.line("");
        out.line("---@param entity Entity?");
        out.line("---@return table|nil");
        out.line(&format!(
            "function engine.{}.{}(entity) end",
            lua_engine::GRID,
            lua_grid::CELL_FOR_ENTITY
        ));
        out.line("");
        out.line("---@param actor Entity");
        out.line("---@param cell table");
        out.line("---@return boolean");
        out.line(&format!(
            "function engine.{}.{}(actor, cell) end",
            lua_engine::GRID,
            lua_grid::IS_BLOCKED
        ));
        out.line("");
        out.line("---@param cell table");
        out.line("---@return table");
        out.line(&format!(
            "function engine.{}.{}(cell) end",
            lua_engine::GRID,
            lua_grid::NEIGHBORS
        ));
        out.line("");
        out.line("---@param actor Entity");
        out.line("---@param from_cell table");
        out.line("---@param to_cell table");
        out.line("---@return boolean");
        out.line(&format!(
            "function engine.{}.{}(actor, from_cell, to_cell) end",
            lua_engine::GRID,
            lua_grid::CAN_STEP
        ));
        out.line("");
        out.line("---@param actor Entity");
        out.line("---@param target_cell table");
        out.line("---@param options? table");
        out.line("---@return table|nil");
        out.line(&format!(
            "function engine.{}.{}(actor, target_cell, options) end",
            lua_engine::GRID,
            lua_grid::NEXT_STEP_TOWARDS
        ));
        out.line("");
        out.line("---@param options table");
        out.line("---@return table");
        out.line(&format!(
            "function engine.{}.{}(options) end",
            lua_engine::GRID,
            lua_grid::FIND_PATH
        ));
    }
}

fn cell_for_transform(transform: &Transform, grid_size: f32) -> GridCell {
    let inset = grid_size * 0.001;
    let mut position = transform.position;
    if matches!(
        transform.pivot,
        Pivot::BottomLeft | Pivot::BottomCenter | Pivot::BottomRight
    ) {
        position.y -= inset;
    }
    if matches!(
        transform.pivot,
        Pivot::TopRight | Pivot::CenterRight | Pivot::BottomRight
    ) {
        position.x -= inset;
    }
    cell_at(position, grid_size)
}

fn room_for_actor<'a>(
    ecs: &'a Ecs,
    world: &'a World,
    actor: Entity,
) -> Option<(&'a Room, RoomLayer)> {
    let current_room = ecs.get::<CurrentRoom>(actor)?;
    Some((world.get_room(current_room.room_id)?, current_room.layer))
}

fn grid_cell_from_lua(table: Table) -> LuaResult<GridCell> {
    Ok(GridCell {
        x: table.get(lua_grid::X)?,
        y: table.get(lua_grid::Y)?,
    })
}

fn grid_cell_to_lua(lua: &Lua, cell: GridCell) -> LuaResult<Table> {
    let table = lua.create_table()?;
    table.set(lua_grid::X, cell.x)?;
    table.set(lua_grid::Y, cell.y)?;
    Ok(table)
}

fn path_result_to_lua(lua: &Lua, result: GridPathResult) -> LuaResult<Table> {
    let table = lua.create_table()?;
    table.set(lua_grid::STATUS, status_to_lua(result.status))?;
    table.set(lua_grid::REASON, status_to_lua(result.status))?;
    let path = lua.create_table()?;
    for (index, cell) in result.path.into_iter().enumerate() {
        path.set(index + 1, grid_cell_to_lua(lua, cell)?)?;
    }
    table.set(lua_grid::PATH, path)?;
    Ok(table)
}

fn status_to_lua(status: GridQueryStatus) -> &'static str {
    match status {
        GridQueryStatus::Ok => lua_grid::OK,
        GridQueryStatus::Blocked => lua_grid::BLOCKED,
        GridQueryStatus::NoPath => lua_grid::NO_PATH,
        GridQueryStatus::BudgetExhausted => lua_grid::BUDGET_EXHAUSTED,
        GridQueryStatus::InvalidEndpoint => lua_grid::INVALID_ENDPOINT,
    }
}

fn cardinal_neighbors(cell: GridCell) -> [GridCell; 4] {
    [
        GridCell {
            x: cell.x + 1,
            y: cell.y,
        },
        GridCell {
            x: cell.x - 1,
            y: cell.y,
        },
        GridCell {
            x: cell.x,
            y: cell.y + 1,
        },
        GridCell {
            x: cell.x,
            y: cell.y - 1,
        },
    ]
}
