use super::super::*;

#[test]
fn relink_room_exits_populates_new_room_exit_targets() {
    let grid_size = 16.0;
    let mut world = World::new(WorldId(1), "Main".to_string(), grid_size);
    world.add_room(Room {
        id: RoomId(1),
        position: Vec2::ZERO,
        size: vec2(4.0, 4.0),
        exits: vec![Exit {
            position: vec2(4.0, 1.0),
            direction: ExitDirection::Right,
            layer: RoomLayer::Front,
            target_room_id: None,
        }],
        ..Default::default()
    });
    world.add_room(Room {
        id: RoomId(2),
        position: vec2(grid_size * 4.0, 0.0),
        size: vec2(4.0, 4.0),
        exits: vec![Exit {
            position: vec2(-1.0, 1.0),
            direction: ExitDirection::Left,
            layer: RoomLayer::Front,
            target_room_id: None,
        }],
        ..Default::default()
    });

    world.link_all_room_exits();

    assert_eq!(
        world.get_room(RoomId(1)).unwrap().exits[0].target_room_id,
        Some(RoomId(2))
    );
    assert_eq!(
        world.get_room(RoomId(2)).unwrap().exits[0].target_room_id,
        Some(RoomId(1))
    );
}
