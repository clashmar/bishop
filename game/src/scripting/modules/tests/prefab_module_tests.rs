use crate::engine::game_instance::GameInstance;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::prefab_module::PrefabModule;
use engine_core::game::Game;
use engine_core::prefab::{PrefabAsset, PrefabId, PrefabNode};
use engine_core::scripting::lua_constants::lua_engine;
use engine_core::scripting::LuaModule;
use engine_core::worlds::{Room, RoomId, World, WorldId};
use mlua::{Lua, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn prefab_test_instance(prefab_name: &str) -> (Lua, Rc<RefCell<GameInstance>>) {
    let lua = Lua::new();
    lua.globals()
        .set(lua_engine::ENGINE, lua.create_table().unwrap())
        .unwrap();
    PrefabModule.register(&lua).unwrap();

    let mut game = Game::default();
    let mut world = World::new(WorldId(1), "Main".to_string(), 16.0);
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.current_room_id = Some(RoomId(1));
    game.add_world(world);
    game.current_world_id = Some(WorldId(1));

    let prefab_id = PrefabId(1);
    let prefab = PrefabAsset {
        id: prefab_id,
        name: prefab_name.to_string(),
        next_node_id: 2,
        root_node_id: 1,
        nodes: vec![PrefabNode {
            node_id: 1,
            parent_node_id: None,
            components: vec![],
        }],
    };
    game.prefab_manager.prefabs.insert(prefab_id, prefab);
    game.prefab_manager
        .prefab_ids_by_name
        .insert(prefab_name.to_string(), prefab_id);
    game.prefab_manager.next_prefab_id = 2;

    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
    }));
    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .unwrap();

    (lua, game_instance)
}

#[test]
fn spawning_prefab_touches_the_runtime_recency_tracker() {
    let (lua, game_instance) = prefab_test_instance("Bullet");

    let spawned = lua
        .load("return engine.prefab.spawn('Bullet', { x = 0.0, y = 0.0 })")
        .eval::<Value>()
        .unwrap();

    assert!(matches!(spawned, Value::UserData(_)));
    assert!(game_instance
        .borrow()
        .game
        .prefab_manager
        .runtime_recency()
        .contains(&PrefabId(1)));
}
