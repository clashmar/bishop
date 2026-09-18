use bishop::Vec2;
use crate::engine::GameInstance;
use crate::physics::events::{emit_physics_events, KinematicContactEvent, PhysicsEvents, SensorEvent};
use crate::save_system::SaveProviderRegistry;
use crate::scripting::lua_ctx::register_save_lua_context;
use crate::scripting::modules::entity_module::{lua_entity_handle, EntityHandle};
use crate::scripting::script_system::ScriptSystem;
use engine_core::camera::CameraManager;
use engine_core::constants::paths;
use engine_core::ecs::{
    Active,
    Entity,
    RoomCamera,
    Script,
    ScriptData,
    ScriptId,
    SubPixel,
    Transform,
    WorldEntry,
};
use engine_core::engine_global::set_game_name;
use engine_core::game::Game;
use engine_core::rendering::RoomRenderState;
use engine_core::scripting::lua_constants::{
    lua_dirs,
    lua_events,
    lua_files,
    lua_globals,
    lua_kinematic,
    lua_sensor,
};
use engine_core::storage::test_utils::{game_fs_test_lock, TestGameFolder};
use engine_core::storage::{game_folder, load_game_shell_from_folder};
use engine_core::worlds::{Room, RoomId, RoomLayer, World};
use mlua::{AnyUserData, Lua};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

fn register_demo_save_ctx(lua: &Lua) {
    register_save_lua_context(
        lua,
        std::rc::Rc::new(std::cell::RefCell::new(SaveProviderRegistry::new())),
        std::rc::Rc::new(std::cell::Cell::new(false)),
    )
    .unwrap();
}

fn test_game_instance() -> GameInstance {
    test_game_instance_with(Game::default())
}

fn test_game_instance_with(game: Game) -> GameInstance {
    GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    }
}

fn spawn_scripted_entity(game: &mut Game, lua: &Lua, script_src: &str) -> Entity {
    let entity = game
        .ecs
        .create_entity()
        .with(Transform::default())
        .finish();
    let script_id = ScriptId(900 + *entity);
    let instance: mlua::Table = lua.load(script_src).eval().unwrap();
    let handle = lua_entity_handle(lua, entity).unwrap();
    instance.set(lua_globals::ENTITY_HANDLE, handle).unwrap();
    game.ecs.replace_component(
        entity,
        Script {
            script_id,
            data: ScriptData::default(),
        },
    );
    game.script_manager.instances.insert((entity, script_id), instance);
    entity
}

fn script_field(game: &Game, entity: Entity, field: &str) -> String {
    let script = game.ecs.get::<Script>(entity).unwrap();
    let instance = game
        .script_manager
        .instances
        .get(&(entity, script.script_id))
        .unwrap();
    instance.get::<String>(field).unwrap()
}

fn local_callback_script(callback_name: &str, body: &str) -> String {
    format!("return {{ {callback_name} = function(self, event) {body} end }}")
}

#[test]
fn prepare_loaded_game_sets_current_world_room_to_start_room() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("prepare_loaded_game_start_room");
    set_game_name(test_game.name());

    let mut world = World::default();
    world.current_room_id = Some(RoomId(2));
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.add_room(Room {
        id: RoomId(2),
        position: Vec2::new(32.0, 0.0),
        ..Default::default()
    });

    let mut game = Game::with_name(test_game.name());
    game.add_world(world);

    let prepared = GameInstance::prepare_loaded_game(&Lua::new(), game);

    assert_eq!(prepared.room_id, RoomId(1));
    assert_eq!(prepared.game.current_world().current_room_id, Some(RoomId(1)));
}

#[test]
fn startup_room_and_layer_follow_the_start_entry() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("prepare_loaded_game_start_layer");
    set_game_name(test_game.name());

    let mut world = World::default();
    world.current_room_id = Some(RoomId(1));
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.add_room(Room {
        id: RoomId(2),
        position: Vec2::new(32.0, 0.0),
        ..Default::default()
    });

    let mut game = Game::with_name(test_game.name());
    game.add_world(world);
    game.ecs
        .create_entity()
        .with(WorldEntry {
            name: WorldEntry::START_NAME.to_string(),
            is_start: true,
        })
        .with_current_room_layer(RoomId(2), RoomLayer::Back)
        .finish();

    let prepared = GameInstance::prepare_loaded_game(&Lua::new(), game);

    assert_eq!(prepared.room_id, RoomId(2));
    assert_eq!(prepared.room_layer, RoomLayer::Back);
    assert_eq!(prepared.game.current_world().current_room_id, Some(RoomId(2)));
}

