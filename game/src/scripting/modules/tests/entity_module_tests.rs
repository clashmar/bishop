use crate::engine::game_instance::GameInstance;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::EntityHandle;
use engine_core::prelude::*;
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[test]
fn despawn_is_safe_to_call_on_an_already_removed_entity() {
    let lua = Lua::new();
    let mut game = Game::default();
    game.worlds.push(World::default());

    let entity = game.ecs.create_entity().with(Transform::default()).finish();
    let game_instance = Rc::new(RefCell::new(GameInstance {
        game,
        prev_positions: HashMap::new(),
    }));

    LuaGameCtx {
        game_instance: game_instance.clone(),
    }
    .set_lua_ctx(&lua)
    .unwrap();

    let entity_handle = lua.create_userdata(EntityHandle { entity }).unwrap();
    lua.globals().set("entity", entity_handle).unwrap();

    let result = lua.load("entity:despawn(); entity:despawn()").exec();

    assert!(result.is_ok(), "unexpected error: {result:?}");
    assert!(game_instance
        .borrow()
        .game
        .ecs
        .get::<Transform>(entity)
        .is_none());
}
