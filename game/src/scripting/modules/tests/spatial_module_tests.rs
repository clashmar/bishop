use crate::engine::GameInstance;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::EntityHandle;
use crate::scripting::modules::spatial_module::SpatialModule;
use bishop::prelude::Vec2;
use engine_core::ecs::{Collider, ColliderShape, Entity, Transform};
use engine_core::game::Game;
use engine_core::scripting::lua_constants::{lua_engine, lua_fields, lua_spatial};
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter};
use engine_core::scripting::LuaModule;
use engine_core::worlds::{Room, RoomId, World, WorldId};
use mlua::{Lua, Table};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn setup_spatial_lua() -> (Lua, Rc<RefCell<GameInstance>>, Entity, Entity, Entity) {
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();

    let mut game = Game::default();
    let mut world = World::new(WorldId(1), "Overworld".to_string(), 16.0);
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.current_room_id = Some(RoomId(1));
    game.add_world(world);
    game.select_world(WorldId(1));

    let observer = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(0.0, 0.0),
            ..Default::default()
        })
        .with_current_room(RoomId(1))
        .finish();
    let blocker = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(8.0, 0.0),
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 4.0,
                height: 4.0,
            },
            ..Default::default()
        })
        .with_current_room(RoomId(1))
        .finish();
    let target = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(16.0, 0.0),
            ..Default::default()
        })
        .with(Collider {
            shape: ColliderShape::Aabb {
                width: 4.0,
                height: 4.0,
            },
            ..Default::default()
        })
        .with_current_room(RoomId(1))
        .finish();

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
        .set("observer", EntityHandle { entity: observer })
        .unwrap();
    lua.globals()
        .set("blocker", EntityHandle { entity: blocker })
        .unwrap();
    lua.globals()
        .set("target", EntityHandle { entity: target })
        .unwrap();

    SpatialModule.register(&lua).unwrap();

    (lua, game_instance, observer, blocker, target)
}

#[test]
fn spatial_module_raycast_when_hit_returns_structured_hit_table() {
    let (lua, _game_instance, _observer, blocker, _target) = setup_spatial_lua();

    let hit: Table = lua
        .load(format!(
            r#"
            return engine.{}.{}({{
              ["{}"] = {{ ["{}"] = 0, ["{}"] = 0 }},
              ["{}"] = {{ ["{}"] = 20, ["{}"] = 0 }},
            }})
            "#,
            lua_engine::SPATIAL,
            lua_spatial::RAYCAST,
            lua_spatial::FROM,
            lua_fields::X,
            lua_fields::Y,
            lua_spatial::TO,
            lua_fields::X,
            lua_fields::Y,
        ))
        .eval()
        .unwrap();

    assert!(hit.get::<bool>(lua_spatial::HIT).unwrap());
    let entity: mlua::AnyUserData = hit.get(lua_spatial::ENTITY).unwrap();
    let entity = entity.borrow::<EntityHandle>().unwrap();
    let point: Table = hit.get(lua_spatial::POINT).unwrap();
    assert_eq!(entity.entity, blocker);
    assert_eq!(point.get::<f32>(lua_fields::X).unwrap(), 6.0);
    assert_eq!(point.get::<f32>(lua_fields::Y).unwrap(), 0.0);
    assert_eq!(hit.get::<f32>(lua_spatial::DISTANCE).unwrap(), 6.0);
}

#[test]
fn spatial_module_raycast_when_no_hit_returns_hit_false() {
    let (lua, _game_instance, _observer, _blocker, _target) = setup_spatial_lua();

    let hit: Table = lua
        .load(format!(
            r#"
            return engine.{}.{}({{
              ["{}"] = {{ ["{}"] = 0, ["{}"] = 8 }},
              ["{}"] = {{ ["{}"] = 20, ["{}"] = 8 }},
            }})
            "#,
            lua_engine::SPATIAL,
            lua_spatial::RAYCAST,
            lua_spatial::FROM,
            lua_fields::X,
            lua_fields::Y,
            lua_spatial::TO,
            lua_fields::X,
            lua_fields::Y,
        ))
        .eval()
        .unwrap();

    assert!(!hit.get::<bool>(lua_spatial::HIT).unwrap());
}

#[test]
fn spatial_module_raycast_when_ignore_entity_is_supplied_skips_entity() {
    let (lua, _game_instance, _observer, _blocker, target) = setup_spatial_lua();

    let hit: Table = lua
        .load(format!(
            r#"
            return engine.{}.{}({{
              ["{}"] = {{ ["{}"] = 0, ["{}"] = 0 }},
              ["{}"] = {{ ["{}"] = 20, ["{}"] = 0 }},
              ["{}"] = blocker,
            }})
            "#,
            lua_engine::SPATIAL,
            lua_spatial::RAYCAST,
            lua_spatial::FROM,
            lua_fields::X,
            lua_fields::Y,
            lua_spatial::TO,
            lua_fields::X,
            lua_fields::Y,
            lua_spatial::IGNORE,
        ))
        .eval()
        .unwrap();

    let entity: mlua::AnyUserData = hit.get(lua_spatial::ENTITY).unwrap();
    let entity = entity.borrow::<EntityHandle>().unwrap();
    assert_eq!(entity.entity, target);
}

#[test]
fn spatial_module_has_line_of_sight_when_blocked_returns_false() {
    let (lua, _game_instance, _observer, _blocker, _target) = setup_spatial_lua();

    let visible = lua
        .load(format!(
            "return engine.{}.{}(observer, target)",
            lua_engine::SPATIAL,
            lua_spatial::HAS_LINE_OF_SIGHT,
        ))
        .eval::<bool>()
        .unwrap();

    assert!(!visible);
}

#[test]
fn spatial_module_emit_api_includes_return_annotations() {
    let mut out = LuaApiWriter::default();

    SpatialModule.emit_api(&mut out);

    assert!(out.buf.contains(&format!(
        "function engine.{}.{}(options) end",
        lua_engine::SPATIAL,
        lua_spatial::RAYCAST
    )));
    assert!(out.buf.contains(&format!(
        "function engine.{}.{}(from_entity, to_entity, options) end",
        lua_engine::SPATIAL,
        lua_spatial::HAS_LINE_OF_SIGHT
    )));
    assert!(out.buf.contains("---@return table"));
    assert!(out.buf.contains("---@return boolean"));
}
