use crate::ecs::{CurrentFrame, Ecs, Entity, Transform};
use crate::worlds::test_utils::make_room;
use crate::worlds::{Exit, RoomId, World, WorldId};
use bishop::prelude::*;

pub(crate) fn make_spillover_entity(
    ecs: &mut Ecs,
    position: Vec2,
    room_id: Option<RoomId>,
) -> Entity {
    let builder = ecs
        .create_entity()
        .with(Transform {
            position,
            ..Default::default()
        })
        .with(CurrentFrame {
            frame_size: vec2(16.0, 32.0),
            ..Default::default()
        });

    match room_id {
        Some(room_id) => builder.with_current_room(room_id).finish(),
        None => builder.finish(),
    }
}

pub(crate) fn make_vertical_spillover_fixture(
    entity_pos: Vec2,
    exit: Option<Exit>,
) -> (World, Ecs, Entity, RoomId) {
    let mut room_a = make_room(Some(1), 0.0, 0.0, 4.0, 4.0);
    if let Some(exit) = exit {
        room_a.exits.push(exit);
    }
    let room_b = make_room(Some(2), 0.0, 64.0, 4.0, 4.0);
    let world = World::from_rooms(WorldId(0), String::new(), vec![room_a, room_b], 16.0);
    let mut ecs = Ecs::default();
    let entity = make_spillover_entity(&mut ecs, entity_pos, Some(RoomId(2)));

    (world, ecs, entity, RoomId(1))
}