#[test]
fn prepare_loaded_room_sets_current_world_room_to_selected_room() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("prepare_loaded_room_selected_room");
    set_game_name(test_game.name());

    let room = Room {
        id: RoomId(2),
        position: Vec2::new(32.0, 0.0),
        ..Default::default()
    };
    let mut world = World::default();
    world.current_room_id = Some(RoomId(1));
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    world.add_room(room.clone());

    let mut game = Game::with_name(test_game.name());
    game.add_world(world);

    let prepared = GameInstance::prepare_loaded_room(&Lua::new(), room, game);

    assert_eq!(prepared.room_id, RoomId(2));
    assert_eq!(prepared.game.current_world().current_room_id, Some(RoomId(2)));
}

#[test]
fn full_runtime_init_executes_globals_prelude_once_before_main() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let test_game = TestGameFolder::new("game_instance_globals_once");
    set_game_name(test_game.name());

    let scripts_dir = game_folder(test_game.name())
        .join(paths::RESOURCES_FOLDER)
        .join(paths::SCRIPTS_FOLDER);
    let engine_dir = scripts_dir.join(lua_dirs::ENGINE);
    fs::create_dir_all(&engine_dir).unwrap();
    fs::write(
        engine_dir.join(lua_files::GLOBALS),
        "bootstrap_order = (bootstrap_order or \"\") .. \"g\"\nInput = { Space = \"space\" }\n",
    )
    .unwrap();
    fs::write(
        scripts_dir.join(lua_files::MAIN),
        "bootstrap_order = bootstrap_order .. \"m\"\nsaw_input = Input.Space\n",
    )
    .unwrap();

    let mut world = World::default();
    world.add_room(Room {
        id: RoomId(1),
        ..Default::default()
    });
    let mut game = Game::default();
    game.name = test_game.name().to_string();
    game.add_world(world);

    let lua = Lua::new();
    let prepared = GameInstance::prepare_loaded_game(&lua, game);
    ScriptSystem::init(&lua, &prepared.game.script_manager.event_bus);

    assert_eq!(lua.globals().get::<String>("bootstrap_order").unwrap(), "gm");
    assert_eq!(lua.globals().get::<String>("saw_input").unwrap(), "space");
}

#[test]
fn emit_physics_events_forwards_trigger_to_global_bus() {
    let lua = Lua::new();
    let capture = Rc::new(RefCell::new(Vec::<String>::new()));
    let instance = test_game_instance();
    let event_bus = instance.game.script_manager.event_bus.clone();
    let contact_capture = capture.clone();

    event_bus.on(
        lua_events::KINEMATIC_CONTACT.to_string(),
        lua.create_function(move |_, event: mlua::Table| {
            let kinematic: AnyUserData = event.get(lua_kinematic::EVENT_KINEMATIC)?;
            let kinematic = kinematic.borrow::<EntityHandle>()?;
            let kind: String = event.get(lua_kinematic::EVENT_KIND)?;
            contact_capture
                .borrow_mut()
                .push(format!("contact:{}:{}", *kinematic.entity, kind));
            Ok(())
        })
        .unwrap(),
    );

    let mut events = PhysicsEvents::default();
    events.push_kinematic_contact(KinematicContactEvent::Contact {
        kinematic: Entity(11),
        dynamic: Entity(22),
    });

    emit_physics_events(&lua, &instance.game, &mut events);

    assert_eq!(
        capture.borrow().as_slice(),
        [format!("contact:11:{}", lua_kinematic::KIND_TRIGGER)],
    );
}

#[test]
fn emit_physics_events_drains_each_event_once() {
    let lua = Lua::new();
    let capture = Rc::new(RefCell::new(Vec::<String>::new()));
    let instance = test_game_instance();
    let event_bus = instance.game.script_manager.event_bus.clone();
    let contact_capture = capture.clone();

    event_bus.on(
        lua_events::KINEMATIC_CONTACT.to_string(),
        lua.create_function(move |_, event: mlua::Table| {
            let kind: String = event.get(lua_kinematic::EVENT_KIND)?;
            contact_capture.borrow_mut().push(kind);
            Ok(())
        })
        .unwrap(),
    );

    let mut events = PhysicsEvents::default();
    events.push_kinematic_contact(KinematicContactEvent::Contact {
        kinematic: Entity(1),
        dynamic: Entity(2),
    });

    emit_physics_events(&lua, &instance.game, &mut events);
    emit_physics_events(&lua, &instance.game, &mut events);

    assert_eq!(capture.borrow().as_slice(), [lua_kinematic::KIND_TRIGGER]);
    assert!(events.drain().is_empty());
}

