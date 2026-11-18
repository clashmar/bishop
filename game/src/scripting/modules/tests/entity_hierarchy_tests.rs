use crate::engine::game_instance::GameInstance;
use crate::scripting::lua_ctx::LuaGameCtx;
use crate::scripting::modules::entity_module::{EntityHandle, EntityModule};
use crate::scripting::script_system::ScriptSystem;
use engine_core::ecs::{Active, Ecs, Entity, Script, ScriptId, Transform, set_parent};
use engine_core::game::Game;
use engine_core::hydration::Hydratable;
use engine_core::scripting::lua_constants::lua_entity;
use engine_core::scripting::modules::lua_module::{LuaApi, LuaApiWriter};
use engine_core::worlds::{Room, RoomId, World, WorldId};
use mlua::Lua;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn setup_hierarchy_lua() -> (Lua, Rc<RefCell<GameInstance>>, Entity, Entity, Entity) {
    let lua = Lua::new();
    let mut game = Game::default();
    let mut world = World::new(WorldId(1), "Overworld".to_string(), 16.0);
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.current_room_id = Some(RoomId(1));
    game.add_world(world);

    let root = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room(RoomId(1))
        .finish();
    let child = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room(RoomId(1))
        .finish();
    let grandchild = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .with_current_room(RoomId(1))
        .finish();

    set_parent(&mut game.ecs, child, root);
    set_parent(&mut game.ecs, grandchild, child);

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
        .set("root", EntityHandle { entity: root })
        .unwrap();
    lua.globals()
        .set("child", EntityHandle { entity: child })
        .unwrap();
    lua.globals()
        .set("grandchild", EntityHandle { entity: grandchild })
        .unwrap();

    (lua, game_instance, root, child, grandchild)
}

#[test]
fn entity_hierarchy_parent_when_entity_has_parent_returns_parent_handle() {
    let (lua, _game_instance, root, _child, _grandchild) = setup_hierarchy_lua();

    let parent_id = lua
        .load(format!("return child:{}().id", lua_entity::PARENT))
        .eval::<usize>()
        .unwrap();

    assert_eq!(parent_id, *root);
}

#[test]
fn entity_hierarchy_parent_when_entity_has_no_parent_returns_nil() {
    let (lua, _game_instance, _root, _child, _grandchild) = setup_hierarchy_lua();

    let has_parent = lua
        .load(format!("return root:{}() ~= nil", lua_entity::PARENT))
        .eval::<bool>()
        .unwrap();

    assert!(!has_parent);
}

#[test]
fn entity_hierarchy_children_when_entity_has_children_returns_ordered_handles() {
    let (lua, _game_instance, _root, child, _grandchild) = setup_hierarchy_lua();

    let child_id = lua
        .load(format!("local children = root:{}(); return children[1].id", lua_entity::CHILDREN))
        .eval::<usize>()
        .unwrap();

    assert_eq!(child_id, *child);
}

#[test]
fn entity_hierarchy_root_when_entity_is_nested_returns_top_entity() {
    let (lua, _game_instance, root, _child, _grandchild) = setup_hierarchy_lua();

    let root_id = lua
        .load(format!("return grandchild:{}().id", lua_entity::ROOT))
        .eval::<usize>()
        .unwrap();

    assert_eq!(root_id, *root);
}

#[test]
fn entity_hierarchy_methods_when_entity_is_removed_return_live_entity_error() {
    let (lua, game_instance, _root, child, _grandchild) = setup_hierarchy_lua();
    {
        let mut game_instance = game_instance.borrow_mut();
        let mut game_ctx = game_instance.game.ctx_mut();
        Ecs::remove_entity(&mut game_ctx, child);
    }

    let error = lua
        .load(format!("return child:{}()", lua_entity::PARENT))
        .eval::<mlua::Value>()
        .unwrap_err();

    assert!(error.to_string().contains("no longer alive"));
}

#[test]
fn entity_hierarchy_emit_api_includes_return_annotations() {
    let mut out = LuaApiWriter::default();

    EntityModule.emit_api(&mut out);

    assert!(out.buf.contains(&format!("function Entity:{}() end", lua_entity::PARENT)));
    assert!(out.buf.contains(&format!("function Entity:{}() end", lua_entity::CHILDREN)));
    assert!(out.buf.contains(&format!("function Entity:{}() end", lua_entity::ROOT)));
    assert!(out.buf.contains("---@return Entity|nil"));
    assert!(out.buf.contains("---@return Entity[]"));
    assert!(out.buf.contains("---@return Entity"));
}

#[test]
fn entity_hierarchy_despawn_when_parent_has_scripted_child_unloads_child_script() {
    let (lua, game_instance, root, child, _grandchild) = setup_hierarchy_lua();
    let parent_script_id = ScriptId(101);
    let child_script_id = ScriptId(102);
    {
        let mut game_instance = game_instance.borrow_mut();
        let game = &mut game_instance.game;
        let parent_def = lua
            .load("return { init = function(self) end }")
            .eval()
            .unwrap();
        let child_def = lua
            .load("return { init = function(self) end }")
            .eval()
            .unwrap();
        game.script_manager.table_defs.insert(parent_script_id, parent_def);
        game.script_manager.table_defs.insert(child_script_id, child_def);
        game.script_manager.increment_ref(parent_script_id);
        game.script_manager.increment_ref(child_script_id);
        game.ecs.add_component_to_entity(root, Active::default());
        game.ecs.add_component_to_entity(child, Active::default());
        game.ecs.add_component_to_entity(
            root,
            Script {
                script_id: parent_script_id,
                ..Default::default()
            },
        );
        game.ecs.add_component_to_entity(
            child,
            Script {
                script_id: child_script_id,
                ..Default::default()
            },
        );
        ScriptSystem::activate_payload_scripts(
            &lua,
            &mut game.ecs,
            &mut game.script_manager,
            &[root, child],
        )
        .unwrap();

        assert!(game
            .script_manager
            .instances
            .contains_key(&(root, parent_script_id)));
        assert!(game
            .script_manager
            .instances
            .contains_key(&(child, child_script_id)));
    }

    lua.load(format!("root:{}()", lua_entity::DESPAWN))
        .exec()
        .unwrap();

    let game_instance = game_instance.borrow();
    assert!(!game_instance
        .game
        .script_manager
        .instances
        .contains_key(&(root, parent_script_id)));
    assert!(!game_instance
        .game
        .script_manager
        .instances
        .contains_key(&(child, child_script_id)));
}
