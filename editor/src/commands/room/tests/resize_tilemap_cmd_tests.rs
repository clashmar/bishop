use super::*;
use engine_core::tiles::TileMap;
use engine_core::worlds::{BackRoomLayer, Exit, ExitDirection, InteriorZoneBounds, RoomLayer};

#[test]
fn resize_tilemap_cmd_rejects_resize_when_interior_zone_would_leave_room_bounds() {
    let _ctx = setup_editor("resize_tilemap_cmd_zone_bounds");
    enter_room_mode();

    let (room_id, variant_index, old_position, old_size, old_width, old_height) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let room = editor
            .game
            .current_world_mut()
            .expect("world should exist")
            .get_room_mut(room_id)
            .expect("room should exist");
        room.current_variant_mut().layers.back = Some(BackRoomLayer {
            interior_zones: vec![InteriorZone {
                id: InteriorZoneId(1),
                bounds: InteriorZoneBounds::new(32, 0, 16, 16),
            }],
            ..Default::default()
        });
        room.size = Vec2::new(4.0, 4.0);
        room.current_variant_mut().tilemap = TileMap::new(4, 4);
        (
            room_id,
            room.current_variant_index(),
            room.position,
            room.size,
            room.current_variant().tilemap.width,
            room.current_variant().tilemap.height,
        )
    });

    push_command(Box::new(ResizeTilemapCmd::new(
        room_id,
        variant_index,
        HandleSide::Right,
        -3,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let room = editor
            .game
            .current_world()
            .get_room(room_id)
            .expect("room should exist");
        assert_eq!(room.position, old_position);
        assert_eq!(room.size, old_size);
        assert_eq!(room.current_variant().tilemap.width, old_width);
        assert_eq!(room.current_variant().tilemap.height, old_height);
    });
}

#[test]
fn resize_tilemap_cmd_when_expanding_left_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_expand_left",
        HandleSide::Left,
        1,
        Vec2::new(-1.0, 0.0),
        Vec2::new(5.0, 4.0),
        [
            Vec2::new(2.0, -1.0),
            Vec2::new(3.0, 4.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(5.0, 2.0),
        ],
    );
}

#[test]
fn resize_tilemap_cmd_when_shrinking_left_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_shrink_left",
        HandleSide::Left,
        -1,
        Vec2::new(1.0, 0.0),
        Vec2::new(3.0, 4.0),
        [
            Vec2::new(0.0, -1.0),
            Vec2::new(1.0, 4.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(3.0, 2.0),
        ],
    );
}

#[test]
fn resize_tilemap_cmd_when_expanding_right_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_expand_right",
        HandleSide::Right,
        1,
        Vec2::ZERO,
        Vec2::new(5.0, 4.0),
        [
            Vec2::new(1.0, -1.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(5.0, 2.0),
        ],
    );
}

#[test]
fn resize_tilemap_cmd_when_shrinking_right_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_shrink_right",
        HandleSide::Right,
        -1,
        Vec2::ZERO,
        Vec2::new(3.0, 4.0),
        [
            Vec2::new(1.0, -1.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(3.0, 2.0),
        ],
    );
}

#[test]
fn resize_tilemap_cmd_when_expanding_top_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_expand_top",
        HandleSide::Top,
        1,
        Vec2::new(0.0, -1.0),
        Vec2::new(4.0, 5.0),
        [
            Vec2::new(1.0, -1.0),
            Vec2::new(2.0, 5.0),
            Vec2::new(-1.0, 2.0),
            Vec2::new(4.0, 3.0),
        ],
    );
}

#[test]
fn resize_tilemap_cmd_when_shrinking_top_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_shrink_top",
        HandleSide::Top,
        -1,
        Vec2::new(0.0, 1.0),
        Vec2::new(4.0, 3.0),
        [
            Vec2::new(1.0, -1.0),
            Vec2::new(2.0, 3.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(4.0, 1.0),
        ],
    );
}

#[test]
fn resize_tilemap_cmd_when_expanding_bottom_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_expand_bottom",
        HandleSide::Bottom,
        1,
        Vec2::ZERO,
        Vec2::new(4.0, 5.0),
        [
            Vec2::new(1.0, -1.0),
            Vec2::new(2.0, 5.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(4.0, 2.0),
        ],
    );
}

#[test]
fn resize_tilemap_cmd_when_shrinking_bottom_then_all_exit_positions_update_correctly() {
    assert_resize_case(
        "resize_tilemap_cmd_shrink_bottom",
        HandleSide::Bottom,
        -1,
        Vec2::ZERO,
        Vec2::new(4.0, 3.0),
        [
            Vec2::new(1.0, -1.0),
            Vec2::new(2.0, 3.0),
            Vec2::new(-1.0, 1.0),
            Vec2::new(4.0, 2.0),
        ],
    );
}

fn assert_resize_case(
    test_name: &str,
    side: HandleSide,
    delta: i32,
    expected_position_delta_tiles: Vec2,
    expected_size: Vec2,
    expected_exit_positions: [Vec2; 4],
) {
    let _ctx = setup_editor(test_name);
    enter_room_mode();

    let (room_id, variant_index, old_position) = with_editor(|editor| {
        let room_id = editor.cur_room_id.expect("room mode should select a room");
        let room = editor
            .game
            .current_world_mut()
            .expect("world should exist")
            .get_room_mut(room_id)
            .expect("room should exist");
        room.size = Vec2::new(4.0, 4.0);
        room.current_variant_mut().tilemap = TileMap::new(4, 4);
        room.exits = edge_exits();
        (room_id, room.current_variant_index(), room.position)
    });

    push_command(Box::new(ResizeTilemapCmd::new(
        room_id,
        variant_index,
        side,
        delta,
    )));
    apply_pending_commands();

    with_editor(|editor| {
        let world = editor.game.current_world();
        let room = world.get_room(room_id).expect("room should exist");
        let expected_position = old_position + expected_position_delta_tiles * world.grid_size;
        assert_eq!(room.position, expected_position);
        assert_eq!(room.size, expected_size);
        let actual_positions = [
            room.exits[0].position,
            room.exits[1].position,
            room.exits[2].position,
            room.exits[3].position,
        ];
        assert_eq!(actual_positions, expected_exit_positions);
    });
}

fn edge_exits() -> Vec<Exit> {
    vec![
        Exit {
            position: Vec2::new(1.0, -1.0),
            direction: ExitDirection::Up,
            layer: RoomLayer::Front,
            target_room_id: None,
        },
        Exit {
            position: Vec2::new(2.0, 4.0),
            direction: ExitDirection::Down,
            layer: RoomLayer::Front,
            target_room_id: None,
        },
        Exit {
            position: Vec2::new(-1.0, 1.0),
            direction: ExitDirection::Left,
            layer: RoomLayer::Front,
            target_room_id: None,
        },
        Exit {
            position: Vec2::new(4.0, 2.0),
            direction: ExitDirection::Right,
            layer: RoomLayer::Front,
            target_room_id: None,
        },
    ]
}
