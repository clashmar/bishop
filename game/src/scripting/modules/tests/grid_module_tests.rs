use crate::engine::GameInstance;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::EntityHandle;
use crate::scripting::modules::grid_module::GridModule;
use bishop::prelude::Vec2;
use engine_core::ecs::{CurrentRoom, Entity, Ecs, TilePlacement, Transform};
use engine_core::game::Game;
use engine_core::scripting::lua_constants::{lua_engine, lua_fields, lua_grid};
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter};
use engine_core::scripting::LuaModule;
use engine_core::tiles::TileDefId;
use engine_core::worlds::{Room, RoomId, RoomLayer, World, WorldId};
use mlua::{Lua, Table, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn setup_grid_lua(layer: RoomLayer) -> (Lua, Rc<RefCell<GameInstance>>, Entity) {
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();

    let mut game = Game::default();
    let mut world = World::new(WorldId(1), "Overworld".to_string(), 16.0);
    world.add_room(Room {
        id: RoomId(1),
        size: Vec2::new(12.0, 10.0),
        ..Default::default()
    });
    world.current_room_id = Some(RoomId(1));
    game.add_world(world);
    game.select_world(WorldId(1));

    let actor = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(0.0, 0.0),
            ..Default::default()
        })
        .with_current_room_layer(RoomId(1), layer)
        .finish();
    place_tile(&mut game.ecs, RoomId(1), 2, 4);
    place_tile(&mut game.ecs, RoomId(1), 1, 0);

    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    }));

    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .unwrap();

    lua.globals()
        .set("actor", EntityHandle { entity: actor })
        .unwrap();

    GridModule.register(&lua).unwrap();

    (lua, game_instance, actor)
}

fn place_tile(ecs: &mut Ecs, room_id: RoomId, x: usize, y: usize) {
    let entity = ecs.create_entity().finish();
    ecs.insert_component(entity, TilePlacement::new(TileDefId(3), x, y));
    ecs.insert_component(entity, CurrentRoom::new(room_id, RoomLayer::Front));
}

#[test]
fn grid_module_cell_at_returns_named_cell_table() {
    let (lua, _game_instance, _actor) = setup_grid_lua(RoomLayer::Front);

    let cell: Table = lua
        .load(format!(
            "return engine.{}.{}({{ [\"{}\"] = 17, [\"{}\"] = 31 }})",
            lua_engine::GRID,
            lua_grid::CELL_AT,
            lua_fields::X,
            lua_fields::Y,
        ))
        .eval()
        .unwrap();

    assert_eq!(cell.get::<i32>(lua_grid::X).unwrap(), 1);
    assert_eq!(cell.get::<i32>(lua_grid::Y).unwrap(), 1);
}

#[test]
fn grid_module_cell_for_entity_when_argument_is_nil_returns_nil() {
    let (lua, _game_instance, _entity) = setup_grid_lua(RoomLayer::Front);

    let result: Value = lua
        .load(format!(
            "return engine.{}.{}(nil)",
            lua_engine::GRID,
            lua_grid::CELL_FOR_ENTITY,
        ))
        .eval()
        .unwrap();

    assert!(matches!(result, Value::Nil));
}

#[test]
fn grid_module_cell_for_entity_uses_interior_side_of_bottom_pivot() {
    let (lua, game_instance, entity) = setup_grid_lua(RoomLayer::Front);
    if let Some(transform) = game_instance.borrow_mut().game.ecs.get_mut::<Transform>(entity) {
        transform.position = Vec2::new(0.0, 160.0);
    }

    let cell: Table = lua
        .load(format!(
            "return engine.{}.{}(actor)",
            lua_engine::GRID,
            lua_grid::CELL_FOR_ENTITY,
        ))
        .eval()
        .unwrap();
    assert_eq!(cell.get::<i32>(lua_grid::X).unwrap(), 0);
    assert_eq!(cell.get::<i32>(lua_grid::Y).unwrap(), 9);

    let next_step: Table = lua
        .load("return engine.grid.next_step_towards(actor, { x = 2, y = 9 })")
        .eval()
        .unwrap();
    assert_eq!(next_step.get::<i32>(lua_grid::X).unwrap(), 1);
    assert_eq!(next_step.get::<i32>(lua_grid::Y).unwrap(), 9);
}

#[test]
fn grid_module_is_blocked_when_cell_has_tile_returns_true() {
    let (lua, _game_instance, _actor) = setup_grid_lua(RoomLayer::Front);

    let blocked = lua
        .load(format!(
            "return engine.{}.{}(actor, {{ [\"{}\"] = 2, [\"{}\"] = 4 }})",
            lua_engine::GRID,
            lua_grid::IS_BLOCKED,
            lua_grid::X,
            lua_grid::Y,
        ))
        .eval::<bool>()
        .unwrap();

    assert!(blocked);
}