#[test]
fn emit_physics_events_calls_local_callbacks_on_both_entities() {
    let lua = Lua::new();
    let mut game = Game::default();
    let kinematic = spawn_scripted_entity(
        &mut game,
        &lua,
        &local_callback_script(
            lua_kinematic::CALLBACK_KINEMATIC_CONTACT,
            "self.hit = event.role",
        ),
    );
    let other = spawn_scripted_entity(
        &mut game,
        &lua,
        &local_callback_script(
            lua_kinematic::CALLBACK_KINEMATIC_CONTACT,
            "self.hit = event.role",
        ),
    );
    let instance = test_game_instance_with(game);
    let mut events = PhysicsEvents::default();
    events.push_kinematic_contact(KinematicContactEvent::Contact { kinematic, dynamic: other });

    emit_physics_events(&lua, &instance.game, &mut events);

    assert_eq!(
        script_field(&instance.game, kinematic, "hit"),
        lua_kinematic::ROLE_KINEMATIC
    );
    assert_eq!(
        script_field(&instance.game, other, "hit"),
        lua_kinematic::ROLE_OTHER
    );
}

#[test]
fn callback_error_does_not_block_other_callback_or_global_listener() {
    let lua = Lua::new();
    let mut game = Game::default();
    let kinematic = spawn_scripted_entity(
        &mut game,
        &lua,
        &local_callback_script(
            lua_kinematic::CALLBACK_KINEMATIC_CONTACT,
            "error('boom')",
        ),
    );
    let other = spawn_scripted_entity(
        &mut game,
        &lua,
        &local_callback_script(
            lua_kinematic::CALLBACK_KINEMATIC_CONTACT,
            "self.ok = event.kind",
        ),
    );
    let capture = Rc::new(RefCell::new(Vec::<String>::new()));
    let bus = game.script_manager.event_bus.clone();
    let capture_clone = capture.clone();
    bus.on(
        lua_events::KINEMATIC_CONTACT.to_string(),
        lua.create_function(move |_, event: mlua::Table| {
            capture_clone
                .borrow_mut()
                .push(event.get::<String>(lua_kinematic::EVENT_KIND)?);
            Ok(())
        })
        .unwrap(),
    );
    let instance = test_game_instance_with(game);
    let mut events = PhysicsEvents::default();
    events.push_kinematic_contact(KinematicContactEvent::Contact { kinematic, dynamic: other });

    emit_physics_events(&lua, &instance.game, &mut events);

    assert_eq!(capture.borrow().as_slice(), [lua_kinematic::KIND_TRIGGER]);
    assert_eq!(
        script_field(&instance.game, other, "ok"),
        lua_kinematic::KIND_TRIGGER
    );
}

#[test]
fn emit_physics_events_when_sensor_events_present_forwards_to_global_bus() {
    let lua = Lua::new();
    let capture = Rc::new(RefCell::new(Vec::<String>::new()));
    let instance = test_game_instance();
    let event_bus = instance.game.script_manager.event_bus.clone();

    for event_name in [
        lua_events::SENSOR_ENTER,
        lua_events::SENSOR_STAY,
        lua_events::SENSOR_EXIT,
    ] {
        let capture = capture.clone();
        event_bus.on(
            event_name.to_string(),
            lua.create_function(move |_, event: mlua::Table| {
                let sensor: AnyUserData = event.get(lua_sensor::EVENT_SENSOR)?;
                let sensor = sensor.borrow::<EntityHandle>()?;
                let body: AnyUserData = event.get(lua_sensor::EVENT_BODY)?;
                let body = body.borrow::<EntityHandle>()?;
                let kind: String = event.get(lua_sensor::EVENT_KIND)?;
                capture.borrow_mut().push(format!("{}:{}:{}", *sensor.entity, *body.entity, kind));
                Ok(())
            })
            .unwrap(),
        );
    }

    let mut events = PhysicsEvents::default();
    events.push_sensor(SensorEvent::Enter { body: Entity(22), sensor: Entity(11) });
    events.push_sensor(SensorEvent::Stay { body: Entity(22), sensor: Entity(11) });
    events.push_sensor(SensorEvent::Exit { body: Entity(22), sensor: Entity(11) });

    emit_physics_events(&lua, &instance.game, &mut events);

    assert_eq!(
        capture.borrow().as_slice(),
        [
            format!("11:22:{}", lua_sensor::KIND_ENTER),
            format!("11:22:{}", lua_sensor::KIND_STAY),
            format!("11:22:{}", lua_sensor::KIND_EXIT),
        ],
    );
}

