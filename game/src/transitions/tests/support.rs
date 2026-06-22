use crate::engine::game_instance::GameInstance;
use crate::physics::physics_system::update_physics;
use crate::transitions::room_transition_manager::RoomTransitionManager;
use bishop::prelude::{Vec2, vec2};
use engine_core::assets::SpriteManager;
use engine_core::ecs::{
    Active, Collider, Entity, PhysicsBody, Player, SubPixel, Transform, Velocity,
};
use engine_core::game::Game;
use engine_core::rendering::visual_position;
use engine_core::scripting::event_bus::EventBus;
use engine_core::scripting::event_tags::event_tag::EventTag;
use engine_core::scripting::lua_constants::lua_events;
use engine_core::tiles::TileMap;
use engine_core::worlds::{Exit, ExitDirection, Room, RoomId, RoomVariant, World};
use mlua::{Lua, Value, Variadic};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn make_room_pair(
    left_size: Vec2,
    right_size: Vec2,
    right_position: Vec2,
    tilemap_size: Option<(usize, usize)>,
) -> (Room, Room) {
    let variants = || {
        tilemap_size.map(|(width, height)| {
            vec![RoomVariant {
                tilemap: TileMap::new(width, height),
                ..Default::default()
            }]
        })
    };

    let left = Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        size: left_size,
        variants: variants().unwrap_or_default(),
        ..Default::default()
    };
    let right = Room {
        id: RoomId(2),
        position: right_position,
        size: right_size,
        variants: variants().unwrap_or_default(),
        ..Default::default()
    };
    (left, right)
}

pub(super) fn make_two_rooms() -> (Room, Room) {
    make_room_pair(
        Vec2::new(32.0, 32.0),
        Vec2::new(32.0, 32.0),
        Vec2::new(32.0, 0.0),
        None,
    )
}

fn make_two_physics_rooms() -> (Room, Room) {
    make_room_pair(
        Vec2::new(2.0, 2.0),
        Vec2::new(2.0, 2.0),
        Vec2::new(32.0, 0.0),
        Some((2, 2)),
    )
}

pub(super) fn setup_tagged_game(
    tags: Vec<String>,
) -> (Lua, Game, Entity, Arc<Mutex<Vec<String>>>) {
    let lua = Lua::new();
    let event_bus = EventBus::default();
    let received = Arc::new(Mutex::new(Vec::<String>::new()));
    let (room_a, mut room_b) = make_two_rooms();
    room_b.tags = tags.into_iter().map(EventTag::Custom).collect();

    let mut world = World::default();
    world.add_room(room_a);
    world.add_room(room_b);
    world.grid_size = 1.0;
    world.rebuild_room_grid();

    let mut game = Game::default();
    game.script_manager.event_bus = event_bus.clone();
    game.add_world(world);
    let player = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(40.0, 9.0),
            ..Default::default()
        })
        .with(Collider::default())
        .with(Player)
        .with_current_room(RoomId(1))
        .finish();

    let captured = received.clone();
    let handler = lua
        .create_function(move |_lua, args: Variadic<Value>| {
            let mut values = captured.lock().unwrap();
            values.clear();
            for arg in args {
                match arg {
                    Value::Integer(n) => values.push(n.to_string()),
                    Value::String(s) => values.push(s.to_str().unwrap().to_string()),
                    other => panic!("unexpected value: {other:?}"),
                }
            }
            Ok(())
        })
        .unwrap();
    event_bus.on(lua_events::ROOM_ENTERED.to_string(), handler);

    (lua, game, player, received)
}

pub(super) fn setup_physics_transition_game(
    start_room: RoomId,
    position_x: f32,
    velocity_x: f32,
) -> (Lua, GameInstance, Entity) {
    let lua = Lua::new();
    let (mut room_a, mut room_b) = make_two_physics_rooms();
    room_a.exits.push(Exit {
        position: vec2(2.0, 1.0),
        direction: ExitDirection::Right,
        target_room_id: Some(room_b.id),
    });
    room_b.exits.push(Exit {
        position: vec2(-1.0, 1.0),
        direction: ExitDirection::Left,
        target_room_id: Some(room_a.id),
    });

    let mut world = World::default();
    world.grid_size = 16.0;
    world.current_room_id = Some(start_room);
    world.add_room(room_a);
    world.add_room(room_b);
    world.rebuild_room_grid();

    let mut game = Game::default();
    game.add_world(world);
    let entity = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(position_x, 32.0),
            ..Default::default()
        })
        .with(Collider::default())
        .with(PhysicsBody)
        .with(Active::default())
        .with(SubPixel::default())
        .with(Velocity { x: velocity_x, y: 0.0 })
        .with_current_room(start_room)
        .finish();

    (
        lua,
        GameInstance {
            game,
            prev_positions: HashMap::new(), 
            traversal_residency_diagnostics: None,
        },
        entity,
    )
}

pub(super) fn setup_physics_down_transition_game() -> (Lua, GameInstance, Entity) {
    let lua = Lua::new();
    let mut room_a = Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        size: Vec2::new(32.0, 9.0),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(32, 9),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut room_b = Room {
        id: RoomId(2),
        position: Vec2::new(56.0, 72.0),
        size: Vec2::new(13.0, 5.0),
        variants: vec![RoomVariant {
            tilemap: TileMap::new(13, 5),
            ..Default::default()
        }],
        ..Default::default()
    };
    for x in [12.0, 13.0, 14.0] {
        room_a.exits.push(Exit {
            position: vec2(x, 9.0),
            direction: ExitDirection::Down,
            target_room_id: Some(room_b.id),
        });
    }
    for x in [5.0, 6.0, 7.0] {
        room_b.exits.push(Exit {
            position: vec2(x, -1.0),
            direction: ExitDirection::Up,
            target_room_id: Some(room_a.id),
        });
    }

    let mut world = World::default();
    world.grid_size = 8.0;
    world.current_room_id = Some(room_a.id);
    world.add_room(room_a);
    world.add_room(room_b);
    world.rebuild_room_grid();

    let mut game = Game::default();
    game.add_world(world);
    let entity = game
        .ecs
        .create_entity()
        .with(Transform {
            position: Vec2::new(108.0, 69.0),
            ..Default::default()
        })
        .with(Collider::default())
        .with(PhysicsBody)
        .with(Active::default())
        .with(SubPixel::default())
        .with(Velocity { x: 0.0, y: 200.0 })
        .with_current_room(RoomId(1))
        .finish();

    (
        lua,
        GameInstance {
            game,
            prev_positions: HashMap::new(), 
            traversal_residency_diagnostics: None,
        },
        entity,
    )
}

pub(super) fn step_physics_and_transitions(
    lua: &Lua,
    game_instance: &mut GameInstance,
    entity: Entity,
) -> Vec2 {
    let world = game_instance.game.current_world().clone();
    update_physics(
        &SpriteManager::default(),
        &mut game_instance.game.ecs,
        &world,
        1.0 / 60.0,
    );

    RoomTransitionManager::handle_transitions(lua, game_instance);

    let transform = game_instance.game.ecs.get::<Transform>(entity).unwrap();
    visual_position(
        transform.position,
        game_instance.game.ecs.get::<SubPixel>(entity),
    )
}