#[test]
fn grid_module_is_blocked_when_cell_is_outside_actor_room_returns_true() {
    let (lua, _game_instance, _actor) = setup_grid_lua(RoomLayer::Front);

    let blocked = lua
        .load(format!(
            "return engine.{}.{}(actor, {{ [\"{}\"] = 12, [\"{}\"] = 1 }})",
            lua_engine::GRID,
            lua_grid::IS_BLOCKED,
            lua_grid::X,
            lua_grid::Y,
        ))
        .eval::<bool>()
        .unwrap();

    assert!(blocked);
}

#[test]
fn grid_module_is_blocked_uses_actor_layer_instead_of_front_layer() {
    let (lua, _game_instance, _actor) = setup_grid_lua(RoomLayer::Back);

    let blocked = lua
        .load(format!(
            "return engine.{}.{}(actor, {{ [\"{}\"] = 2, [\"{}\"] = 4 }})",
            lua_engine::GRID,
            lua_grid::IS_BLOCKED,
            lua_grid::X,
            lua_grid::Y,
        ))
        .eval::<bool>()
        .unwrap();

    assert!(!blocked);
}

#[test]
fn grid_module_can_step_when_target_is_free_returns_true() {
    let (lua, _game_instance, _actor) = setup_grid_lua(RoomLayer::Front);

    let can_step = lua
        .load(format!(
            "return engine.{}.{}(actor, {{ [\"{}\"] = 0, [\"{}\"] = 0 }}, {{ [\"{}\"] = 0, [\"{}\"] = 1 }})",
            lua_engine::GRID,
            lua_grid::CAN_STEP,
            lua_grid::X,
            lua_grid::Y,
            lua_grid::X,
            lua_grid::Y,
        ))
        .eval::<bool>()
        .unwrap();

    assert!(can_step);
}

#[test]
fn grid_module_find_path_when_target_reachable_returns_ok_table() {
    let (lua, _game_instance, _actor) = setup_grid_lua(RoomLayer::Front);

    let result: Table = lua
        .load(format!(
            r#"
            return engine.{}.{}({{
              ["{}"] = actor,
              ["{}"] = {{ ["{}"] = 0, ["{}"] = 0 }},
              ["{}"] = {{ ["{}"] = 2, ["{}"] = 0 }},
              ["{}"] = 20,
            }})
            "#,
            lua_engine::GRID,
            lua_grid::FIND_PATH,
            lua_grid::ACTOR,
            lua_grid::FROM,
            lua_grid::X,
            lua_grid::Y,
            lua_grid::TO,
            lua_grid::X,
            lua_grid::Y,
            lua_grid::MAX_STEPS,
        ))
        .eval()
        .unwrap();

    assert_eq!(result.get::<String>(lua_grid::STATUS).unwrap(), lua_grid::OK);
    let path: Table = result.get(lua_grid::PATH).unwrap();
    let first: Table = path.get(1).unwrap();
    assert_eq!(first.get::<i32>(lua_grid::X).unwrap(), 0);
    assert_eq!(first.get::<i32>(lua_grid::Y).unwrap(), 0);
}

#[test]
fn grid_module_find_path_when_budget_exhausted_returns_budget_status() {
    let (lua, _game_instance, _actor) = setup_grid_lua(RoomLayer::Front);

    let result: Table = lua
        .load(format!(
            r#"
            return engine.{}.{}({{
              ["{}"] = actor,
              ["{}"] = {{ ["{}"] = 0, ["{}"] = 0 }},
              ["{}"] = {{ ["{}"] = 10, ["{}"] = 0 }},
              ["{}"] = 2,
            }})
            "#,
            lua_engine::GRID,
            lua_grid::FIND_PATH,
            lua_grid::ACTOR,
            lua_grid::FROM,
            lua_grid::X,
            lua_grid::Y,
            lua_grid::TO,
            lua_grid::X,
            lua_grid::Y,
            lua_grid::MAX_STEPS,
        ))
        .eval()
        .unwrap();

    assert_eq!(
        result.get::<String>(lua_grid::STATUS).unwrap(),
        lua_grid::BUDGET_EXHAUSTED
    );
}

#[test]
fn grid_module_emit_api_includes_return_annotations() {
    let mut out = LuaApiWriter::default();

    GridModule.emit_api(&mut out);

    assert!(out.buf.contains(&format!("function engine.{}.{}(position) end", lua_engine::GRID, lua_grid::CELL_AT)));
    assert!(out.buf.contains(&format!("function engine.{}.{}(entity) end", lua_engine::GRID, lua_grid::CELL_FOR_ENTITY)));
    assert!(out.buf.contains("---@param entity Entity?"));
    assert!(out.buf.contains(&format!("function engine.{}.{}(actor, cell) end", lua_engine::GRID, lua_grid::IS_BLOCKED)));
    assert!(out.buf.contains(&format!("function engine.{}.{}(actor, from_cell, to_cell) end", lua_engine::GRID, lua_grid::CAN_STEP)));
    assert!(out.buf.contains(&format!("function engine.{}.{}(options) end", lua_engine::GRID, lua_grid::FIND_PATH)));
    assert!(out.buf.contains("---@return table"));
    assert!(out.buf.contains("---@return boolean"));
}