#[test]
fn emit_physics_events_when_sensor_event_present_calls_local_callbacks_on_body_and_sensor() {
    let lua = Lua::new();
    let mut game = Game::default();
    let sensor = spawn_scripted_entity(
        &mut game,
        &lua,
        &local_callback_script(
            lua_sensor::CALLBACK_SENSOR_ENTER,
            "self.hit = event.role .. ':' .. event.kind .. ':' .. (event.other ~= nil and 'other' or 'missing')",
        ),
    );
    let body = spawn_scripted_entity(
        &mut game,
        &lua,
        &local_callback_script(
            lua_sensor::CALLBACK_SENSOR_ENTER,
            "self.hit = event.role .. ':' .. event.kind .. ':' .. (event.other ~= nil and 'other' or 'missing')",
        ),
    );
    let instance = test_game_instance_with(game);
    let mut events = PhysicsEvents::default();
    events.push_sensor(SensorEvent::Enter { body, sensor });

    emit_physics_events(&lua, &instance.game, &mut events);

    assert_eq!(
        script_field(&instance.game, sensor, "hit"),
        format!("{}:{}:other", lua_sensor::ROLE_SENSOR, lua_sensor::KIND_ENTER)
    );
    assert_eq!(
        script_field(&instance.game, body, "hit"),
        format!("{}:{}:other", lua_sensor::ROLE_BODY, lua_sensor::KIND_ENTER)
    );
}


#[test]
fn current_render_state_falls_back_to_room_camera_layer_and_position() {
    let room_id = RoomId(1);
    let room = Room {
        id: room_id,
        ..Default::default()
    };

    let mut world = World::default();
    world.current_room_id = Some(room_id);
    world.add_room(room);

    let mut game = Game::default();
    game.add_world(world);

    game.ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(48.0, 64.0),
            ..Default::default()
        })
        .with(RoomCamera::default())
        .with_current_room_layer(room_id, RoomLayer::Back)
        .finish();

    let game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };

    assert_eq!(
        game_instance.current_render_state(),
        RoomRenderState {
            current_layer: RoomLayer::Back,
            viewpoint_position: Some(Vec2::new(48.0, 64.0)),
            show_all_back_bounds: false,
        }
    );
}

#[test]
fn store_previous_positions_uses_visual_position_with_subpixel_remainder() {
    let room_id = RoomId(1);
    let room = Room {
        id: room_id,
        ..Default::default()
    };

    let mut world = World::default();
    world.current_room_id = Some(room_id);
    world.add_room(room);

    let mut game = Game::default();
    game.add_world(world);

    let entity = game.ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(10.0, 12.0),
            ..Default::default()
        })
        .with(Active::default())
        .with(SubPixel { x: 0.25, y: -0.5 })
        .with_current_room(room_id)
        .finish();

    let mut game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };

    game_instance.store_previous_positions(&mut CameraManager::default());

    assert_eq!(
        game_instance.prev_positions.get(&entity).copied(),
        Some(Vec2::new(10.25, 11.5))
    );
}

#[test]
fn prepare_loaded_game_can_activate_demo_runtime_scripts() {
    let _lock = game_fs_test_lock().lock().unwrap();
    let resources_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../games/Demo/Resources");
    let lua = Lua::new();
    let game = load_game_shell_from_folder(&resources_dir).unwrap();
    let mut prepared = GameInstance::prepare_loaded_game(&lua, game);

    register_demo_save_ctx(&lua);
    ScriptSystem::init(&lua, &prepared.game.script_manager.event_bus);
    ScriptSystem::activate_entity_scripts(
        &lua,
        &mut prepared.game.ecs,
        &mut prepared.game.script_manager,
    )
    .unwrap();

    let scripted_active_entities = prepared
        .game
        .ecs
        .get_store::<Script>()
        .data
        .iter()
        .filter_map(|(&entity, script)| {
            (script.script_id != ScriptId(0)
                && prepared
                    .game
                    .ecs
                    .get::<Active>(entity)
                    .is_some_and(Active::is_enabled))
            .then_some((entity, script.script_id))
        })
        .collect::<Vec<_>>();

    assert!(!scripted_active_entities.is_empty());
    for (entity, script_id) in scripted_active_entities {
        assert!(
            prepared
                .game
                .script_manager
                .instances
                .contains_key(&(entity, script_id)),
            "missing Lua instance for entity {entity:?} script {script_id:?}"
        );
    }
}

#[test]
fn store_previous_positions_keeps_pinned_inactive_entities() {
    let room_id = RoomId(1);
    let room = Room {
        id: room_id,
        ..Default::default()
    };

    let mut world = World::default();
    world.current_room_id = Some(room_id);
    world.add_room(room);

    let mut game = Game::default();
    game.add_world(world);

    let entity = game.ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(20.0, 24.0),
            ..Default::default()
        })
        .with(Active::new(false))
        .with_current_room(room_id)
        .finish();
    game.ecs.get_mut::<Active>(entity).unwrap().pin();

    let mut game_instance = GameInstance {
        game,
        prev_positions: HashMap::new(),
        traversal_residency_diagnostics: None,
    };

    game_instance.store_previous_positions(&mut CameraManager::default());

    assert_eq!(
        game_instance.prev_positions.get(&entity).copied(),
        Some(Vec2::new(20.0, 24.0))
    );
}
